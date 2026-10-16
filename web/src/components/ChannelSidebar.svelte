<script lang="ts">
  import { app, net } from '../lib/state.svelte'
  import { shortPublicId } from '../lib/identity'
  import { avatarColor, initials } from '../lib/format'
  import InviteDialog from './InviteDialog.svelte'
  import ActionDialog from './ActionDialog.svelte'
  import type { ChannelKind, Id, VoiceMember } from '../lib/protocol'

  type Dialog = {
    kind: 'rename-ws' | 'delete-ws' | 'leave-ws' | 'rename-ch' | 'delete-ch'
    id: Id
    name: string
  }

  let menuOpen = $state(false)
  let menuBtn = $state<HTMLButtonElement>()
  let menuEl = $state<HTMLDivElement>()
  let dialog = $state<Dialog | null>(null)

  function openDialog(d: Dialog): void {
    menuOpen = false
    dialog = d
  }

  function onWindowMouseDown(event: MouseEvent): void {
    if (!menuOpen) return
    const target = event.target as Node
    if (menuEl?.contains(target) || menuBtn?.contains(target)) return
    menuOpen = false
  }

  let creating = $state<ChannelKind | null>(null)
  let createId = ''
  let pending = $state(false)
  let name = $state('')
  let error = $state('')
  let invite = $state(false)
  let inputEl = $state<HTMLInputElement>()

  let workspace = $derived(app.workspaces.find((w) => w.id === app.currentWorkspaceId))
  let textChannels = $derived(
    (workspace?.channels ?? [])
      .filter((c) => c.kind === 'text')
      .sort((a, b) => a.position - b.position),
  )
  let voiceChannels = $derived(
    (workspace?.channels ?? [])
      .filter((c) => c.kind === 'voice')
      .sort((a, b) => a.position - b.position),
  )

  $effect(() => {
    if (creating && inputEl) inputEl.focus()
  })

  function startCreate(kind: ChannelKind): void {
    creating = kind
    createId = crypto.randomUUID()
    name = ''
    error = ''
  }

  function cancel(): void {
    creating = null
    name = ''
    error = ''
  }

  async function submit(): Promise<void> {
    const kind = creating
    const n = name.trim()
    if (!kind || pending) return
    if (!n) {
      cancel()
      return
    }
    pending = true
    try {
      await app.createChannel(n, kind, createId)
      cancel()
    } catch (e) {
      error = e instanceof Error ? e.message : 'Failed to create channel'
    } finally {
      pending = false
    }
  }

  function onKeydown(e: KeyboardEvent): void {
    if (e.key === 'Escape') cancel()
  }

  function membersOf(channelId: Id): VoiceMember[] {
    return app.voice.find((v) => v.channel_id === channelId)?.members ?? []
  }
</script>

<svelte:window onmousedown={onWindowMouseDown} />

