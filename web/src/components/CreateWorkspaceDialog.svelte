<script lang="ts">
  import { app } from '../lib/state.svelte'

  let { onclose }: { onclose: () => void } = $props()

  let name = $state('')
  let error = $state('')
  let busy = $state(false)
  let inputEl = $state<HTMLInputElement>()

  $effect(() => {
    inputEl?.focus()
  })

  function onKeydown(event: KeyboardEvent): void {
    if (event.key === 'Escape') onclose()
  }

  async function submit(event: SubmitEvent): Promise<void> {
    event.preventDefault()
    const value = name.trim()
    if (!value) {
      error = 'Dê um nome ao workspace'
      return
    }
    busy = true
    error = ''
    try {
      await app.createWorkspace(value)
      onclose()
    } catch (e) {
      error = e instanceof Error ? e.message : 'Falha ao criar'
    } finally {
      busy = false
    }
  }
</script>

<svelte:window onkeydown={onKeydown} />

<div class="overlay">
  <div class="modal" role="dialog" aria-modal="true" aria-label="Criar workspace">
    <header>
      <h2>Criar workspace</h2>
      <button class="close" aria-label="Fechar" onclick={onclose}>×</button>
    </header>
    <form onsubmit={submit}>
      <label for="ws-name">Nome</label>
      <input
        id="ws-name"
        bind:this={inputEl}
        bind:value={name}
        maxlength="40"
        placeholder="Meu servidor"
        autocomplete="off"
      />
      {#if error}
        <p class="error">{error}</p>
      {/if}
      <button type="submit" disabled={busy}>{busy ? 'Criando…' : 'Criar'}</button>
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

  button[type='submit'] {
    margin-top: 4px;
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

  .error {
    margin: 0;
    font-size: 12px;
    color: var(--danger);
  }
</style>
