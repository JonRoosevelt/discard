<script lang="ts">
  import { avatarColor, initials } from '../lib/format'

  interface Props {
    stream?: MediaStream | null
    name?: string
    local?: boolean
    micMuted?: boolean
    video?: boolean
    screen?: boolean
    deafened?: boolean
    hasVideo?: boolean
    conn?: string
    spotlit?: boolean
    compact?: boolean
    watchers?: string[]
    onselect?: () => void
  }

  let {
    stream = null,
    name = '',
    local = false,
    micMuted = false,
    video = false,
    screen = false,
    deafened = false,
    hasVideo = false,
    conn = '',
    spotlit = false,
    compact = false,
    watchers = [],
    onselect,
  }: Props = $props()

  let el = $state<HTMLVideoElement>()

  let status = $derived(local || !conn || conn === 'connected' ? '' : conn)

  $effect(() => {
    if (el && el.srcObject !== stream) el.srcObject = stream
  })

  $effect(() => {
    void hasVideo
    if (el && stream) void el.play().catch(() => {})
  })

  function onKeydown(e: KeyboardEvent): void {
    if (e.key !== 'Enter' && e.key !== ' ') return
    e.preventDefault()
    onselect?.()
  }
</script>

<div
  class="tile"
  class:novideo={!hasVideo}
  class:spotlit
  class:compact
  class:screen
  role="button"
  tabindex="0"
  aria-pressed={spotlit}
  aria-label={spotlit ? `Return to grid from ${name}` : `Focus ${name}`}
  onclick={() => onselect?.()}
  onkeydown={onKeydown}
>
  <video bind:this={el} autoplay playsinline muted={local || deafened}></video>

  {#if !hasVideo}
    <span class="avatar" style="background: {avatarColor(name)}">{initials(name)}</span>
  {/if}

  {#if status}
    <span class="conn">{status}</span>
  {/if}

  {#if watchers.length > 0}
    <span class="watchers" title="Watching your screen">👁 {watchers.join(', ')}</span>
  {/if}

  <div class="bar">
    <span class="name">{name}</span>
    <span class="flags">
      {#if micMuted}<span title="Microphone muted">🔇</span>{/if}
      {#if screen}<span title="Sharing screen">🖥️</span>{/if}
      {#if video}<span title="Camera on">📹</span>{/if}
      {#if deafened && !local}<span title="Audio muted by you">🔈</span>{/if}
    </span>
  </div>
</div>

<style>
  .tile {
    position: relative;
    aspect-ratio: 4 / 3;
    background: #111214;
    border-radius: var(--radius);
    overflow: hidden;
    display: grid;
    place-items: center;
    cursor: pointer;
    outline: 2px solid transparent;
    outline-offset: -2px;
  }

  .tile:hover {
    outline-color: var(--bg-3);
  }

  .tile:focus-visible {
    outline-color: var(--accent);
  }

  .tile.spotlit {
    aspect-ratio: auto;
    flex: 1;
    min-width: 0;
    height: 100%;
  }

  .tile.compact {
    flex: none;
    height: 100%;
    aspect-ratio: 16 / 9;
  }

  video {
    width: 100%;
    height: 100%;
    object-fit: cover;
    display: block;
  }

  .screen video {
    object-fit: contain;
  }

  .novideo video {
    display: none;
  }

  .avatar {
    width: 48px;
    height: 48px;
    border-radius: 50%;
    display: grid;
    place-items: center;
    color: #fff;
    font-weight: 600;
    font-size: 16px;
  }

  .conn {
    position: absolute;
    top: 4px;
    left: 4px;
    padding: 1px 6px;
    border-radius: 999px;
    background: rgba(0, 0, 0, 0.6);
    color: var(--text-2);
    font-size: 10px;
  }

  .watchers {
    position: absolute;
    top: 4px;
    right: 4px;
    max-width: calc(100% - 8px);
    padding: 1px 6px;
    border-radius: 999px;
    background: rgba(0, 0, 0, 0.6);
    color: var(--text-0);
    font-size: 11px;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }

  .bar {
    position: absolute;
    left: 0;
    right: 0;
    bottom: 0;
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 6px;
    padding: 4px 6px;
    font-size: 12px;
    color: var(--text-0);
    background: linear-gradient(transparent, rgba(0, 0, 0, 0.7));
  }

  .name {
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }

  .flags {
    display: inline-flex;
    gap: 3px;
    font-size: 11px;
    flex: none;
  }
</style>
