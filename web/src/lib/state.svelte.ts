import { ws, onReconnect, onStatus } from './ws'
import type { WsStatus } from './ws'
import { decryptText, deriveKey, encryptText } from './crypto'
import {
  Identity,
  createChannel as apiCreateChannel,
  createWorkspace as apiCreateWorkspace,
  getMessages,
  joinWorkspace as apiJoinWorkspace,
  listSounds,
  listWorkspaces,
  postMessage,
  token,
} from './api'
import type {
  Channel,
  ChannelKind,
  Id,
  Identity as IdentityData,
  Message,
  ServerEvent,
  Sound,
  UserPublic,
  VoicePresence,
  Workspace,
} from './protocol'

export const net = $state<{ status: WsStatus }>({ status: 'offline' })

export const typing = $state<Record<Id, UserPublic[]>>({})
const typingTimers = new Map<string, ReturnType<typeof setTimeout>>()

function selfId(): Id | null {
  return app.identity?.public_id ?? null
}

function selfUser(): UserPublic | null {
  const id = app.identity
  if (!id) return null
  return { id: id.public_id, display_name: id.display_name, avatar_url: null }
}

function channelById(id: Id): Channel | undefined {
  for (const w of app.workspaces) {
    const c = w.channels.find((x) => x.id === id)
    if (c) return c
  }
  return undefined
}

const contentKeys = new Map<Id, Promise<CryptoKey>>()
const decoded = new Map<Id, string>()

function workspaceOfChannel(channelId: Id): Workspace | undefined {
  for (const w of app.workspaces) {
    if (w.channels.some((c) => c.id === channelId)) return w
  }
  return undefined
}

function loadKey(w: Workspace): Promise<CryptoKey> {
  const existing = contentKeys.get(w.id)
  if (existing) return existing
  const p = deriveKey(w.secret as string)
  contentKeys.set(w.id, p)
  p.catch(() => contentKeys.delete(w.id))
  return p
}

function keyForChannel(channelId: Id): Promise<CryptoKey> | null {
  const w = workspaceOfChannel(channelId)
  if (!w || !w.secret) return null
  return loadKey(w)
}

async function displayMessage(m: Message): Promise<Message> {
  const known = decoded.get(m.id)
  if (known !== undefined) return { ...m, content: known }
  const key = await keyForChannel(m.channel_id)
  if (!key) return m
  let content: string
  try {
    content = await decryptText(key, m.content)
  } catch {
    content = '🔒 unreadable message'
  }
  decoded.set(m.id, content)
  return { ...m, content }
}

async function receiveMessage(m: Message): Promise<void> {
  appendMessage(await displayMessage(m))
}

function refreshMembers(): void {
  const map = new Map<Id, UserPublic>()
  const me = selfUser()
  if (me) map.set(me.id, me)
  for (const m of app.messages) map.set(m.author.id, m.author)
  for (const p of app.voice) for (const vm of p.members) map.set(vm.user.id, vm.user)
  app.members = [...map.values()]
}

function appendMessage(m: Message): void {
  if (m.channel_id !== app.currentChannelId) return
  if (app.messages.some((x) => x.id === m.id)) return
  const next = app.messages.concat(m)
  next.sort((a, b) => (a.created_at < b.created_at ? -1 : a.created_at > b.created_at ? 1 : 0))
  app.messages = next
  refreshMembers()
}

function upsertWorkspace(wsp: Workspace): void {
  const i = app.workspaces.findIndex((w) => w.id === wsp.id)
  if (i >= 0) app.workspaces[i] = wsp
  else app.workspaces = [...app.workspaces, wsp]
}

function upsertPresence(p: VoicePresence): void {
  if (app.currentWorkspaceId != null && p.workspace_id !== app.currentWorkspaceId) return
  const i = app.voice.findIndex((v) => v.channel_id === p.channel_id)
  if (i >= 0) app.voice[i] = p
  else app.voice.push(p)
  refreshMembers()
}

function removeVoiceMember(channelId: Id, userId: Id): void {
  const i = app.voice.findIndex((v) => v.channel_id === channelId)
  if (i < 0) return
  const presence = app.voice[i]
  const members = presence.members.filter((m) => m.user.id !== userId)
  if (members.length === 0) app.voice.splice(i, 1)
  else app.voice[i] = { ...presence, members }
  refreshMembers()
}

function setTyping(channelId: Id, user: UserPublic): void {
  if (user.id === selfId()) return
  const list = typing[channelId] ?? []
  if (!list.some((u) => u.id === user.id)) typing[channelId] = [...list, user]
  const key = `${channelId}:${user.id}`
  const prev = typingTimers.get(key)
  if (prev) clearTimeout(prev)
  typingTimers.set(
    key,
    setTimeout(() => {
      typingTimers.delete(key)
      const l = typing[channelId]
      if (l) typing[channelId] = l.filter((u) => u.id !== user.id)
    }, 4000),
  )
}