<aside class="sidebar">
  <header class="ws-header">
    <h1>{workspace?.name ?? 'Discard'}</h1>
    {#if workspace}
      <div class="head-actions">
        <button class="icon" aria-label="Workspace invite" title="Invite" onclick={() => (invite = true)}>
          ✚
        </button>
        <button
          bind:this={menuBtn}
          class="icon"
          aria-label="Workspace settings"
          title="Workspace settings"
          aria-haspopup="menu"
          aria-expanded={menuOpen}
          onclick={() => (menuOpen = !menuOpen)}
        >
          ⋯
        </button>
      </div>
    {/if}
  </header>

  {#if menuOpen && workspace}
    <div bind:this={menuEl} class="menu" role="menu">
      <button role="menuitem" onclick={() => openDialog({ kind: 'rename-ws', id: workspace.id, name: workspace.name })}>
        Rename workspace
      </button>
      <button role="menuitem" onclick={() => openDialog({ kind: 'leave-ws', id: workspace.id, name: workspace.name })}>
        Leave workspace
      </button>
      <button
        role="menuitem"
        class="danger"
        onclick={() => openDialog({ kind: 'delete-ws', id: workspace.id, name: workspace.name })}
      >
        Delete workspace
      </button>
    </div>
  {/if}

{#if workspace}
  <div class="sections">
    <section>
      <div class="section-head">
        <span>Text channels</span>
        <button class="icon" aria-label="Create text channel" onclick={() => startCreate('text')}>
          +
        </button>
      </div>
      {#if creating === 'text'}
        <form
          class="create"
          onsubmit={(e) => {
            e.preventDefault()
            void submit()
          }}
        >
          <input
            bind:this={inputEl}
            bind:value={name}
            onkeydown={onKeydown}
            maxlength="40"
            disabled={pending}
            placeholder="new-channel"
            aria-label="Text channel name"
          />
        </form>
      {/if}
      {#each textChannels as ch (ch.id)}
        <div class="row">
          <button
            class="chan"
            class:active={ch.id === app.currentChannelId}
            onclick={() => app.selectChannel(ch.id)}
          >
            <span class="glyph">#</span><span class="label">{ch.name}</span>
          </button>
          {@render rowActions(ch.id, ch.name)}
        </div>
      {/each}
    </section>

    <section>
      <div class="section-head">
        <span>Voice channels</span>
        <button class="icon" aria-label="Create voice channel" onclick={() => startCreate('voice')}>
          +
        </button>
      </div>
      {#if creating === 'voice'}
        <form
          class="create"
          onsubmit={(e) => {
            e.preventDefault()
            void submit()
          }}
        >
          <input
            bind:this={inputEl}
            bind:value={name}
            onkeydown={onKeydown}
            maxlength="40"
            disabled={pending}
            placeholder="new-channel"
            aria-label="Voice channel name"
          />
        </form>
      {/if}
      {#each voiceChannels as ch (ch.id)}
        <div class="row">
          <button
            class="chan"
            class:active={ch.id === app.currentChannelId}
            onclick={() => app.selectChannel(ch.id)}
          >
            <span class="glyph">🔊</span><span class="label">{ch.name}</span>
          </button>
          {@render rowActions(ch.id, ch.name)}
        </div>
        {#each membersOf(ch.id) as vm (vm.user.id)}
          <div class="voice-member">
            <span class="dot" style="background: {avatarColor(vm.user.display_name)}"></span>
            <span class="vname">{vm.user.display_name}</span>
            {#if vm.muted}<span class="flag">🔇</span>{/if}
            {#if vm.video}<span class="flag">📹</span>{/if}
          </div>
        {/each}
      {/each}
    </section>
  </div>
{:else}
  <div class="empty-ws">
    <p>No workspace yet.</p>
    <p class="hint">Use the <b>+</b> on the left rail to <b>create</b> a workspace or <b>join with an invite ticket</b>.</p>
  </div>
{/if}

  {#if error}
    <p class="error">{error}</p>
  {/if}

  <footer class="me">
    <span class="avatar" style="background: {avatarColor(app.identity?.display_name ?? '')}">
      {initials(app.identity?.display_name ?? '')}
    </span>
    <div class="who">
      <strong>{app.identity?.display_name}</strong>
      <small>
        <span class="status {net.status}" aria-hidden="true"></span>
        {shortPublicId(app.identity?.public_id ?? '')}
      </small>
    </div>
  </footer>
</aside>

{#snippet rowActions(id: Id, name: string)}
  <span class="row-actions">
    <button
      class="mini"
      aria-label="Rename channel {name}"
      title="Rename"
      onclick={() => openDialog({ kind: 'rename-ch', id, name })}
    >
      ✎
    </button>
    <button
      class="mini"
      aria-label="Delete channel {name}"
      title="Delete"
      onclick={() => openDialog({ kind: 'delete-ch', id, name })}
    >
      ✕
    </button>
  </span>
{/snippet}

{#if invite}
  <InviteDialog onclose={() => (invite = false)} />
{/if}

{#if dialog}
  {@const d = dialog}
  {#if d.kind === 'rename-ws'}
    <ActionDialog
      title="Rename workspace"
      value={d.name}
      confirmLabel="Save"
      onconfirm={(v) => app.renameWorkspace(d.id, v)}
      onclose={() => (dialog = null)}
    />
  {:else if d.kind === 'leave-ws'}
    <ActionDialog
      title="Leave workspace"
      message={`Leave "${d.name}"? It is removed from this device only. You can rejoin with an invite ticket.`}
      confirmLabel="Leave workspace"
      danger
      onconfirm={() => app.leaveWorkspace(d.id)}
      onclose={() => (dialog = null)}
    />
  {:else if d.kind === 'delete-ws'}
    <ActionDialog
      title="Delete workspace"
      message={`Delete "${d.name}" for every member? Its channels go with it. This cannot be undone.`}
      confirmLabel="Delete workspace"
      danger
      onconfirm={() => app.deleteWorkspace(d.id)}
      onclose={() => (dialog = null)}
    />
  {:else if d.kind === 'rename-ch'}
    <ActionDialog
      title="Rename channel"
      value={d.name}
      confirmLabel="Save"
      onconfirm={(v) => app.renameChannel(d.id, v)}
      onclose={() => (dialog = null)}
    />
  {:else}
    <ActionDialog
      title="Delete channel"
      message={`Delete "${d.name}" for every member? This cannot be undone.`}
      confirmLabel="Delete channel"
      danger
      onconfirm={() => app.deleteChannel(d.id)}
      onclose={() => (dialog = null)}
    />
  {/if}
{/if}

<style>
  .sidebar {
    position: relative;
    display: flex;
    flex-direction: column;
    background: var(--bg-1);
    min-width: 0;
  }

  .ws-header {
    height: 48px;
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 8px;
    padding: 0 8px 0 14px;
    border-bottom: 1px solid var(--bg-0);
    box-shadow: 0 1px 0 rgba(0, 0, 0, 0.2);
  }

  .head-actions {
    display: flex;
    align-items: center;
    gap: 2px;
    flex: none;
  }

  .menu {
    position: absolute;
    top: 52px;
    right: 8px;
    z-index: 20;
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

  .menu button.danger {
    color: var(--danger);
  }

  .row {
    position: relative;
    display: flex;
    align-items: center;
  }

  .row-actions {
    position: absolute;
    right: 4px;
    display: none;
    gap: 2px;
  }

  .row:hover .row-actions,
  .row:focus-within .row-actions {
    display: flex;
  }

  .mini {
    background: transparent;
    border: none;
    color: var(--text-2);
    font-size: 12px;
    line-height: 1;
    padding: 4px;
    border-radius: var(--radius);
    cursor: pointer;
  }

  .mini:hover {
    color: var(--text-0);
    background: var(--bg-3);
  }

  .empty-ws {
    flex: 1;
    padding: 18px 16px;
    color: var(--text-1);
    font-size: 13px;
  }

  .empty-ws p {
    margin: 0 0 8px;
  }

  .empty-ws .hint {
    color: var(--text-2);
    font-size: 12px;
    line-height: 1.5;
  }

  .ws-header h1 {
    margin: 0;
    font-size: 15px;
    color: var(--text-0);
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }

  .sections {
    flex: 1;
    overflow-y: auto;
    padding: 12px 8px;
    display: flex;
    flex-direction: column;
    gap: 16px;
  }

  .section-head {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 0 8px 4px;
    font-size: 11px;
    text-transform: uppercase;
    letter-spacing: 0.5px;
    color: var(--text-2);
  }

  .icon {
    background: transparent;
    border: none;
    color: var(--text-2);
    font-size: 16px;
    cursor: pointer;
    padding: 0 4px;
    border-radius: var(--radius);
    line-height: 1;
  }

  .icon:hover {
    color: var(--text-0);
  }

  .create {
    padding: 2px 4px 4px;
  }

  .create input {
    width: 100%;
    box-sizing: border-box;
    background: var(--bg-0);
    border: 1px solid var(--accent);
    border-radius: var(--radius);
    color: var(--text-0);
    font: inherit;
    font-size: 13px;
    padding: 6px 8px;
    outline: none;
  }

  .chan {
    width: 100%;
    display: flex;
    align-items: center;
    gap: 6px;
    background: transparent;
    border: none;
    color: var(--text-2);
    font: inherit;
    font-size: 14px;
    text-align: left;
    padding: 6px 8px;
    border-radius: var(--radius);
    cursor: pointer;
  }

  .chan:hover {
    background: var(--bg-2);
    color: var(--text-1);
  }

  .chan.active {
    background: var(--bg-3);
    color: var(--text-0);
  }

  .glyph {
    font-size: 14px;
    opacity: 0.8;
    width: 16px;
    text-align: center;
  }

  .label {
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }

  .voice-member {
    display: flex;
    align-items: center;
    gap: 6px;
    padding: 3px 8px 3px 24px;
    font-size: 13px;
    color: var(--text-2);
  }

  .dot {
    width: 16px;
    height: 16px;
    border-radius: 50%;
    flex: none;
  }

  .vname {
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }

  .flag {
    font-size: 11px;
    flex: none;
  }

  .error {
    margin: 0;
    padding: 6px 12px;
    color: var(--danger);
    font-size: 12px;
  }

  .me {
    display: flex;
    align-items: center;
    gap: 8px;
    padding: 8px 10px;
    background: var(--bg-0);
  }

  .avatar {
    width: 32px;
    height: 32px;
    border-radius: 50%;
    display: grid;
    place-items: center;
    color: #fff;
    font-size: 12px;
    font-weight: 600;
    flex: none;
  }

  .who {
    display: flex;
    flex-direction: column;
    min-width: 0;
    flex: 1;
  }

  .who strong {
    font-size: 13px;
    color: var(--text-0);
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }

  .who small {
    display: flex;
    align-items: center;
    gap: 5px;
    font-size: 11px;
    color: var(--text-2);
  }

  .status {
    width: 8px;
    height: 8px;
    border-radius: 50%;
    background: var(--text-2);
    flex: none;
  }

  .status.online {
    background: var(--ok);
  }

  .status.connecting {
    background: #d9a53b;
  }

  .status.offline {
    background: var(--danger);
  }
</style>
