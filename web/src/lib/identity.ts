import { downloadBackup, Identity } from './api'
import { app } from './state.svelte'
import type { Identity as IdentityData, UserPublic } from './protocol'

export function selfUser(): UserPublic | null {
  const id = app.identity
  if (!id) return null
  return { id: id.public_id, display_name: id.display_name, avatar_url: null }
}

export function shortPublicId(publicId: string): string {
  if (!publicId) return ''
  return publicId.length > 14 ? `${publicId.slice(0, 8)}…${publicId.slice(-4)}` : publicId
}

export async function loadIdentity(): Promise<IdentityData | null> {
  const id = await Identity.get()
  app.identity = id
  return id
}

export async function createIdentity(displayName: string): Promise<IdentityData> {
  const id = await Identity.create(displayName)
  app.identity = id
  return id
}

export async function importIdentity(backup: string, passphrase?: string): Promise<IdentityData> {
  const id = await Identity.import(backup, passphrase)
  app.identity = id
  return id
}

export async function exportBackup(): Promise<void> {
  const blob = await downloadBackup()
  const objectUrl = URL.createObjectURL(blob)
  const a = document.createElement('a')
  a.href = objectUrl
  a.download = 'cadence-identity.backup'
  document.body.appendChild(a)
  a.click()
  a.remove()
  URL.revokeObjectURL(objectUrl)
}
