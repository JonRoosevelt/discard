<script lang="ts">
  import { onMount } from 'svelte'
  import { app } from '../lib/state.svelte'
  import { hasSession } from '../lib/api'
  import Login from './Login.svelte'
  import WorkspaceRail from './WorkspaceRail.svelte'
  import ChannelSidebar from './ChannelSidebar.svelte'
  import ChatView from './ChatView.svelte'
  import VoicePanel from './VoicePanel.svelte'

  onMount(() => {
    if (app.identity == null) void app.bootstrap()
  })

  let selfId = $derived(app.identity?.public_id ?? '')
  let inCall = $derived(app.voice.some((p) => p.members.some((m) => m.user.id === selfId)))
</script>

{#if !hasSession()}
  <div class="gate">
    <div class="gate-card">
      <h1>Discard</h1>
      <p>Open this interface through the local node.</p>
      <p class="hint">
        The node starts the UI at <code>http://127.0.0.1:&lt;port&gt;/?k=&lt;token&gt;</code>.
      </p>
    </div>
  </div>
{:else if !app.booted}
  <div class="gate"><p class="hint">Connecting to the local node…</p></div>
{:else if app.entered}
  <div class="shell" class:incall={inCall}>
    <WorkspaceRail />
    <ChannelSidebar />
    <ChatView />
    {#if inCall}<VoicePanel />{/if}
  </div>
{:else}
  <Login />
{/if}

<style>
  .shell {
    display: grid;
    grid-template-columns: 72px 240px minmax(0, 1fr);
    height: 100dvh;
    overflow: hidden;
    background: var(--bg-2);
  }

  .shell.incall {
    grid-template-columns: 72px 240px minmax(0, 1fr) 320px;
  }

  @media (max-width: 1100px) {
    .shell.incall {
      grid-template-columns: 72px 220px minmax(0, 1fr) 280px;
    }
  }

  @media (max-width: 880px) {
    .shell,
    .shell.incall {
      grid-template-columns: 72px 200px minmax(0, 1fr);
    }
    .shell :global(.panel) {
      display: none;
    }
  }

  @media (max-width: 520px) {
    .shell,
    .shell.incall {
      grid-template-columns: 64px minmax(0, 1fr);
    }
    .shell :global(.sidebar) {
      display: none;
    }
  }

  .gate {
    display: grid;
    place-items: center;
    min-height: 100vh;
    background: var(--bg-0);
  }

  .gate-card {
    width: 360px;
    padding: 28px 24px;
    background: var(--bg-2);
    border-radius: 8px;
    box-shadow: 0 8px 24px rgba(0, 0, 0, 0.4);
    text-align: center;
  }

  .gate-card h1 {
    margin: 0 0 8px;
    font-size: 24px;
    color: var(--text-0);
    letter-spacing: -0.5px;
  }

  .gate-card p {
    margin: 0;
    color: var(--text-1);
    font-size: 14px;
  }

  .hint {
    margin-top: 10px;
    color: var(--text-2);
    font-size: 12px;
  }

  .hint code {
    font-family: ui-monospace, SFMono-Regular, Menlo, monospace;
    font-size: 11px;
  }
</style>
