<script lang="ts">
  import { app } from '../lib/state.svelte'
  import { ws } from '../lib/ws'
  import { VoiceSession } from '../lib/rtc'
  import VideoTile from './VideoTile.svelte'
  import Soundboard from './Soundboard.svelte'
  import type { Channel } from '../lib/protocol'

  let session = $state<VoiceSession | null>(null)
  let tick = $state(0)
  let micOn = $state(true)
  let deafened = $state(false)
  let camOn = $state(false)
  let screenOn = $state(false)

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

  function leave(): void {
    const presence = myPresence
    if (presence) ws.send({ op: 'voice_leave', channel_id: presence.channel_id })
  }
</script>

<aside class="panel" aria-label="Voz">
  {#if !myPresence}
    <div class="empty">
      <span class="empty-icon">🎧</span>
      <p>Você não está em um canal de voz.</p>
    </div>
  {:else}
    <header class="head">
      <div class="title">
        <span class="glyph">🔊</span>
        <strong>{myChannel?.name ?? 'Canal de voz'}</strong>
      </div>
      <span class="status" class:on={connected}>
        {connected ? 'Conectado' : 'Conectando…'}
      </span>
    </header>

    {#if mediaError}
      <p class="error">{mediaError}</p>
    {/if}

    <div class="grid">
      <VideoTile
        stream={localPreview}
        name={app.identity?.display_name ?? 'Você'}
        local={true}
        micMuted={!micOn}
        video={camOn}
        screen={screenOn}
        hasVideo={localHasVideo}
      />
      {#each tiles as tile (tile.member.user.id)}
        <VideoTile
          stream={tile.stream}
          name={tile.member.user.display_name}
          micMuted={tile.member.muted}
          video={tile.member.video}
          screen={tile.member.screen}
          deafened={deafened}
          hasVideo={tile.hasVideo}
          conn={tile.conn}
        />
      {/each}
    </div>

    <div class="soundboard">
      <Soundboard />
    </div>

    <div class="controls">
      <button
        class="ctl"
        class:off={!micOn}
        aria-pressed={micOn}
        aria-label={micOn ? 'Silenciar microfone' : 'Ativar microfone'}
        title={micOn ? 'Silenciar microfone' : 'Ativar microfone'}
        onclick={toggleMic}
      >
        {micOn ? '🎤' : '🔇'}
      </button>
      <button
        class="ctl"
        class:off={deafened}
        aria-pressed={deafened}
        aria-label={deafened ? 'Cancelar surdez' : 'Ensurdar'}
        title={deafened ? 'Cancelar surdez' : 'Ensurdar'}
        onclick={toggleDeafen}
      >
        {deafened ? '🔈' : '🔊'}
      </button>
      <button
        class="ctl"
        class:on={camOn}
        aria-pressed={camOn}
        aria-label={camOn ? 'Desligar câmera' : 'Ligar câmera'}
        title={camOn ? 'Desligar câmera' : 'Ligar câmera'}
        onclick={toggleCam}
      >
        📹
      </button>
      <button
        class="ctl"
        class:on={screenOn}
        aria-pressed={screenOn}
        aria-label={screenOn ? 'Parar compartilhamento' : 'Compartilhar tela'}
        title={screenOn ? 'Parar compartilhamento' : 'Compartilhar tela'}
        onclick={toggleScreen}
      >
        🖥️
      </button>
      <button class="ctl danger" aria-label="Sair do canal de voz" title="Sair" onclick={leave}>
        ✖
      </button>
    </div>
  {/if}
</aside>

<style>
  .panel {
    display: flex;
    flex-direction: column;
    min-height: 0;
    background: var(--bg-1);
    border-left: 1px solid var(--bg-0);
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
