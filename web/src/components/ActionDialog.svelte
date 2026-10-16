<script lang="ts">
  import { untrack } from 'svelte'

  let {
    title,
    message = '',
    value = null,
    confirmLabel,
    danger = false,
    onconfirm,
    onclose,
  }: {
    title: string
    message?: string
    value?: string | null
    confirmLabel: string
    danger?: boolean
    onconfirm: (value: string) => Promise<void>
    onclose: () => void
  } = $props()

  let text = $state(untrack(() => value ?? ''))
  let error = $state('')
  let busy = $state(false)
  let inputEl = $state<HTMLInputElement>()
  let confirmEl = $state<HTMLButtonElement>()

  $effect(() => {
    if (inputEl) {
      inputEl.focus()
      inputEl.select()
    } else {
      confirmEl?.focus()
    }
  })

  function onKeydown(event: KeyboardEvent): void {
    if (event.key === 'Escape') onclose()
  }

  async function submit(event: SubmitEvent): Promise<void> {
    event.preventDefault()
    if (busy) return
    const v = text.trim()
    if (value !== null && !v) {
      error = 'Name is required'
      return
    }
    busy = true
    error = ''
    try {
      await onconfirm(v)
      onclose()
    } catch (e) {
      error = e instanceof Error ? e.message : 'Something went wrong'
    } finally {
      busy = false
    }
  }
</script>

<svelte:window onkeydown={onKeydown} />

<div class="overlay">
  <div class="modal" role="dialog" aria-modal="true" aria-label={title}>
    <header>
      <h2>{title}</h2>
      <button class="close" aria-label="Close" onclick={onclose}>×</button>
    </header>
    <form onsubmit={submit}>
      {#if message}
        <p class="message">{message}</p>
      {/if}
      {#if value !== null}
        <label for="action-name">Name</label>
        <input
          id="action-name"
          bind:this={inputEl}
          bind:value={text}
          maxlength="40"
          autocomplete="off"
        />
      {/if}
      {#if error}
        <p class="error">{error}</p>
      {/if}
      <div class="actions">
        <button type="button" class="cancel" onclick={onclose}>Cancel</button>
        <button type="submit" class="confirm" class:danger bind:this={confirmEl} disabled={busy}>
          {confirmLabel}
        </button>
      </div>
    </form>
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

  form {
    display: flex;
    flex-direction: column;
    gap: 6px;
    padding-top: 8px;
    border-top: 1px solid var(--bg-3);
  }

  .message {
    margin: 0 0 4px;
    font-size: 14px;
    color: var(--text-1);
  }

  label {
    font-size: 12px;
    text-transform: uppercase;
    letter-spacing: 0.4px;
    color: var(--text-2);
  }

  input {
    background: var(--bg-0);
    border: 1px solid var(--bg-3);
    border-radius: var(--radius);
    color: var(--text-0);
    font: inherit;
    font-size: 14px;
    padding: 9px 10px;
    outline: none;
  }

  input:focus {
    border-color: var(--accent);
  }

  .actions {
    display: flex;
    justify-content: flex-end;
    gap: 8px;
    margin-top: 4px;
  }

  .actions button {
    padding: 9px 14px;
    border: none;
    border-radius: var(--radius);
    font: inherit;
    font-weight: 600;
    cursor: pointer;
  }

  .cancel {
    background: transparent;
    color: var(--text-1);
  }

  .cancel:hover {
    text-decoration: underline;
  }

  .confirm {
    background: var(--accent);
    color: #fff;
  }

  .confirm:hover:not(:disabled) {
    background: var(--accent-hover);
  }

  .confirm.danger {
    background: var(--danger);
  }

  .confirm.danger:hover:not(:disabled) {
    filter: brightness(0.9);
  }

  .confirm:disabled {
    opacity: 0.6;
    cursor: default;
  }

  .error {
    margin: 0;
    font-size: 12px;
    color: var(--danger);
  }
</style>
