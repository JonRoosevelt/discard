const INFO = new TextEncoder().encode('discard-content-v1')
const PREFIX = 'E1:'
const NONCE_BYTES = 12

function hexToBytes(hex: string): Uint8Array<ArrayBuffer> {
  const clean = hex.trim()
  if (clean.length % 2 !== 0 || /[^0-9a-fA-F]/.test(clean)) {
    throw new Error('invalid secret')
  }
  const out = new Uint8Array(clean.length / 2)
  for (let i = 0; i < out.length; i++) {
    out[i] = parseInt(clean.slice(i * 2, i * 2 + 2), 16)
  }
  return out
}

function bytesToBase64(bytes: Uint8Array): string {
  let bin = ''
  for (let i = 0; i < bytes.length; i++) bin += String.fromCharCode(bytes[i])
  return btoa(bin)
}

function base64ToBytes(b64: string): Uint8Array<ArrayBuffer> {
  const bin = atob(b64)
  const out = new Uint8Array(bin.length)
  for (let i = 0; i < bin.length; i++) out[i] = bin.charCodeAt(i)
  return out
}

export async function deriveKey(secretHex: string): Promise<CryptoKey> {
  const raw = hexToBytes(secretHex)
  if (raw.length === 0) throw new Error('empty secret')
  const base = await crypto.subtle.importKey('raw', raw, 'HKDF', false, ['deriveKey'])
  return crypto.subtle.deriveKey(
    {
      name: 'HKDF',
      hash: 'SHA-256',
      salt: new Uint8Array(0),
      info: INFO,
    },
    base,
    { name: 'AES-GCM', length: 256 },
    false,
    ['encrypt', 'decrypt'],
  )
}

export async function encryptText(key: CryptoKey, text: string): Promise<string> {
  const nonce = crypto.getRandomValues(new Uint8Array(NONCE_BYTES))
  const sealed = new Uint8Array(
    await crypto.subtle.encrypt(
      { name: 'AES-GCM', iv: nonce },
      key,
      new TextEncoder().encode(text),
    ),
  )
  const out = new Uint8Array(nonce.length + sealed.length)
  out.set(nonce, 0)
  out.set(sealed, nonce.length)
  return PREFIX + bytesToBase64(out)
}

export async function decryptText(key: CryptoKey, payload: string): Promise<string> {
  if (!payload.startsWith(PREFIX)) return payload
  const raw = base64ToBytes(payload.slice(PREFIX.length))
  if (raw.length <= NONCE_BYTES) throw new Error('invalid envelope')
  const nonce = raw.subarray(0, NONCE_BYTES)
  const sealed = raw.subarray(NONCE_BYTES)
  const opened = await crypto.subtle.decrypt({ name: 'AES-GCM', iv: nonce }, key, sealed)
  return new TextDecoder().decode(opened)
}
