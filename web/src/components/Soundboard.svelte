<script lang="ts">
  import { onDestroy } from 'svelte'
  import { app } from '../lib/state.svelte'
  import { ws } from '../lib/ws'
  import { deleteSound, updateSound, uploadSound } from '../lib/api'
  import type { ServerEvent, Sound } from '../lib/protocol'

  const GAIN = 0.85

  let volume = $state(0.8)
  let uploading = $state(false)
  let error = $state('')
  let newEmoji = $state('🔊')
  let newName = $state('')
  let fileEl = $state<HTMLInputElement>()
  let editing = $state<Sound | null>(null)
  let editName = $state('')
  let editEmoji = $state('')

  const mine = (sound: Sound): boolean => !sound.builtin && sound.owner === app.identity?.public_id

  let myChannelId = $derived.by(() => {
    const me = app.identity?.public_id
    if (!me) return null
    return app.voice.find((v) => v.members.some((m) => m.user.id === me))?.channel_id ?? null
  })

  let inVoice = $derived(myChannelId != null)

  function play(sound: Sound): void {
    const audio = new Audio(sound.url)
    audio.volume = Math.min(1, Math.max(0, volume * GAIN))
    void audio.play().catch(() => {})
  }

  function trigger(sound: Sound): void {
    if (myChannelId == null) return
    play(sound)
    ws.send({ op: 'sound_play', channel_id: myChannelId, sound_id: sound.id })
  }

  const off = ws.on((ev: ServerEvent) => {
    if (ev.op !== 'sound_play') return
    if (ev.channel_id !== myChannelId) return
    if (ev.by === app.identity?.public_id) return
    const sound = app.sounds.find((s) => s.id === ev.sound_id)
    if (sound) play(sound)
  })

  onDestroy(off)

  function startEdit(sound: Sound): void {
    editing = sound
    editName = sound.name
    editEmoji = sound.emoji
    error = ''
  }

  async function saveEdit(): Promise<void> {
    if (!editing) return
    try {
      await updateSound(editing.id, editName.trim(), editEmoji.trim() || '🔊')
      await app.reloadSounds()
      editing = null
    } catch (err) {
      error = err instanceof Error ? err.message : 'Failed to update sound'
    }
  }

  async function remove(sound: Sound): Promise<void> {
    if (!confirm(`Delete "${sound.name}" for everyone?`)) return
    try {
      await deleteSound(sound.id)
      await app.reloadSounds()
    } catch (err) {
      error = err instanceof Error ? err.message : 'Failed to delete sound'
    }
  }

  function pickFile(): void {
    if (uploading) return
    fileEl?.click()
  }

  async function onFile(e: Event): Promise<void> {
    const input = e.currentTarget as HTMLInputElement
    const file = input.files?.[0]
    if (!file) return
    const name = newName.trim() || file.name.replace(/\.[^.]+$/, '')
    const emoji = newEmoji.trim() || '🔊'
    uploading = true
    error = ''
    try {
      await uploadSound(file, name, emoji)
      await app.reloadSounds()
      newName = ''
    } catch (err) {
      error = err instanceof Error ? err.message : 'Failed to upload sound'
    } finally {
      uploading = false
      input.value = ''
    }
  }
</script>

