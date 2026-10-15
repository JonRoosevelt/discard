import { token } from './api'
import type { ClientEvent, ServerEvent } from './protocol'

export type WsStatus = 'offline' | 'connecting' | 'online'

type Handler = (ev: ServerEvent) => void

const handlers = new Set<Handler>()
const openHandlers = new Set<() => void>()
const statusHandlers = new Set<(s: WsStatus) => void>()

let socket: WebSocket | null = null
let reconnectTimer: ReturnType<typeof setTimeout> | null = null
let attempt = 0
let manual = false
let queue: ClientEvent[] = []
let current: WsStatus = 'offline'

function url(t: string): string {
  return `${location.origin.replace(/^http/, 'ws')}/ws?token=${encodeURIComponent(t)}`
}

function setStatus(s: WsStatus): void {
  if (current === s) return
  current = s
  statusHandlers.forEach((h) => h(s))
}

function scheduleReconnect(): void {
  if (reconnectTimer || manual || !token.get()) return
  const delay = Math.min(30000, 500 * 2 ** attempt)
  attempt += 1
  reconnectTimer = setTimeout(() => {
    reconnectTimer = null
    open()
  }, delay)
}

function open(): void {
  const t = token.get()
  if (!t || manual) return
  if (socket && (socket.readyState === WebSocket.OPEN || socket.readyState === WebSocket.CONNECTING)) {
    return
  }
  setStatus('connecting')
  let s: WebSocket
  try {
    s = new WebSocket(url(t))
  } catch {
    setStatus('offline')
    scheduleReconnect()
    return
  }
  socket = s

  s.onopen = () => {
    attempt = 0
    setStatus('online')
    const pending = queue
    queue = []
    for (const ev of pending) {
      try {
        s.send(JSON.stringify(ev))
      } catch {
        queue.push(ev)
      }
    }
    openHandlers.forEach((h) => h())
  }

  s.onmessage = (e) => {
    let ev: ServerEvent
    try {
      ev = JSON.parse(e.data as string) as ServerEvent
    } catch {
      return
    }
    if (ev.op === 'pong') return
    handlers.forEach((h) => h(ev))
  }

  s.onclose = () => {
    if (socket === s) socket = null
    setStatus('offline')
    if (!manual) scheduleReconnect()
  }

  s.onerror = () => {
    try {
      s.close()
    } catch {
      void 0
    }
  }
}

function connect(): void {
  if (!token.get()) return
  manual = false
  attempt = 0
  if (reconnectTimer) {
    clearTimeout(reconnectTimer)
    reconnectTimer = null
  }
  open()
}

function send(ev: ClientEvent): void {
  if (socket && socket.readyState === WebSocket.OPEN) {
    try {
      socket.send(JSON.stringify(ev))
      return
    } catch {
      void 0
    }
  }
  queue.push(ev)
  if (!socket && !manual && token.get()) open()
}

function close(): void {
  manual = true
  queue = []
  attempt = 0
  if (reconnectTimer) {
    clearTimeout(reconnectTimer)
    reconnectTimer = null
  }
  if (socket) {
    const s = socket
    socket = null
    try {
      s.close()
    } catch {
      void 0
    }
  }
  setStatus('offline')
}

export const ws = {
  connect,
  send,
  on(handler: Handler): () => void {
    handlers.add(handler)
    return () => handlers.delete(handler)
  },
  status(): WsStatus {
    return current
  },
  close,
}

export function onReconnect(handler: () => void): () => void {
  openHandlers.add(handler)
  return () => openHandlers.delete(handler)
}

export function onStatus(handler: (s: WsStatus) => void): () => void {
  statusHandlers.add(handler)
  return () => statusHandlers.delete(handler)
}
