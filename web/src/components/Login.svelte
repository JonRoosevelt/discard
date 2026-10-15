<script lang="ts">
  import { app } from '../lib/state.svelte'
  import { createIdentity, exportBackup, importIdentity, shortPublicId } from '../lib/identity'
  import { avatarColor, initials } from '../lib/format'

  let mode = $state<'create' | 'import'>('create')
  let displayName = $state('')
  let backup = $state('')
  let passphrase = $state('')
  let error = $state('')
  let busy = $state(false)

  let identity = $derived(app.identity)

  async function submitCreate(event: SubmitEvent): Promise<void> {
    event.preventDefault()
    const name = displayName.trim()
    if (!name) {
      error = 'Informe um nome de exibição'
      return
    }
    busy = true
    error = ''
    try {
      await createIdentity(name)
    } catch (e) {
      error = e instanceof Error ? e.message : 'Falha ao criar identidade'
    } finally {
      busy = false
    }
  }

  async function submitImport(event: SubmitEvent): Promise<void> {
    event.preventDefault()
    const data = backup.trim()
    if (!data) {
      error = 'Cole o conteúdo do backup'
      return
    }
    busy = true
    error = ''
    try {
      await importIdentity(data, passphrase.trim() || undefined)
    } catch (e) {
      error = e instanceof Error ? e.message : 'Falha ao importar backup'
    } finally {
      busy = false
    }
  }

  async function onBackupFile(event: Event): Promise<void> {
    const input = event.currentTarget as HTMLInputElement
    const file = input.files?.[0]
    if (file) backup = await file.text()
    input.value = ''
  }

  async function enter(): Promise<void> {
    busy = true
    error = ''
    try {
      await app.enter()
    } catch (e) {
      error = e instanceof Error ? e.message : 'Falha ao entrar'
    } finally {
      busy = false
    }
  }

  async function saveBackup(): Promise<void> {
    error = ''
    try {
      await exportBackup()
    } catch (e) {
      error = e instanceof Error ? e.message : 'Falha ao exportar backup'
    }
  }
</script>

