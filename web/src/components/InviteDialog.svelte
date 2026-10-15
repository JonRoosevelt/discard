<script lang="ts">
  import { app } from '../lib/state.svelte'
  import { getTicket } from '../lib/api'

  let { onclose }: { onclose: () => void } = $props()

  let ticket = $state('')
  let loading = $state(true)
  let loadError = $state('')
  let join = $state('')
  let joinError = $state('')
  let busy = $state(false)
  let copied = $state(false)

  let workspace = $derived(app.workspaces.find((w) => w.id === app.currentWorkspaceId) ?? null)

  $effect(() => {
    const id = app.currentWorkspaceId
    if (id == null) {
      loading = false
      return
    }
    let cancelled = false
    loading = true
    loadError = ''
    getTicket(id)
      .then((t) => {
        if (!cancelled) ticket = t
      })
      .catch((e) => {
        if (!cancelled) loadError = e instanceof Error ? e.message : 'Failed to get ticket'
      })
      .finally(() => {
        if (!cancelled) loading = false
      })
    return () => {
      cancelled = true
    }
  })

  async function copy(): Promise<void> {
    if (!ticket) return
    try {
      await navigator.clipboard.writeText(ticket)
      copied = true
      setTimeout(() => (copied = false), 1500)
    } catch {
      loadError = 'Could not copy'
    }
  }

  async function submitJoin(event: SubmitEvent): Promise<void> {
    event.preventDefault()
    const value = join.trim()
    if (!value) {
      joinError = 'Paste a ticket'
      return
    }
    busy = true
    joinError = ''
    try {
      await app.joinWorkspace(value)
      onclose()
    } catch (e) {
      joinError = e instanceof Error ? e.message : 'Failed to join'
    } finally {
      busy = false
    }
  }

  function onKeydown(event: KeyboardEvent): void {
    if (event.key === 'Escape') onclose()
  }
</script>

<svelte:window onkeydown={onKeydown} />

<div class="overlay">
  <div class="modal" role="dialog" aria-modal="true" aria-label="Workspace invite">
    <header>
      <h2>Invite</h2>
      <button class="close" aria-label="Close" onclick={onclose}>×</button>
    </header>

    <section>
      <p class="label">Ticket for {workspace?.name ?? 'workspace'}</p>
      {#if loading}
        <p class="hint">Loading…</p>
      {:else if loadError}
        <p class="error">{loadError}</p>
      {:else if ticket}
        <div class="ticket">
          <code>{ticket}</code>
          <button class="copy" onclick={copy}>{copied ? 'Copied' : 'Copy'}</button>
        </div>
        <p class="hint">The ticket grants access to the workspace. Share it only with those who should join.</p>
      {:else}
        <p class="hint">Select a workspace to see its ticket.</p>
      {/if}
    </section>

    <section>
      <form onsubmit={submitJoin}>
        <label for="join-ticket">Join with a ticket</label>
        <textarea id="join-ticket" bind:value={join} placeholder="paste the ticket here"></textarea>
        {#if joinError}
          <p class="error">{joinError}</p>
        {/if}
        <button type="submit" disabled={busy}>{busy ? 'Joining…' : 'Join'}</button>
      </form>
    </section>
  </div>
</div>

<style>
  header {
    display: flex;
    align-items: center;
    justify-content: space-between;
  }

  h2 {
    margin: 0;
    font-size: 16px;
    color: var(--text-0);
  }

  .close {
    background: transparent;
    border: none;
    color: var(--text-2);
    font-size: 20px;
    line-height: 1;
    cursor: pointer;
    padding: 0 4px;
  }

  .close:hover {
    color: var(--text-0);
  }

  section {
    display: flex;
    flex-direction: column;
    gap: 6px;
    padding-top: 8px;
    border-top: 1px solid var(--bg-3);
  }

  .label {
    margin: 0;
    font-size: 11px;
    text-transform: uppercase;
    letter-spacing: 0.5px;
    color: var(--text-2);
  }

  .ticket {
    display: flex;
    align-items: stretch;
    gap: 6px;
    background: var(--bg-0);
    border: 1px solid var(--bg-3);
    border-radius: var(--radius);
    padding: 6px;
  }

  .ticket code {
    flex: 1;
    min-width: 0;
    font-family: ui-monospace, SFMono-Regular, Menlo, monospace;
    font-size: 11px;
    color: var(--text-1);
    word-break: break-all;
    max-height: 96px;
    overflow-y: auto;
  }

  .copy {
    flex: none;
    align-self: flex-start;
    background: var(--accent);
    color: #fff;
    border: none;
    border-radius: var(--radius);
    font: inherit;
    font-size: 12px;
    padding: 5px 10px;
    cursor: pointer;
  }

  .copy:hover {
    background: var(--accent-hover);
  }

  form {
    display: flex;
    flex-direction: column;
    gap: 6px;
  }

  label {
    font-size: 12px;
    text-transform: uppercase;
    letter-spacing: 0.4px;
    color: var(--text-2);
  }

  textarea {
    resize: vertical;
    min-height: 64px;
    background: var(--bg-0);
    border: 1px solid var(--bg-3);
    border-radius: var(--radius);
    color: var(--text-0);
    font: inherit;
    font-size: 12px;
    padding: 8px 10px;
    outline: none;
  }

  textarea:focus {
    border-color: var(--accent);
  }

  button[type='submit'] {
    padding: 9px;
    border: none;
    border-radius: var(--radius);
    background: var(--accent);
    color: #fff;
    font: inherit;
    font-weight: 600;
    cursor: pointer;
  }

  button[type='submit']:hover:not(:disabled) {
    background: var(--accent-hover);
  }

  button[type='submit']:disabled {
    opacity: 0.6;
    cursor: default;
  }

  .hint {
    margin: 0;
    font-size: 12px;
    color: var(--text-2);
  }

  .error {
    margin: 0;
    font-size: 12px;
    color: var(--danger);
  }
</style>
