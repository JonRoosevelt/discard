import type {
  Attachment,
  Channel,
  ChannelKind,
  Id,
  Identity as IdentityData,
  Message,
  Sound,
  Workspace,
} from './protocol'

const TOKEN_KEY = 'discard_session'

function fromQuery(key: string): string | null {
  if (typeof location === 'undefined') return null
  const value = new URLSearchParams(location.search).get(key)
  if (value && typeof history !== 'undefined') {
    const url = new URL(location.href)
    url.searchParams.delete(key)
    history.replaceState(null, '', `${url.pathname}${url.search}${url.hash}`)
  }
  return value
}

function readStored(): string | null {
  try {
    return sessionStorage.getItem(TOKEN_KEY)
  } catch {
    return null
  }
}

function initialToken(): string | null {
  const fromUrl = fromQuery('k')
  if (fromUrl) {
    try {
      sessionStorage.setItem(TOKEN_KEY, fromUrl)
    } catch {
      void 0
    }
    return fromUrl
  }
  const stored = readStored()
  if (stored) return stored
  if (import.meta.env.DEV) return 'dev'
  return null
}

let current: string | null = initialToken()

export const token = {
  get(): string | null {
    return current
  },
  set(t: string | null): void {
    current = t
    try {
      if (t) sessionStorage.setItem(TOKEN_KEY, t)
      else sessionStorage.removeItem(TOKEN_KEY)
    } catch {
      void 0
    }
  },
}

export function hasSession(): boolean {
  return current !== null
}

export class ApiError extends Error {
  status: number

  constructor(status: number, message: string) {
    super(message)
    this.name = 'ApiError'
    this.status = status
  }
}

export async function api<T>(path: string, init: RequestInit = {}): Promise<T> {
  const headers = new Headers(init.headers)
  const body = init.body
  const isForm = typeof FormData !== 'undefined' && body instanceof FormData
  if (body != null && !isForm && !headers.has('Content-Type')) {
    headers.set('Content-Type', 'application/json')
  }
  const t = token.get()
  if (t) headers.set('Authorization', `Bearer ${t}`)

  let res: Response
  try {
    res = await fetch(`/api${path}`, { ...init, headers })
  } catch {
    throw new ApiError(0, 'Sem conexão com o nó local')
  }

  if (!res.ok) {
    let message = `Erro ${res.status}`
    try {
      const data = await res.json()
      if (data && typeof data.error === 'string') message = data.error
    } catch {
      try {
        const text = await res.text()
        if (text) message = text
      } catch {
        void 0
      }
    }
    throw new ApiError(res.status, message)
  }

  if (res.status === 204) return undefined as T
  const text = await res.text()
  return (text ? JSON.parse(text) : undefined) as T
}

export const Identity = {
  async get(): Promise<IdentityData | null> {
    try {
      return await api<IdentityData>('/identity')
    } catch (e) {
      if (e instanceof ApiError && e.status === 404) return null
      throw e
    }
  },
  create(displayName: string): Promise<IdentityData> {
    return api<IdentityData>('/identity', {
      method: 'POST',
      body: JSON.stringify({ display_name: displayName }),
    })
  },
  backupUrl(): string {
    return '/api/identity/backup'
  },
  import(backup: string, passphrase?: string): Promise<IdentityData> {
    return api<IdentityData>('/identity/import', {
      method: 'POST',
      body: JSON.stringify({ backup, passphrase: passphrase ?? null }),
    })
  },
}

export async function downloadBackup(): Promise<Blob> {
  const headers = new Headers()
  const t = token.get()
  if (t) headers.set('Authorization', `Bearer ${t}`)
  const res = await fetch(Identity.backupUrl(), { headers })
  if (!res.ok) throw new ApiError(res.status, `Erro ${res.status}`)
  return res.blob()
}

export async function uploadFile(file: File): Promise<Attachment> {
  const fd = new FormData()
  fd.append('file', file)
  return api<Attachment>('/upload', { method: 'POST', body: fd })
}

export function listWorkspaces(): Promise<Workspace[]> {
  return api<Workspace[]>('/workspaces')
}

export function createWorkspace(name: string): Promise<Workspace> {
  return api<Workspace>('/workspaces', { method: 'POST', body: JSON.stringify({ name }) })
}

export function joinWorkspace(ticket: string): Promise<Workspace> {
  return api<Workspace>('/workspaces/join', {
    method: 'POST',
    body: JSON.stringify({ ticket }),
  })
}

export async function getTicket(id: Id): Promise<string> {
  const res = await api<{ ticket: string }>(`/workspaces/${id}/ticket`)
  return res.ticket
}

export function createChannel(workspaceId: Id, name: string, kind: ChannelKind): Promise<Channel> {
  return api<Channel>(`/workspaces/${workspaceId}/channels`, {
    method: 'POST',
    body: JSON.stringify({ name, kind }),
  })
}

export function getMessages(channelId: Id): Promise<Message[]> {
  return api<Message[]>(`/channels/${channelId}/messages`)
}

export function postMessage(channelId: Id, content: string, attachmentIds: Id[] = []): Promise<Message> {
  return api<Message>(`/channels/${channelId}/messages`, {
    method: 'POST',
    body: JSON.stringify({ content, attachment_ids: attachmentIds }),
  })
}

export function listSounds(): Promise<Sound[]> {
  return api<Sound[]>('/sounds')
}

export function uploadSound(file: File, name: string, emoji: string): Promise<Sound> {
  const fd = new FormData()
  fd.append('file', file)
  fd.append('name', name)
  fd.append('emoji', emoji)
  return api<Sound>('/sounds', { method: 'POST', body: fd })
}
