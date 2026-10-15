<script lang="ts">
  import { app, typing } from '../lib/state.svelte'
  import { ws } from '../lib/ws'
  import { avatarColor, initials } from '../lib/format'
  import MessageList from './MessageList.svelte'
  import Composer from './Composer.svelte'
  import MembersPanel from './MembersPanel.svelte'

  let showMembers = $state(false)

  let channel = $derived.by(() => {
    const id = app.currentChannelId
    if (id == null) return null
    for (const w of app.workspaces) {
      const c = w.channels.find((x) => x.id === id)
      if (c) return c
    }
    return null
  })

  let voiceMembers = $derived(
    channel ? (app.voice.find((v) => v.channel_id === channel?.id)?.members ?? []) : [],
  )

  let selfInVoice = $derived(voiceMembers.some((m) => m.user.id === app.identity?.public_id))

  let typers = $derived(
    app.currentChannelId != null ? (typing[app.currentChannelId] ?? []) : [],
  )

  function join(): void {
    if (channel) ws.send({ op: 'voice_join', channel_id: channel.id, muted: false, video: false })
  }

  function leave(): void {
    if (channel) ws.send({ op: 'voice_leave', channel_id: channel.id })
  }
</script>

<main class="chat">
  <header class="head">
    <div class="title">
      {#if channel}
        <span class="glyph">{channel.kind === 'text' ? '#' : '🔊'}</span>
        <strong>{channel.name}</strong>
        {#if channel.kind === 'text'}
          <span class="ephemeral" title="Messages disappear after 24h">24h</span>
        {/if}
      {/if}
    </div>
    <div class="actions">
      {#if channel?.kind === 'voice'}
        {#if selfInVoice}
          <button class="primary danger" onclick={leave}>Leave channel</button>
        {:else}
          <button class="primary" onclick={join}>Join voice channel</button>
        {/if}
      {/if}
      <button
        class="toggle"
        class:on={showMembers}
        aria-label="Show members"
        aria-pressed={showMembers}
        onclick={() => (showMembers = !showMembers)}
      >
        👥 {app.members.length}
      </button>
    </div>
  </header>

  <div class="body">
    <div class="center">
      {#if !channel}
        <div class="empty">
          <h2>No channel selected</h2>
          <p>Choose a channel in the sidebar to chat.</p>
        </div>
      {:else if channel.kind === 'text'}
        <MessageList />
        <div class="typing">
          {#if typers.length > 0}
            <span class="dots"><i></i><i></i><i></i></span>
            <span>
              {typers.map((u) => u.display_name).join(', ')}
              {typers.length === 1 ? 'is typing…' : 'are typing…'}
            </span>
          {/if}
        </div>
        <Composer />
      {:else}
        <div class="voice">
          <h2>{channel.name}</h2>
          {#if voiceMembers.length === 0}
            <p>Nobody here yet.</p>
          {:else}
            <div class="grid">
              {#each voiceMembers as vm (vm.user.id)}
                <div class="card">
                  <span class="avatar" style="background: {avatarColor(vm.user.display_name)}">
                    {initials(vm.user.display_name)}
                  </span>
                  <span class="name">{vm.user.display_name}</span>
                  <span class="flags">
                    {#if vm.muted}🔇{/if}
                    {#if vm.deafened}🔈{/if}
                    {#if vm.video}📹{/if}
                    {#if vm.screen}🖥️{/if}
                  </span>
                </div>
              {/each}
            </div>
          {/if}
          {#if selfInVoice}
            <button class="primary danger" onclick={leave}>Leave channel</button>
          {:else}
            <button class="primary" onclick={join}>Join voice channel</button>
          {/if}
        </div>
      {/if}
    </div>

    {#if showMembers}
      <MembersPanel />
    {/if}
  </div>
</main>

<style>
  .chat {
    display: flex;
    flex-direction: column;
    min-width: 0;
    background: var(--bg-2);
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
    box-shadow: 0 1px 0 rgba(0, 0, 0, 0.15);
  }

  .title {
    display: flex;
    align-items: center;
    gap: 8px;
    color: var(--text-0);
    min-width: 0;
  }

  .title .glyph {
    color: var(--text-2);
  }

  .title strong {
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }

  .ephemeral {
    font-size: 10px;
    font-weight: 600;
    letter-spacing: 0.4px;
    color: var(--text-2);
    border: 1px solid var(--bg-3);
    border-radius: 10px;
    padding: 1px 7px;
    text-transform: uppercase;
  }

  .actions {
    display: flex;
    align-items: center;
    gap: 8px;
  }

  .primary {
    background: var(--accent);
    color: #fff;
    border: none;
    border-radius: var(--radius);
    font: inherit;
    font-size: 13px;
    padding: 6px 12px;
    cursor: pointer;
    white-space: nowrap;
  }

  .primary:hover {
    background: var(--accent-hover);
  }

  .primary.danger {
    background: var(--danger);
  }

  .toggle {
    background: transparent;
    border: none;
    color: var(--text-2);
    font: inherit;
    font-size: 13px;
    cursor: pointer;
    padding: 6px 8px;
    border-radius: var(--radius);
  }

  .toggle:hover,
  .toggle.on {
    background: var(--bg-3);
    color: var(--text-0);
  }

  .body {
    flex: 1;
    display: flex;
    min-height: 0;
  }

  .center {
    flex: 1;
    display: flex;
    flex-direction: column;
    min-width: 0;
    min-height: 0;
  }

  .empty {
    flex: 1;
    display: grid;
    place-content: center;
    text-align: center;
    color: var(--text-2);
  }

  .empty h2 {
    margin: 0 0 6px;
    color: var(--text-1);
    font-size: 18px;
  }

  .empty p {
    margin: 0;
    font-size: 14px;
  }

  .typing {
    height: 20px;
    flex: none;
    padding: 0 16px;
    font-size: 12px;
    color: var(--text-2);
    display: flex;
    align-items: center;
    gap: 6px;
  }

  .dots {
    display: inline-flex;
    gap: 2px;
  }

  .dots i {
    width: 4px;
    height: 4px;
    border-radius: 50%;
    background: var(--text-2);
    animation: blink 1.2s infinite ease-in-out;
  }

  .dots i:nth-child(2) {
    animation-delay: 0.2s;
  }

  .dots i:nth-child(3) {
    animation-delay: 0.4s;
  }

  @keyframes blink {
    0%,
    60%,
    100% {
      opacity: 0.25;
    }
    30% {
      opacity: 1;
    }
  }

  .voice {
    flex: 1;
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    gap: 16px;
    color: var(--text-2);
    text-align: center;
    padding: 24px;
  }

  .voice h2 {
    margin: 0;
    color: var(--text-0);
  }

  .grid {
    display: flex;
    flex-wrap: wrap;
    gap: 12px;
    justify-content: center;
    max-width: 520px;
  }

  .card {
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 6px;
    width: 96px;
    padding: 12px 8px;
    background: var(--bg-1);
    border-radius: 8px;
  }

  .card .avatar {
    width: 44px;
    height: 44px;
    border-radius: 50%;
    display: grid;
    place-items: center;
    color: #fff;
    font-weight: 600;
  }

  .card .name {
    font-size: 12px;
    color: var(--text-1);
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
    max-width: 100%;
  }

  .card .flags {
    font-size: 11px;
    min-height: 14px;
  }
</style>
