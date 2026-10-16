<script lang="ts">
  import { onMount } from 'svelte'
  import { ApiError, getVersion, installUpdate, type VersionInfo } from '../lib/api'

  let info = $state<VersionInfo | null>(null)
  let checking = $state(false)
  let dismissed = $state(false)
  let note = $state('')
  let noteTimer: ReturnType<typeof setTimeout> | undefined

  let busy = $derived(
    info?.status === 'downloading' || info?.status === 'installing' || info?.status === 'restarting',
  )
  let visible = $derived(
    note !== '' || busy || info?.status === 'error' || (!!info?.update_available && !dismissed),
  )

  function say(text: string): void {
    note = text
    clearTimeout(noteTimer)
    noteTimer = setTimeout(() => (note = ''), 5000)
  }

  async function refresh(manual = false): Promise<void> {
    if (manual) checking = true
    try {
      info = await getVersion(manual)
      if (manual) {
        dismissed = false
        if (info.latest === null) say('Could not check for updates')
        else if (!info.update_available) say(`Discard ${info.current} is up to date`)
      }
    } catch (e) {
      if (manual) say(e instanceof ApiError ? e.message : 'Could not check for updates')
    } finally {
      checking = false
    }
  }

  async function install(): Promise<void> {
    try {
      await installUpdate()
      await refresh()
    } catch (e) {
      say(e instanceof ApiError ? e.message : 'Update failed')
    }
  }

  onMount(() => {
    void refresh()
    const timer = setInterval(() => {
      if (busy) void refresh()
    }, 1500)
    return () => {
      clearInterval(timer)
      clearTimeout(noteTimer)
    }
  })
</script>

<button
  class="check"
  type="button"
  title={info ? `Discard ${info.current}: check for updates` : 'Check for updates'}
  aria-label="Check for updates"
  disabled={checking || busy}
  onclick={() => refresh(true)}
>
  <svg
    class:spin={checking}
    viewBox="0 0 24 24"
    width="16"
    height="16"
    fill="none"
    stroke="currentColor"
    stroke-width="2"
    stroke-linecap="round"
    stroke-linejoin="round"
    aria-hidden="true"
  >
    <path d="M21 12a9 9 0 1 1-3-6.7" />
    <path d="M21 4v5h-5" />
  </svg>
</button>

{#if visible}
  <div class="banner" role="status" aria-live="polite">
    {#if info?.status === 'restarting'}
      <span>Updated. Discard is restarting; reopen it if no window appears.</span>
    {:else if info?.status === 'downloading'}
      <span>Downloading v{info.latest}…</span>
    {:else if info?.status === 'installing'}
      <span>Installing v{info.latest}…</span>
    {:else if info?.status === 'error'}
      <span class="err">Update failed: {info.error}</span>
      {#if info.installable}
        <button type="button" class="primary" aria-label="Retry update" onclick={install}>Retry</button>
      {/if}
    {:else if note}
      <span>{note}</span>
    {:else if info}
      <span>Update available: v{info.latest}</span>
      {#if info.installable}
        <button
          type="button"
          class="primary"
          aria-label={`Install version ${info.latest}`}
          onclick={install}
        >
          Install
        </button>
      {/if}
      <a
        href={info.notes_url}
        target="_blank"
        rel="noreferrer noopener"
        aria-label="What's new in this release">What's new</a
      >
      <button type="button" class="close" aria-label="Dismiss update notice" onclick={() => (dismissed = true)}>
        ×
      </button>
    {/if}
  </div>
{/if}

<style>
  .check {
    position: fixed;
    left: 22px;
    bottom: 10px;
    z-index: 40;
    width: 28px;
    height: 28px;
    display: grid;
    place-items: center;
    padding: 0;
    border: none;
    border-radius: 50%;
    background: transparent;
    color: var(--text-2);
    cursor: pointer;
  }

  .check:hover:not(:disabled) {
    background: var(--bg-3);
    color: var(--text-0);
  }

  .check:disabled {
    cursor: default;
    opacity: 0.6;
  }

  .spin {
    animation: spin 1s linear infinite;
  }

  @keyframes spin {
    to {
      transform: rotate(360deg);
    }
  }

  .banner {
    position: fixed;
    left: 60px;
    bottom: 10px;
    z-index: 40;
    max-width: min(420px, calc(100vw - 72px));
    display: flex;
    align-items: center;
    gap: var(--gap);
    padding: 8px 10px;
    background: var(--bg-2);
    border: 1px solid var(--bg-3);
    border-radius: 8px;
    box-shadow: 0 8px 24px rgba(0, 0, 0, 0.45);
    color: var(--text-1);
    font-size: 13px;
  }

  .err {
    color: var(--danger);
  }

  .banner a {
    color: var(--text-1);
    white-space: nowrap;
  }

  .primary {
    border: none;
    border-radius: var(--radius);
    background: var(--accent);
    color: var(--text-0);
    font: inherit;
    padding: 4px 10px;
    cursor: pointer;
  }

  .primary:hover {
    background: var(--accent-hover);
  }

  .close {
    border: none;
    background: transparent;
    color: var(--text-2);
    font-size: 18px;
    line-height: 1;
    cursor: pointer;
  }

  .close:hover {
    color: var(--text-0);
  }
</style>
