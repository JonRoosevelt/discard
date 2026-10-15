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
</script>

<div class="tile" class:novideo={!hasVideo}>
  <video bind:this={el} autoplay playsinline muted={local || deafened}></video>

  {#if !hasVideo}
    <span class="avatar" style="background: {avatarColor(name)}">{initials(name)}</span>
  {/if}

  {#if status}
    <span class="conn">{status}</span>
  {/if}

  <div class="bar">
    <span class="name">{name}</span>
    <span class="flags">
      {#if micMuted}<span title="Microfone silenciado">🔇</span>{/if}
      {#if screen}<span title="Compartilhando tela">🖥️</span>{/if}
      {#if video}<span title="Câmera ligada">📹</span>{/if}
      {#if deafened && !local}<span title="Áudio silenciado por você">🔈</span>{/if}
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
  }

  video {
    width: 100%;
    height: 100%;
    object-fit: cover;
    display: block;
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
