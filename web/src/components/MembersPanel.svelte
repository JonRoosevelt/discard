<script lang="ts">
  import { app } from '../lib/state.svelte'
  import { shortPublicId } from '../lib/identity'
  import { avatarColor, initials } from '../lib/format'

  let members = $derived(app.members)
</script>

<aside class="members" aria-label="Membros">
  <h2>Membros — {members.length}</h2>
  <div class="list">
    {#each members as m (m.id)}
      <div class="member">
        <span class="avatar" style="background: {avatarColor(m.display_name)}">
          {initials(m.display_name)}
        </span>
        <div class="who">
          <strong>{m.display_name}</strong>
          <small>{shortPublicId(m.id)}</small>
        </div>
      </div>
    {/each}
  </div>
</aside>

<style>
  .members {
    width: 220px;
    flex: none;
    background: var(--bg-1);
    display: flex;
    flex-direction: column;
    border-left: 1px solid var(--bg-0);
  }

  h2 {
    margin: 0;
    padding: 14px 12px 8px;
    font-size: 11px;
    text-transform: uppercase;
    letter-spacing: 0.5px;
    color: var(--text-2);
  }

  .list {
    flex: 1;
    overflow-y: auto;
    padding: 0 8px 12px;
  }

  .member {
    display: flex;
    align-items: center;
    gap: 8px;
    padding: 5px 6px;
    border-radius: var(--radius);
  }

  .member:hover {
    background: var(--bg-2);
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
  }

  .who strong {
    font-size: 13px;
    color: var(--text-0);
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }

  .who small {
    font-size: 11px;
    color: var(--text-2);
  }
</style>
