<script lang="ts">
  import { app } from '../lib/state.svelte'
  import { avatarColor, initials } from '../lib/format'
  import InviteDialog from './InviteDialog.svelte'
  import CreateWorkspaceDialog from './CreateWorkspaceDialog.svelte'

  let menu = $state(false)
  let creating = $state(false)
  let invite = $state(false)
  let addBtn = $state<HTMLButtonElement>()
  let menuEl = $state<HTMLDivElement>()
  let menuPos = $state({ top: 0, left: 0 })

  function toggleMenu(): void {
    if (!menu) {
      const rect = addBtn?.getBoundingClientRect()
      if (rect) menuPos = { top: rect.top, left: rect.right + 8 }
    }
    menu = !menu
  }

  function openCreate(): void {
    menu = false
    creating = true
  }

  function openInvite(): void {
    menu = false
    invite = true
  }

  function onWindowMouseDown(event: MouseEvent): void {
    if (!menu) return
    const target = event.target as Node
    if (menuEl?.contains(target) || addBtn?.contains(target)) return
    menu = false
  }

  function onWindowKeydown(event: KeyboardEvent): void {
    if (event.key === 'Escape') menu = false
  }
</script>

<svelte:window onmousedown={onWindowMouseDown} onkeydown={onWindowKeydown} onresize={() => (menu = false)} />

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

  <button
    bind:this={addBtn}
    class="ws add"
    aria-label="Adicionar workspace"
    aria-haspopup="menu"
    aria-expanded={menu}
    onclick={toggleMenu}
  >
    +
  </button>
</nav>

{#if menu}
  <div
    bind:this={menuEl}
    class="menu"
    role="menu"
    style="top: {menuPos.top}px; left: {menuPos.left}px"
  >
    <button role="menuitem" onclick={openCreate}>Criar workspace</button>
    <button role="menuitem" onclick={openInvite}>Entrar com ticket</button>
  </div>
{/if}

{#if creating}
  <CreateWorkspaceDialog onclose={() => (creating = false)} />
{/if}

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
    flex: none;
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

  .menu {
    position: fixed;
    z-index: 30;
    display: flex;
    flex-direction: column;
    min-width: 190px;
    padding: 6px;
    background: var(--bg-2);
    border: 1px solid var(--bg-3);
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
</style>
