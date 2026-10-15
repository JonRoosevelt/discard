<script lang="ts">
  import { app } from '../lib/state.svelte'
  import { avatarColor, formatBytes, formatDay, formatTime, initials, isImage } from '../lib/format'
  import type { Attachment, Id, Message } from '../lib/protocol'

  type Row =
    | { kind: 'day'; key: string; label: string }
    | { kind: 'msg'; key: string; message: Message; grouped: boolean }

  let scroller: HTMLDivElement | undefined
  let stick = $state(true)

  let rows = $derived.by(() => {
    const out: Row[] = []
    let lastDay = ''
    let lastAuthor: Id | null = null
    let lastTime = 0
    for (const m of app.messages) {
      const day = formatDay(m.created_at)
      if (day !== lastDay) {
        out.push({ kind: 'day', key: `day-${m.id}`, label: day })
        lastDay = day
        lastAuthor = null
      }
      const t = new Date(m.created_at).getTime()
      const grouped = m.author.id === lastAuthor && t - lastTime < 5 * 60 * 1000
      out.push({ kind: 'msg', key: `msg-${m.id}`, message: m, grouped })
      lastAuthor = m.author.id
      lastTime = t
    }
    return out
  })

  let channelName = $derived.by(() => {
    const id = app.currentChannelId
    if (id == null) return ''
    for (const w of app.workspaces) {
      const c = w.channels.find((x) => x.id === id)
      if (c) return c.name
    }
    return ''
  })

  function onScroll(): void {
    if (!scroller) return
    stick = scroller.scrollHeight - scroller.scrollTop - scroller.clientHeight < 120
  }

  $effect(() => {
    app.messages.length
    if (scroller && stick) scroller.scrollTop = scroller.scrollHeight
  })

  $effect(() => {
    app.currentChannelId
    stick = true
    if (scroller) scroller.scrollTop = scroller.scrollHeight
  })
</script>

{#snippet attachments(list: Attachment[])}
  {#if list.length > 0}
    <div class="attachments">
      {#each list as a (a.id)}
        {#if isImage(a.content_type)}
          <a class="image" href={a.url} target="_blank" rel="noreferrer noopener">
            <img src={a.url} alt={a.filename} loading="lazy" />
          </a>
        {:else}
          <a class="file" href={a.url} download={a.filename}>
            <span class="ficon">📄</span>
            <span class="fmeta">
              <strong>{a.filename}</strong>
              <small>{formatBytes(a.size)}</small>
            </span>
          </a>
        {/if}
      {/each}
    </div>
  {/if}
{/snippet}

<div class="list" bind:this={scroller} onscroll={onScroll} role="log" aria-live="polite">
  {#if app.messages.length === 0}
    <div class="welcome">
      <h2>Bem-vindo a #{channelName}</h2>
      <p>Este é o começo do canal. Diga olá!</p>
      <p class="ephemeral">As mensagens desaparecem em 24h.</p>
    </div>
  {/if}

  {#each rows as row (row.key)}
    {#if row.kind === 'day'}
      <div class="day"><span>{row.label}</span></div>
    {:else if row.grouped}
      <div class="msg grouped">
        <span class="time">{formatTime(row.message.created_at)}</span>
        <div class="content">
          {#if row.message.content}<p class="text">{row.message.content}</p>{/if}
          {@render attachments(row.message.attachments)}
        </div>
      </div>
    {:else}
      <div class="msg">
        <span class="avatar" style="background: {avatarColor(row.message.author.display_name)}">
          {initials(row.message.author.display_name)}
        </span>
        <div class="content">
          <div class="meta">
            <strong>{row.message.author.display_name}</strong>
            <span>{formatTime(row.message.created_at)}</span>
          </div>
          {#if row.message.content}<p class="text">{row.message.content}</p>{/if}
          {@render attachments(row.message.attachments)}
        </div>
      </div>
    {/if}
  {/each}
</div>

<style>
  .list {
    flex: 1;
    overflow-y: auto;
    padding: 16px 0 8px;
    display: flex;
    flex-direction: column;
    min-height: 0;
  }

  .welcome {
    padding: 32px 16px;
    color: var(--text-2);
  }

  .welcome h2 {
    margin: 0 0 6px;
    color: var(--text-0);
    font-size: 22px;
  }

  .welcome p {
    margin: 0;
    font-size: 14px;
  }

  .welcome .ephemeral {
    margin-top: 8px;
    font-size: 12px;
    color: var(--text-2);
  }

  .day {
    display: flex;
    align-items: center;
    gap: 10px;
    padding: 12px 16px 6px;
    color: var(--text-2);
    font-size: 11px;
    font-weight: 600;
    text-transform: uppercase;
    letter-spacing: 0.4px;
  }

  .day::before,
  .day::after {
    content: '';
    flex: 1;
    height: 1px;
    background: var(--bg-3);
  }

  .msg {
    display: flex;
    gap: 12px;
    padding: 2px 16px;
  }

  .msg:hover {
    background: rgba(0, 0, 0, 0.06);
  }

  .msg.grouped {
    padding-top: 0;
    padding-bottom: 0;
  }

  .avatar {
    width: 36px;
    height: 36px;
    border-radius: 50%;
    display: grid;
    place-items: center;
    color: #fff;
    font-size: 13px;
    font-weight: 600;
    flex: none;
    margin-top: 2px;
  }

  .time {
    width: 36px;
    flex: none;
    text-align: center;
    font-size: 10px;
    color: var(--text-2);
    opacity: 0;
    padding-top: 4px;
  }

  .msg.grouped:hover .time {
    opacity: 1;
  }

  .content {
    min-width: 0;
    flex: 1;
    padding-bottom: 4px;
  }

  .meta {
    display: flex;
    align-items: baseline;
    gap: 8px;
  }

  .meta strong {
    color: var(--text-0);
    font-size: 14px;
  }

  .meta span {
    color: var(--text-2);
    font-size: 11px;
  }

  .text {
    margin: 1px 0 0;
    color: var(--text-1);
    font-size: 14px;
    line-height: 1.4;
    white-space: pre-wrap;
    word-break: break-word;
  }

  .attachments {
    display: flex;
    flex-direction: column;
    gap: 6px;
    margin-top: 6px;
    align-items: flex-start;
  }

  .image img {
    max-width: min(420px, 100%);
    max-height: 320px;
    border-radius: 6px;
    display: block;
  }

  .file {
    display: flex;
    align-items: center;
    gap: 10px;
    padding: 8px 12px;
    background: var(--bg-1);
    border-radius: 6px;
    text-decoration: none;
    color: var(--text-1);
    max-width: 320px;
  }

  .file:hover {
    background: var(--bg-3);
  }

  .ficon {
    font-size: 20px;
  }

  .fmeta {
    display: flex;
    flex-direction: column;
    min-width: 0;
  }

  .fmeta strong {
    font-size: 13px;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }

  .fmeta small {
    font-size: 11px;
    color: var(--text-2);
  }
</style>