<section class="board">
  <header class="head">
    <span class="title">Soundboard</span>
    <label class="vol" title="Volume">
      <span aria-hidden="true">🔈</span>
      <input
        type="range"
        min="0"
        max="1"
        step="0.05"
        bind:value={volume}
        aria-label="Soundboard volume"
      />
      <span aria-hidden="true">🔊</span>
    </label>
  </header>

  {#if !inVoice}
    <p class="hint">Join a voice channel to play sounds.</p>
  {/if}

  <div class="grid">
    {#each app.sounds as sound (sound.id)}
      <div class="cell">
        <button
          class="sound"
          type="button"
          title={sound.name}
          aria-label={sound.name}
          disabled={!inVoice}
          onclick={() => trigger(sound)}
        >
          {sound.emoji || '🔊'}
        </button>
        {#if mine(sound)}
          <div class="actions">
            <button
              type="button"
              title="Edit sound"
              aria-label={`Edit ${sound.name}`}
              onclick={() => startEdit(sound)}>✎</button
            >
            <button
              type="button"
              title="Delete sound"
              aria-label={`Delete ${sound.name}`}
              onclick={() => remove(sound)}>✕</button
            >
          </div>
        {/if}
      </div>
    {:else}
      <p class="empty">No sounds yet.</p>
    {/each}
  </div>

  {#if editing}
    <form class="upload" onsubmit={(e) => { e.preventDefault(); void saveEdit() }}>
      <input class="emoji-in" bind:value={editEmoji} maxlength="4" aria-label="Edit sound emoji" />
      <input class="name-in" bind:value={editName} maxlength="24" aria-label="Edit sound name" />
      <button class="add" type="submit" title="Save sound" aria-label="Save sound">✓</button>
      <button class="add cancel" type="button" title="Cancel" aria-label="Cancel editing" onclick={() => (editing = null)}>✕</button>
    </form>
  {/if}

  <div class="upload">
    <input class="emoji-in" bind:value={newEmoji} maxlength="4" aria-label="Sound emoji" />
    <input
      class="name-in"
      bind:value={newName}
      maxlength="24"
      placeholder="sound name"
      aria-label="Sound name"
    />
    <button
      class="add"
      type="button"
      onclick={pickFile}
      disabled={uploading}
      title="Upload sound"
      aria-label="Upload sound"
    >
      {uploading ? '…' : '＋'}
    </button>
    <input
      bind:this={fileEl}
      class="file"
      type="file"
      accept="audio/*"
      onchange={onFile}
      tabindex="-1"
      aria-hidden="true"
    />
  </div>

  {#if error}
    <p class="error">{error}</p>
  {/if}
</section>

<style>
  .board {
    display: flex;
    flex-direction: column;
    gap: 8px;
    padding: 10px;
    background: var(--bg-1);
    border-radius: 8px;
    min-height: 0;
  }

  .head {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 8px;
  }

  .title {
    font-size: 11px;
    text-transform: uppercase;
    letter-spacing: 0.5px;
    color: var(--text-2);
  }

  .vol {
    display: flex;
    align-items: center;
    gap: 4px;
    font-size: 10px;
    color: var(--text-2);
  }

  .vol input[type='range'] {
    width: 72px;
    accent-color: var(--accent);
    cursor: pointer;
  }

  .grid {
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(48px, 1fr));
    gap: 6px;
    max-height: 200px;
    overflow-y: auto;
  }

  .cell {
    position: relative;
    display: grid;
  }

  .actions {
    position: absolute;
    top: 2px;
    right: 2px;
    display: none;
    gap: 2px;
  }

  .cell:hover .actions,
  .cell:focus-within .actions {
    display: flex;
  }

  .actions button {
    width: 16px;
    height: 16px;
    padding: 0;
    display: grid;
    place-items: center;
    font-size: 10px;
    line-height: 1;
    color: var(--text-0);
    background: var(--bg-0);
    border: none;
    border-radius: 4px;
    cursor: pointer;
  }

  .actions button:hover {
    background: var(--accent);
    color: #fff;
  }

  .add.cancel {
    background: var(--bg-3);
  }

  .sound {
    aspect-ratio: 1;
    display: grid;
    place-items: center;
    font-size: 20px;
    line-height: 1;
    background: var(--bg-2);
    border: 1px solid transparent;
    border-radius: var(--radius);
    cursor: pointer;
    padding: 0;
  }

  .sound:hover:not(:disabled) {
    background: var(--bg-3);
    border-color: var(--bg-3);
  }

  .sound:active:not(:disabled) {
    transform: scale(0.94);
  }

  .sound:disabled {
    opacity: 0.35;
    cursor: not-allowed;
  }

  .hint,
  .empty {
    margin: 0;
    font-size: 11px;
    color: var(--text-2);
  }

  .upload {
    display: flex;
    align-items: center;
    gap: 6px;
  }

  .emoji-in {
    width: 34px;
    text-align: center;
  }

  .name-in {
    flex: 1;
    min-width: 0;
  }

  .emoji-in,
  .name-in {
    background: var(--bg-0);
    border: 1px solid var(--bg-3);
    border-radius: var(--radius);
    color: var(--text-0);
    font: inherit;
    font-size: 12px;
    padding: 5px 7px;
    outline: none;
  }

  .emoji-in:focus,
  .name-in:focus {
    border-color: var(--accent);
  }

  .add {
    flex: none;
    width: 30px;
    height: 30px;
    display: grid;
    place-items: center;
    border: none;
    border-radius: var(--radius);
    background: var(--accent);
    color: #fff;
    font-size: 15px;
    line-height: 1;
    cursor: pointer;
  }

  .add:hover:not(:disabled) {
    background: var(--accent-hover);
  }

  .add:disabled {
    opacity: 0.5;
    cursor: default;
  }

  .file {
    position: absolute;
    width: 1px;
    height: 1px;
    opacity: 0;
    pointer-events: none;
  }

  .error {
    margin: 0;
    font-size: 11px;
    color: var(--danger);
  }
</style>
