<script lang="ts">
  import { app } from '../lib/state.svelte'
  import { avatarColor, initials } from '../lib/format'
  import InviteDialog from './InviteDialog.svelte'

  let menu = $state(false)
  let adding = $state(false)
  let invite = $state(false)
  let name = $state('')
  let error = $state('')
  let inputEl = $state<HTMLInputElement>()

  $effect(() => {
    if (adding && inputEl) inputEl.focus()
  })

  async function create(): Promise<void> {
    const n = name.trim()
    if (!n) {
      adding = false
      name = ''
      return
    }
    try {
      await app.createWorkspace(n)
      adding = false
      name = ''
      error = ''
    } catch (e) {
      error = e instanceof Error ? e.message : 'Falha ao criar'
    }
  }

  function openCreate(): void {
    menu = false
    adding = true
    error = ''
  }

  function openInvite(): void {
    menu = false
    invite = true
  }

  function onKeydown(e: KeyboardEvent): void {
    if (e.key === 'Enter') void create()
    if (e.key === 'Escape') {
      adding = false
      name = ''
      error = ''
    }
  }
</script>

<nav class="rail" aria-label="Workspaces">
  {#each app.workspaces as wsp (wsp.id)}
    <button
      class="ws"
      class:active={wsp.id === app.currentWorkspaceId}
      title={wsp.name}
      aria-label={wsp.name}
      aria-current={wsp.id === app.currentWorkspaceId ? 'true' : undefined}
      onclick={() => app.selectWorkspace(wsp.id)}
    >
      {#if wsp.icon_url}
        <img src={wsp.icon_url} alt="" />
      {:else}
        <span class="badge" style="background: {avatarColor(wsp.name)}">
          {initials(wsp.name, 'S')}
        </span>
      {/if}
    </button>
  {/each}

  {#if adding}
    <div class="new">
      <input
        bind:this={inputEl}
        bind:value={name}
        onkeydown={onKeydown}
        maxlength="40"
        placeholder="nome"
        aria-label="Nome do workspace"
      />
      {#if error}
        <span class="error">{error}</span>
      {/if}
    </div>
  {:else}
    <div class="add-wrap">
      <button
        class="ws add"
        aria-label="Adicionar workspace"
        aria-haspopup="menu"
        aria-expanded={menu}
        onclick={() => (menu = !menu)}
      >
        +
      </button>
      {#if menu}
        <div class="menu" role="menu">
          <button role="menuitem" onclick={openCreate}>Criar workspace</button>
          <button role="menuitem" onclick={openInvite}>Entrar com ticket</button>
        </div>
      {/if}
    </div>
  {/if}
</nav>

{#if invite}
  <InviteDialog onclose={() => (invite = false)} />
{/if}

<style>
  .rail {
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: var(--gap);
    padding: 12px 0;
    background: var(--bg-0);
    overflow-y: auto;
  }

  .ws {
    position: relative;
    width: 48px;
    height: 48px;
    border: none;
    border-radius: 50%;
    background: var(--bg-1);
    color: var(--text-0);
    cursor: pointer;
    display: grid;
    place-items: center;
    padding: 0;
    overflow: hidden;
    transition: border-radius 0.15s, background 0.15s;
  }

  .ws:hover {
    border-radius: 16px;
    background: var(--accent);
  }

  .ws.active {
    border-radius: 16px;
    background: var(--accent);
  }

  .ws::before {
    content: '';
    position: absolute;
    left: -12px;
    top: 50%;
    transform: translateY(-50%);
    width: 4px;
    height: 0;
    border-radius: 0 4px 4px 0;
    background: var(--text-0);
    transition: height 0.15s;
  }

  .ws.active::before,
  .ws:hover::before {
    height: 24px;
  }

  .ws img {
    width: 48px;
    height: 48px;
    object-fit: cover;
  }

  .badge {
    font-size: 15px;
    font-weight: 600;
    letter-spacing: 0.3px;
  }

  .add {
    color: var(--ok);
    font-size: 24px;
    font-weight: 400;
  }

  .add-wrap {
    position: relative;
  }

  .menu {
    position: absolute;
    left: 56px;
    top: 0;
    z-index: 20;
    display: flex;
    flex-direction: column;
    min-width: 180px;
    padding: 6px;
    background: var(--bg-2);
    border-radius: 8px;
    box-shadow: 0 8px 24px rgba(0, 0, 0, 0.45);
  }

  .menu button {
    background: transparent;
    border: none;
    color: var(--text-1);
    font: inherit;
    font-size: 13px;
    text-align: left;
    padding: 8px 10px;
    border-radius: var(--radius);
    cursor: pointer;
    white-space: nowrap;
  }

  .menu button:hover {
    background: var(--bg-3);
    color: var(--text-0);
  }

  .new {
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 4px;
  }

  .new input {
    width: 52px;
    background: var(--bg-1);
    border: 1px solid var(--accent);
    border-radius: 8px;
    color: var(--text-0);
    font: inherit;
    font-size: 12px;
    padding: 6px 8px;
    text-align: center;
    outline: none;
  }

  .error {
    color: var(--danger);
    font-size: 10px;
    text-align: center;
    max-width: 60px;
  }
</style>
