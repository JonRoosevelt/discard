<script lang="ts">
  import { onDestroy } from 'svelte'
  import { app } from '../lib/state.svelte'
  import { ws } from '../lib/ws'
  import { VoiceSession } from '../lib/rtc'
  import VideoTile from './VideoTile.svelte'
  import Soundboard from './Soundboard.svelte'
  import type { Channel, Id } from '../lib/protocol'

  interface Props {
    stage?: boolean
  }

  interface TileView {
    key: Id
    stream: MediaStream | null
    name: string
    local: boolean
    micMuted: boolean
    video: boolean
    screen: boolean
    deafened: boolean
    hasVideo: boolean
    conn: string
  }

  const SELF = 'self'

  let { stage = false }: Props = $props()

  let session = $state<VoiceSession | null>(null)
  let tick = $state(0)
  let micOn = $state(true)
  let deafened = $state(false)
  let camOn = $state(false)
  let screenOn = $state(false)
  let spotlight = $state<Id | null>(null)
  let showBoard = $state(false)

  onDestroy(() => session?.leave())

  let selfId = $derived(app.identity?.public_id ?? '')

  let myPresence = $derived(
    app.voice.find((p) => p.members.some((m) => m.user.id === selfId)) ?? null,
  )

  let myChannel = $derived.by((): Channel | null => {
    const presence = myPresence
    if (!presence) return null
    for (const workspace of app.workspaces) {
      const channel = workspace.channels.find((c) => c.id === presence.channel_id)
      if (channel) return channel
    }
    return null
  })

  let members = $derived(myPresence?.members ?? [])
  let others = $derived(members.filter((m) => m.user.id !== selfId))

  let tiles = $derived.by(() => {
    void tick
    return others.map((member) => {
      const peer = session?.peers.get(member.user.id)
      const stream = peer?.stream ?? null
      return {
        member,
        stream,
        hasVideo:
          (member.video || member.screen) && !!stream && stream.getVideoTracks().length > 0,
        conn: peer?.conn ?? '',
      }
    })
  })

  let localPreview = $derived.by(() => {
    void tick
    const active = session
    return active ? (active.screenStream ?? active.localStream) : null
  })

  let localHasVideo = $derived.by(() => {
    void tick
    return !!localPreview && localPreview.getVideoTracks().length > 0
  })

  let views = $derived.by((): TileView[] => [
    {
      key: SELF,
      stream: localPreview,
      name: app.identity?.display_name ?? 'You',
      local: true,
      micMuted: !micOn,
      video: camOn,
      screen: screenOn,
      deafened: false,
      hasVideo: localHasVideo,
      conn: '',
    },
    ...tiles.map((tile) => ({
      key: tile.member.user.id,
      stream: tile.stream,
      name: tile.member.user.display_name,
      local: false,
      micMuted: tile.member.muted,
      video: tile.member.video,
      screen: tile.member.screen,
      deafened,
      hasVideo: tile.hasVideo,
      conn: tile.conn,
    })),
  ])

  let focused = $derived(stage ? (views.find((v) => v.key === spotlight) ?? null) : null)

  let connected = $derived.by(() => {
    void tick
    return session?.connected ?? false
  })

  let mediaError = $derived.by(() => {
    void tick
    return session?.error ?? ''
  })

  $effect(() => {
    const pid = app.identity?.public_id ?? ''
    const presence = pid
      ? (app.voice.find((p) => p.members.some((m) => m.user.id === pid)) ?? null)
      : null
    const channel = presence?.channel_id ?? null
    const active = session
    if (!pid) return
    if (channel == null) {
      if (active) {
        active.leave()
        session = null
      }
      return
    }
    if (active && active.channelId === channel) return
    if (active) {
      active.leave()
      session = null
    }
    const mine = presence?.members.find((m) => m.user.id === pid) ?? null
    const muted = mine?.muted ?? false
    const deaf = mine?.deafened ?? false
    const cam = mine?.video ?? false
    const next = new VoiceSession({
      selfId: pid,
      onChange: () => {
        tick += 1
      },
    })
    session = next
    micOn = !muted
    deafened = deaf
    camOn = cam
    screenOn = false
    next.setMuted(muted)
    next.setDeafened(deaf)
    void next.join(
      channel,
      (presence?.members ?? []).map((m) => m.user.id),
    )
    if (cam) void next.setVideo(true)
  })

  $effect(() => {
    void tick
    const active = session
    if (!active) return
    if (active.screen !== screenOn) {
      screenOn = active.screen
      update()
    }
  })

  function update(): void {
    ws.send({ op: 'voice_update', muted: !micOn, deafened, video: camOn, screen: screenOn })
  }

  function toggleMic(): void {
    micOn = !micOn
    session?.setMuted(!micOn)
    update()
  }

  function toggleDeafen(): void {
    deafened = !deafened
    session?.setDeafened(deafened)
    update()
  }

  function toggleCam(): void {
    void changeCam()
  }

  async function changeCam(): Promise<void> {
    const next = !camOn
    camOn = next
    await session?.setVideo(next)
    camOn = session?.video ?? camOn
    update()
  }

  function toggleScreen(): void {
    void changeScreen()
  }

  async function changeScreen(): Promise<void> {
    const next = !screenOn
    screenOn = next
    await session?.setScreen(next)
    screenOn = session?.screen ?? screenOn
    update()
  }

  function select(key: Id): void {
    if (!stage) {
      spotlight = key
      if (myPresence) void app.selectChannel(myPresence.channel_id)
      return
    }
    spotlight = spotlight === key ? null : key
  }

  function onWindowKeydown(e: KeyboardEvent): void {
    if (e.key === 'Escape' && focused && !e.defaultPrevented) spotlight = null
  }

  function leave(): void {
    const presence = myPresence
    if (presence) ws.send({ op: 'voice_leave', channel_id: presence.channel_id })
  }
