<script lang="ts">
  import { app } from '../lib/state.svelte'
  import { ws } from '../lib/ws'
  import { uploadFile } from '../lib/api'
  import { formatBytes } from '../lib/format'
  import type { Attachment, Id } from '../lib/protocol'

  const MAX_HEIGHT = 200

  let text = $state('')
  let pending = $state<Attachment[]>([])
  let uploading = $state(false)
  let dragging = $state(false)
  let error = $state('')
  let ta: HTMLTextAreaElement | undefined
  let lastTyping = 0

  let channelName = $derived.by(() => {
    const id = app.currentChannelId
    if (id == null) return ''
    for (const w of app.workspaces) {
      const c = w.channels.find((x) => x.id === id)
      if (c) return c.name
    }
    return ''
  })

  let canSend = $derived(text.trim().length > 0 || pending.length > 0)

  function grow(): void {
    if (!ta) return
    ta.style.height = 'auto'
    ta.style.height = `${Math.min(ta.scrollHeight, MAX_HEIGHT)}px`
  }

  $effect(() => {
    void text
    grow()
  })

  function pingTyping(): void {
    const channelId = app.currentChannelId
    if (channelId == null) return
    const now = Date.now()
    if (now - lastTyping > 3000) {
      lastTyping = now
      ws.send({ op: 'typing', channel_id: channelId })
    }
  }

  async function send(): Promise<void> {
    const content = text.trim()
    if (!content && pending.length === 0) return
    const ids = pending.map((a) => a.id)
    text = ''
    pending = []
    error = ''
    grow()
    try {
      await app.sendMessage(content, ids)
    } catch (e) {
      error = e instanceof Error ? e.message : 'Falha ao enviar'
    }
  }

  function onKeydown(e: KeyboardEvent): void {
    if (e.key === 'Enter' && !e.shiftKey && !e.isComposing) {
      e.preventDefault()
      void send()
    }
  }

  async function addFiles(files: FileList | File[]): Promise<void> {
    const arr = Array.from(files)
    if (arr.length === 0) return
    uploading = true
    error = ''
    for (const f of arr) {
      try {
        const a = await uploadFile(f)
        pending = [...pending, a]
      } catch (e) {
        error = e instanceof Error ? e.message : 'Falha no upload'
      }
    }
    uploading = false
  }

  async function onFile(e: Event): Promise<void> {
    const input = e.currentTarget as HTMLInputElement
    if (input.files) await addFiles(input.files)
    input.value = ''
  }

  function onDrop(e: DragEvent): void {
    e.preventDefault()
    dragging = false
    if (e.dataTransfer?.files && e.dataTransfer.files.length > 0) {
      void addFiles(e.dataTransfer.files)
    }
  }

  function onDragOver(e: DragEvent): void {
    e.preventDefault()
    dragging = true
  }

  function remove(id: Id): void {
    pending = pending.filter((a) => a.id !== id)
  }
</script>

<div
  class="composer"
  class:dragging
  role="group"
  aria-label="Enviar mensagem"
  ondragover={onDragOver}
  ondragleave={() => (dragging = false)}
  ondrop={onDrop}
>
  {#if dragging}
    <div class="drop">Solte para anexar</div>
  {/if}

  {#if pending.length > 0}
    <div class="pending">
      {#each pending as a (a.id)}
        <span class="chip">
          <span class="cname">{a.filename}</span>
          <small>{formatBytes(a.size)}</small>
          <button type="button" aria-label="Remover anexo" onclick={() => remove(a.id)}>×</button>
        </span>
      {/each}
    </div>
  {/if}

  {#if error}
    <p class="error">{error}</p>
  {/if}

  <div class="row">
    <label class="attach">
      <span aria-hidden="true">＋</span>
      <span class="sr">Anexar arquivo</span>
      <input type="file" multiple onchange={onFile} />
    </label>

    <textarea
      bind:this={ta}
      bind:value={text}
      onkeydown={onKeydown}
      oninput={pingTyping}
      placeholder={`Mensagem em #${channelName}`}
      rows="1"
      aria-label="Mensagem"
    ></textarea>

    <button class="send" aria-label="Enviar" disabled={!canSend} onclick={() => void send()}>
      ➤
    </button>
  </div>

  {#if uploading}
    <p class="hint">Enviando arquivo…</p>
  {/if}
</div>

<style>
  .composer {
    position: relative;
    flex: none;
    padding: 0 16px 20px;
  }

  .composer.dragging::after {
    content: '';
    position: absolute;
    inset: 0 8px 12px;
    border: 2px dashed var(--accent);
    border-radius: 8px;
    pointer-events: none;
  }

  .drop {
    position: absolute;
    inset: 0 8px 12px;
    display: grid;
    place-items: center;
    background: rgba(88, 101, 242, 0.12);
    border-radius: 8px;
    color: var(--text-0);
    font-size: 14px;
    pointer-events: none;
    z-index: 2;
  }

  .pending {
    display: flex;
    flex-wrap: wrap;
    gap: 6px;
    padding: 8px 10px;
    margin-bottom: -6px;
    background: var(--bg-1);
    border-radius: 8px 8px 0 0;
  }

  .chip {
    display: inline-flex;
    align-items: center;
    gap: 6px;
    background: var(--bg-3);
    border-radius: var(--radius);
    padding: 3px 6px 3px 8px;
    font-size: 12px;
    color: var(--text-1);
    max-width: 240px;
  }

  .cname {
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }

  .chip small {
    color: var(--text-2);
    flex: none;
  }

  .chip button {
    background: transparent;
    border: none;
    color: var(--text-2);
    cursor: pointer;
    font-size: 14px;
    line-height: 1;
    padding: 0 2px;
  }

  .chip button:hover {
    color: var(--danger);
  }

  .row {
    display: flex;
    align-items: flex-end;
    gap: 8px;
    background: var(--bg-3);
    border-radius: 8px;
    padding: 6px 8px;
  }

  .attach {
    flex: none;
    display: grid;
    place-items: center;
    width: 32px;
    height: 32px;
    border-radius: 50%;
    background: var(--text-2);
    color: var(--bg-2);
    cursor: pointer;
    font-size: 18px;
    font-weight: 600;
    line-height: 1;
  }

  .attach:hover {
    background: var(--text-0);
  }

  .attach input {
    display: none;
  }

  .sr {
    position: absolute;
    width: 1px;
    height: 1px;
    overflow: hidden;
    clip: rect(0 0 0 0);
    white-space: nowrap;
  }

  textarea {
    flex: 1;
    resize: none;
    border: none;
    background: transparent;
    color: var(--text-0);
    font: inherit;
    font-size: 14px;
    line-height: 1.4;
    padding: 7px 2px;
    max-height: 200px;
    outline: none;
    overflow-y: auto;
  }

  textarea::placeholder {
    color: var(--text-2);
  }

  .send {
    flex: none;
    width: 32px;
    height: 32px;
    border: none;
    border-radius: 50%;
    background: var(--accent);
    color: #fff;
    cursor: pointer;
    font-size: 14px;
    display: grid;
    place-items: center;
  }

  .send:disabled {
    background: var(--bg-1);
    color: var(--text-2);
    cursor: default;
  }

  .error {
    margin: 6px 2px 0;
    color: var(--danger);
    font-size: 12px;
  }

  .hint {
    margin: 6px 2px 0;
    color: var(--text-2);
    font-size: 12px;
  }
</style>