function handleEvent(ev: ServerEvent): void {
  switch (ev.op) {
    case 'ready':
      app.identity = ev.identity
      app.workspaces = ev.workspaces.map((w) => ({
        ...w,
        secret: w.secret ?? app.workspaces.find((o) => o.id === w.id)?.secret ?? null,
      }))
      app.sounds = ev.sounds
      for (const w of ev.workspaces) if (w.secret) void loadKey(w)
      app.voice = ev.voice.filter(
        (v) => app.currentWorkspaceId == null || v.workspace_id === app.currentWorkspaceId,
      )
      if (app.currentChannelId != null) {
        ws.send({ op: 'subscribe', channel_id: app.currentChannelId })
      }
      refreshMembers()
      break
    case 'message':
      void receiveMessage(ev.message)
      break
    case 'typing':
      setTyping(ev.channel_id, ev.user)
      break
    case 'voice_presence':
    case 'voice_joined':
      upsertPresence(ev.presence)
      break
    case 'voice_left':
      removeVoiceMember(ev.channel_id, ev.user_id)
      break
    case 'voice_updated': {
      const i = app.voice.findIndex((v) => v.channel_id === ev.channel_id)
      if (i >= 0) {
        const presence = app.voice[i]
        const mi = presence.members.findIndex((m) => m.user.id === ev.member.user.id)
        const members = [...presence.members]
        if (mi >= 0) members[mi] = ev.member
        else members.push(ev.member)
        app.voice[i] = { ...presence, members }
        refreshMembers()
      }
      break
    }
    case 'sound_play':
    case 'signal':
    case 'error':
    case 'pong':
      break
  }
}

async function selectChannel(id: Id): Promise<void> {
  if (app.currentChannelId === id) return
  const prev = app.currentChannelId
  app.currentChannelId = id
  app.messages = []
  if (prev != null) ws.send({ op: 'unsubscribe', channel_id: prev })
  const channel = channelById(id)
  if (!channel || channel.kind !== 'text') {
    refreshMembers()
    return
  }
  ws.send({ op: 'subscribe', channel_id: id })
  try {
    const msgs = await getMessages(id)
    if (app.currentChannelId === id) {
      app.messages = await Promise.all(msgs.map(displayMessage))
      refreshMembers()
    }
  } catch {
    if (app.currentChannelId === id) app.messages = []
  }
}

function selectWorkspace(id: Id): void {
  if (app.currentWorkspaceId === id) {
    const wsp = app.workspaces.find((w) => w.id === id)
    if (!app.currentChannelId && wsp) {
      const first = wsp.channels.find((c) => c.kind === 'text') ?? wsp.channels[0]
      if (first) void selectChannel(first.id)
    }
    return
  }
  app.currentWorkspaceId = id
  app.voice = app.voice.filter((v) => v.workspace_id === id)
  const wsp = app.workspaces.find((w) => w.id === id)
  if (wsp?.secret) void loadKey(wsp)
  const first = wsp?.channels.find((c) => c.kind === 'text') ?? wsp?.channels[0]
  if (first) {
    void selectChannel(first.id)
  } else {
    if (app.currentChannelId != null) {
      ws.send({ op: 'unsubscribe', channel_id: app.currentChannelId })
    }
    app.currentChannelId = null
    app.messages = []
    refreshMembers()
  }
}

async function sendMessage(content: string, attachmentIds: Id[] = []): Promise<void> {
  const channelId = app.currentChannelId
  if (channelId == null) return
  const key = await keyForChannel(channelId)
  const payload = key && content.length > 0 ? await encryptText(key, content) : content
  const m = await postMessage(channelId, payload, attachmentIds)
  appendMessage(await displayMessage(m))
}

async function reloadSounds(): Promise<void> {
  try {
    app.sounds = await listSounds()
  } catch {
    void 0
  }
}

async function createWorkspace(name: string): Promise<Workspace> {
  const wsp = await apiCreateWorkspace(name)
  upsertWorkspace(wsp)
  selectWorkspace(wsp.id)
  return wsp
}

async function joinWorkspace(ticket: string): Promise<Workspace> {
  const wsp = await apiJoinWorkspace(ticket)
  upsertWorkspace(wsp)
  selectWorkspace(wsp.id)
  return wsp
}

async function createChannel(name: string, kind: ChannelKind): Promise<Channel> {
  const wsId = app.currentWorkspaceId
  if (wsId == null) throw new Error('No workspace selected')
  const ch = await apiCreateChannel(wsId, name, kind)
  app.workspaces = app.workspaces.map((w) =>
    w.id === wsId ? { ...w, channels: [...w.channels, ch] } : w,
  )
  if (ch.kind === 'text') void selectChannel(ch.id)
  return ch
}

let booting = false

async function bootstrap(): Promise<void> {
  if (!token.get() || booting || app.booted) return
  booting = true
  try {
    app.identity = await Identity.get()
  } catch {
    app.identity = null
  } finally {
    app.booted = true
    booting = false
  }
}

async function enter(): Promise<void> {
  if (!app.identity) throw new Error('No identity')
  const [workspaces, sounds] = await Promise.all([
    listWorkspaces(),
    listSounds().catch(() => [] as Sound[]),
  ])
  app.workspaces = workspaces
  app.sounds = sounds
  ws.connect()
  app.entered = true
  if (workspaces.length > 0) selectWorkspace(workspaces[0].id)
}

export const app = $state({
  booted: false,
  entered: false,
  identity: null as IdentityData | null,
  workspaces: [] as Workspace[],
  currentWorkspaceId: null as Id | null,
  currentChannelId: null as Id | null,
  messages: [] as Message[],
  voice: [] as VoicePresence[],
  sounds: [] as Sound[],
  members: [] as UserPublic[],
  bootstrap,
  enter,
  selectWorkspace,
  selectChannel,
  sendMessage,
  reloadSounds,
  createWorkspace,
  joinWorkspace,
  createChannel,
  appendMessage,
})

ws.on(handleEvent)

onStatus((s) => {
  net.status = s
})

onReconnect(() => {
  if (app.currentChannelId != null) {
    ws.send({ op: 'subscribe', channel_id: app.currentChannelId })
  }
})