</script>

<svelte:window onkeydown={onWindowKeydown} />

{#snippet tile(view: TileView, compact: boolean)}
  <VideoTile
    stream={view.stream}
    name={view.name}
    local={view.local}
    micMuted={view.micMuted}
    video={view.video}
    screen={view.screen}
    deafened={view.deafened}
    hasVideo={view.hasVideo}
    conn={view.conn}
    spotlit={focused?.key === view.key}
    {compact}
    onselect={() => select(view.key)}
  />
{/snippet}

<section class={stage ? 'stage' : 'panel'} aria-label="Voice">
  {#if !myPresence}
    <div class="empty">
      <span class="empty-icon">🎧</span>
      <p>You are not in a voice channel.</p>
    </div>
  {:else}
    <header class="head">
      <div class="title">
        <span class="glyph">🔊</span>
        <strong>{myChannel?.name ?? 'Voice channel'}</strong>
      </div>
      <span class="status" class:on={connected}>
        {connected ? 'Connected' : 'Connecting…'}
      </span>
    </header>

    {#if mediaError}
      <p class="error">{mediaError}</p>
    {/if}

    {#if focused}
      <div class="focus">
        {@render tile(focused, false)}
        <button
          class="unfocus"
          aria-label="Return to grid"
          title="Return to grid (Esc)"
          onclick={() => (spotlight = null)}
        >
          ✕
        </button>
      </div>
      {#if views.length > 1}
        <div class="strip">
          {#each views as view (view.key)}
            {#if view.key !== focused.key}{@render tile(view, true)}{/if}
          {/each}
        </div>
      {/if}
    {:else}
      <div class="grid">
        {#each views as view (view.key)}
          {@render tile(view, false)}
        {/each}
      </div>
    {/if}

    {#if !stage || showBoard}
      <div class="soundboard">
        <Soundboard />
      </div>
    {/if}

    <div class="controls">
      <button
        class="ctl"
        class:off={!micOn}
        aria-pressed={micOn}
        aria-label={micOn ? 'Mute microphone' : 'Unmute microphone'}
        title={micOn ? 'Mute microphone' : 'Unmute microphone'}
        onclick={toggleMic}
      >
        {micOn ? '🎤' : '🔇'}
      </button>
      <button
        class="ctl"
        class:off={deafened}
        aria-pressed={deafened}
        aria-label={deafened ? 'Undeafen' : 'Deafen'}
        title={deafened ? 'Undeafen' : 'Deafen'}
        onclick={toggleDeafen}
      >
        {deafened ? '🔈' : '🔊'}
      </button>
      <button
        class="ctl"
        class:on={camOn}
        aria-pressed={camOn}
        aria-label={camOn ? 'Turn off camera' : 'Turn on camera'}
        title={camOn ? 'Turn off camera' : 'Turn on camera'}
        onclick={toggleCam}
      >
        📹
      </button>
      <button
        class="ctl"
        class:on={screenOn}
        aria-pressed={screenOn}
        aria-label={screenOn ? 'Stop sharing' : 'Share screen'}
        title={screenOn ? 'Stop sharing' : 'Share screen'}
        onclick={toggleScreen}
      >
        🖥️
      </button>
      {#if stage}
        <button
          class="ctl"
          class:on={showBoard}
          aria-pressed={showBoard}
          aria-label={showBoard ? 'Hide soundboard' : 'Show soundboard'}
          title={showBoard ? 'Hide soundboard' : 'Show soundboard'}
          onclick={() => (showBoard = !showBoard)}
        >
          🎵
        </button>
      {/if}
      <button class="ctl danger" aria-label="Leave voice channel" title="Leave" onclick={leave}>
        ✖
      </button>
    </div>
  {/if}
</section>

<style>
  .panel {
    display: flex;
    flex-direction: column;
    min-height: 0;
    background: var(--bg-1);
    border-left: 1px solid var(--bg-0);
  }

  .stage {
    display: flex;
    flex-direction: column;
    min-width: 0;
    min-height: 0;
    background: var(--bg-0);
  }

  .stage .head {
    background: var(--bg-2);
  }

  .stage .grid {
    grid-template-columns: repeat(auto-fit, minmax(240px, 1fr));
    gap: 12px;
    padding: 16px;
    align-content: center;
  }

  .stage .grid :global(.tile) {
    aspect-ratio: 16 / 9;
  }

  .focus {
    position: relative;
    flex: 1;
    min-height: 0;
    display: flex;
    padding: 12px 12px 0;
  }

  .unfocus {
    position: absolute;
    top: 20px;
    right: 20px;
    width: 32px;
    height: 32px;
    border: none;
    border-radius: 50%;
    background: rgba(0, 0, 0, 0.6);
    color: var(--text-0);
    font-size: 14px;
    cursor: pointer;
  }

  .unfocus:hover {
    background: rgba(0, 0, 0, 0.85);
  }

  .strip {
    flex: none;
    height: 104px;
    display: flex;
    justify-content: center;
    gap: 8px;
    padding: 8px 12px;
    overflow-x: auto;
  }

  .empty {
    flex: 1;
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    gap: 10px;
    padding: 24px;
    text-align: center;
    color: var(--text-2);
  }

  .empty-icon {
    font-size: 28px;
    opacity: 0.7;
  }

  .empty p {
    margin: 0;
    font-size: 13px;
  }

  .head {
    height: 48px;
    flex: none;
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 8px;
    padding: 0 12px;
    border-bottom: 1px solid var(--bg-0);
  }

  .title {
    display: flex;
    align-items: center;
    gap: 6px;
    min-width: 0;
    color: var(--text-0);
  }

  .title strong {
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
    font-size: 14px;
  }

  .status {
    flex: none;
    font-size: 11px;
    color: var(--text-2);
  }

  .status.on {
    color: var(--ok);
  }

  .error {
    margin: 0;
    padding: 6px 12px;
    color: var(--danger);
    font-size: 12px;
  }

  .grid {
    flex: 1;
    overflow-y: auto;
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(120px, 1fr));
    gap: 8px;
    padding: 10px;
    align-content: start;
  }

  .soundboard {
    flex: none;
    border-top: 1px solid var(--bg-0);
    padding: 8px;
  }

  .controls {
    flex: none;
    display: flex;
    gap: 6px;
    padding: 8px 12px;
    border-top: 1px solid var(--bg-0);
    background: var(--bg-0);
  }

  .ctl {
    flex: 1;
    height: 34px;
    border: none;
    border-radius: var(--radius);
    background: var(--bg-3);
    color: var(--text-1);
    font-size: 15px;
    cursor: pointer;
    line-height: 1;
  }

  .ctl:hover {
    background: #4a4d54;
  }

  .ctl.on {
    background: var(--accent);
    color: #fff;
  }

  .ctl.off {
    background: var(--danger);
    color: #fff;
  }

  .ctl.danger {
    background: transparent;
    border: 1px solid var(--danger);
    color: var(--danger);
  }

  .ctl.danger:hover {
    background: var(--danger);
    color: #fff;
  }
</style>