<div class="identity">
  <div class="card">
    <h1>Discard</h1>

    {#if identity}
      <p class="sub">Identidade local</p>
      <div class="who">
        <span class="avatar" style="background: {avatarColor(identity.display_name)}">
          {initials(identity.display_name)}
        </span>
        <div class="meta">
          <strong>{identity.display_name}</strong>
          <code>{shortPublicId(identity.public_id)}</code>
        </div>
      </div>
      <p class="note">
        {identity.has_backup
          ? 'Você tem um backup desta identidade.'
          : 'Sem backup exportado. Exporte para não perder esta identidade.'}
      </p>
      <button type="button" class="primary" disabled={busy} onclick={enter}>
        {busy ? 'Aguarde…' : 'Entrar'}
      </button>
      <button type="button" class="ghost" onclick={saveBackup}>Exportar backup</button>
    {:else}
      <p class="sub">
        {mode === 'create' ? 'Crie sua identidade neste dispositivo' : 'Importe um backup existente'}
      </p>

      {#if mode === 'create'}
        <form onsubmit={submitCreate}>
          <label for="display">Nome de exibição</label>
          <input
            id="display"
            type="text"
            autocomplete="nickname"
            bind:value={displayName}
            placeholder="Seu nome"
            maxlength="40"
          />
          <button type="submit" disabled={busy}>{busy ? 'Criando…' : 'Criar identidade'}</button>
          <button type="button" class="ghost" onclick={() => (mode = 'import')}>
            Importar backup
          </button>
        </form>
      {:else}
        <form onsubmit={submitImport}>
          <label for="backup">Backup</label>
          <textarea id="backup" bind:value={backup} placeholder="Cole o conteúdo do backup"></textarea>
          <label class="file">
            <span>Carregar arquivo…</span>
            <input type="file" accept=".json,.txt,.key,application/json" onchange={onBackupFile} />
          </label>
          <label for="passphrase">Passphrase (opcional, local)</label>
          <input
            id="passphrase"
            type="password"
            autocomplete="off"
            bind:value={passphrase}
            placeholder="só se o backup tiver"
          />
          <button type="submit" disabled={busy}>{busy ? 'Importando…' : 'Importar'}</button>
          <button type="button" class="ghost" onclick={() => (mode = 'create')}>Criar nova</button>
        </form>
      {/if}
    {/if}

    {#if error}
      <p class="error">{error}</p>
    {/if}

    <p class="privacy">Chaves ficam só neste dispositivo. Mensagens desaparecem em 24h.</p>
  </div>
</div>

<style>
  .identity {
    display: grid;
    place-items: center;
    min-height: 100vh;
    background: var(--bg-0);
  }

  .card {
    width: 360px;
    display: flex;
    flex-direction: column;
    gap: 8px;
    padding: 28px 24px;
    background: var(--bg-2);
    border-radius: 8px;
    box-shadow: 0 8px 24px rgba(0, 0, 0, 0.4);
  }

  h1 {
    margin: 0;
    font-size: 24px;
    color: var(--text-0);
    text-align: center;
    letter-spacing: -0.5px;
  }

  .sub {
    margin: 0 0 12px;
    text-align: center;
    color: var(--text-2);
    font-size: 14px;
  }

  .who {
    display: flex;
    align-items: center;
    gap: 12px;
    padding: 10px;
    background: var(--bg-1);
    border-radius: var(--radius);
  }

  .avatar {
    width: 40px;
    height: 40px;
    border-radius: 50%;
    display: grid;
    place-items: center;
    color: #fff;
    font-size: 14px;
    font-weight: 600;
    flex: none;
  }

  .meta {
    display: flex;
    flex-direction: column;
    min-width: 0;
  }

  .meta strong {
    color: var(--text-0);
    font-size: 15px;
  }

  .meta code {
    color: var(--text-2);
    font-size: 12px;
  }

  .note {
    margin: 0;
    font-size: 12px;
    color: var(--text-2);
  }

  label {
    font-size: 12px;
    text-transform: uppercase;
    letter-spacing: 0.4px;
    color: var(--text-2);
    margin-top: 6px;
  }

  input,
  textarea {
    background: var(--bg-0);
    border: 1px solid var(--bg-3);
    border-radius: var(--radius);
    color: var(--text-0);
    padding: 9px 10px;
    font: inherit;
    outline: none;
  }

  textarea {
    resize: vertical;
    min-height: 84px;
    font-family: ui-monospace, SFMono-Regular, Menlo, monospace;
    font-size: 12px;
  }

  input:focus,
  textarea:focus {
    border-color: var(--accent);
  }

  button {
    margin-top: 10px;
    padding: 10px;
    border: none;
    border-radius: var(--radius);
    background: var(--accent);
    color: #fff;
    font: inherit;
    font-weight: 600;
    cursor: pointer;
  }

  button:hover:not(:disabled) {
    background: var(--accent-hover);
  }

  button:disabled {
    opacity: 0.6;
    cursor: default;
  }

  .ghost {
    background: transparent;
    color: var(--text-2);
    font-weight: 400;
    font-size: 13px;
    margin-top: 4px;
  }

  .ghost:hover:not(:disabled) {
    background: transparent;
    color: var(--text-0);
    text-decoration: underline;
  }

  .file {
    display: inline-block;
    margin-top: 4px;
    font-size: 12px;
    color: var(--accent);
    text-transform: none;
    letter-spacing: 0;
    cursor: pointer;
  }

  .file input {
    display: none;
  }

  .error {
    margin: 6px 0 0;
    color: var(--danger);
    font-size: 13px;
  }

  .privacy {
    margin: 12px 0 0;
    text-align: center;
    color: var(--text-2);
    font-size: 11px;
  }
</style>
