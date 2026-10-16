import { ws } from './ws'
import type { Id, ServerEvent, SignalData, VoiceMember } from './protocol'

function buildIceServers(): RTCIceServer[] {
  const rawJson = import.meta.env.VITE_ICE_SERVERS
  if (rawJson) {
    try {
      return JSON.parse(rawJson) as RTCIceServer[]
    } catch {
      void 0
    }
  }
  const rawList: string =
    (import.meta.env.VITE_STUN as string | undefined) ??
    'stun:stun.cloudflare.com:3478,stun:stun.l.google.com:19302'
  return rawList
    .split(',')
    .map((url) => url.trim())
    .filter(Boolean)
    .map((urls) => ({ urls }))
}

const ICE_SERVERS: RTCIceServer[] = buildIceServers()
let nodeIceServers: RTCIceServer[] = []

export function setNodeIceServers(servers: RTCIceServer[]): void {
  nodeIceServers = servers
}

export interface PeerState {
  id: Id
  stream: MediaStream | null
  video: boolean
  screen: boolean
  muted: boolean
  conn: string
}

export interface VoiceSessionOptions {
  selfId: Id
  onChange?: (session: VoiceSession) => void
}

interface Peer {
  id: Id
  pc: RTCPeerConnection
  stream: MediaStream
  audioSender: RTCRtpSender | null
  videoSender: RTCRtpSender | null
  polite: boolean
  makingOffer: boolean
  ignoreOffer: boolean
  pendingIce: RTCIceCandidateInit[]
}

export class VoiceSession {
  readonly selfId: Id
  channelId: Id | null = null
  localStream: MediaStream | null = null
  screenStream: MediaStream | null = null
  readonly peers = new Map<Id, PeerState>()
  connected = false
  muted = false
  deafened = false
  video = false
  screen = false
  error: string | null = null

  private readonly onChange?: (session: VoiceSession) => void
  private readonly pcs = new Map<Id, Peer>()
  private unsubscribe: (() => void) | null = null
  private micTrack: MediaStreamTrack | null = null
  private videoTrack: MediaStreamTrack | null = null
  private screenTrack: MediaStreamTrack | null = null
  private closed = false

  constructor(opts: VoiceSessionOptions) {
    this.selfId = opts.selfId
    this.onChange = opts.onChange
  }

  async join(channelId: Id, peerIds: readonly Id[] = []): Promise<void> {
    if (this.channelId === channelId) return
    if (this.channelId != null) this.leave()
    this.closed = false
    this.channelId = channelId
    this.error = null
    this.unsubscribe = ws.on((ev) => this.handle(ev))
    const stream = new MediaStream()
    this.localStream = stream
    try {
      const mic = await navigator.mediaDevices.getUserMedia({ audio: true })
      const track = mic.getAudioTracks()[0] ?? null
      this.micTrack = track
      if (track) {
        track.enabled = !this.muted
        stream.addTrack(track)
        for (const peer of this.pcs.values()) void peer.audioSender?.replaceTrack(track).catch(() => {})
      }
    } catch {
      this.error = 'Could not access the microphone'
    }
    if (this.closed) return
    this.notify()
    for (const id of peerIds) this.ensureOffer(id)
  }

  leave(): void {
    this.closed = true
    if (this.unsubscribe) {
      this.unsubscribe()
      this.unsubscribe = null
    }
    for (const peer of this.pcs.values()) {
      try {
        peer.pc.close()
      } catch {
        void 0
      }
    }
    this.pcs.clear()
    this.peers.clear()
    if (this.micTrack) {
      this.micTrack.stop()
      this.micTrack = null
    }
    if (this.videoTrack) {
      this.videoTrack.stop()
      this.videoTrack = null
    }
    if (this.screenTrack) {
      this.screenTrack.stop()
      this.screenTrack = null
    }
    this.localStream = null
    this.screenStream = null
    this.connected = false
    this.channelId = null
    this.notify()
  }

  setMuted(value: boolean): void {
    this.muted = value
    if (this.micTrack) this.micTrack.enabled = !value
    this.notify()
  }

  setDeafened(value: boolean): void {
    this.deafened = value
    this.notify()
  }

  async setVideo(value: boolean): Promise<void> {
    if (value === this.video) return
    this.video = value
    if (value) {
      if (!this.videoTrack) {
        try {
          const cam = await navigator.mediaDevices.getUserMedia({ video: true })
          const track = cam.getVideoTracks()[0] ?? null
          this.videoTrack = track
          if (track && this.localStream && !this.localStream.getVideoTracks().includes(track)) {
            this.localStream.addTrack(track)
          }
        } catch {
          this.video = false
          this.error = 'Could not access the camera'
          this.notify()
          return
        }
      }
      if (this.closed) return
      await this.applyVideoTrack(this.screenTrack ?? this.videoTrack)
    } else {
      await this.applyVideoTrack(this.screenTrack ?? null)
      if (this.videoTrack) {
        this.videoTrack.stop()
        if (this.localStream) this.localStream.removeTrack(this.videoTrack)
        this.videoTrack = null
      }
    }
    this.notify()
  }

  async setScreen(value: boolean): Promise<void> {
    if (value === this.screen) return
    if (value) {
      const md = navigator.mediaDevices
      if (!md || typeof md.getDisplayMedia !== 'function') {
        this.error = 'Screen sharing unavailable'
        this.notify()
        return
      }
      try {
        const display = await md.getDisplayMedia({ video: true, audio: false })
        const track = display.getVideoTracks()[0] ?? null
        this.screenStream = display
        this.screenTrack = track
        this.screen = true
        if (track) {
          track.addEventListener('ended', () => {
            void this.setScreen(false)
          })
        }
        if (this.closed) {
          track?.stop()
          return
        }
        await this.applyVideoTrack(track)
      } catch {
        this.screen = false
        this.error = 'Screen sharing cancelled'
      }
      this.notify()
      return
    }
    this.screen = false
    if (this.screenTrack) this.screenTrack.stop()
    if (this.screenStream) this.screenStream.getTracks().forEach((t) => t.stop())
    this.screenTrack = null
    this.screenStream = null
    await this.applyVideoTrack(this.video ? this.videoTrack : null)
    this.notify()
  }

  private async applyVideoTrack(track: MediaStreamTrack | null): Promise<void> {
    for (const peer of this.pcs.values()) {
      if (!peer.videoSender) continue
      try {
        await peer.videoSender.replaceTrack(track)
      } catch {
        void 0
      }
    }
  }

  private handle(ev: ServerEvent): void {
    switch (ev.op) {
      case 'signal':
        void this.onSignal(ev.from, ev.data)
        break
      case 'voice_updated':
        if (ev.channel_id === this.channelId) this.setMember(ev.member)
        break
      case 'voice_joined':
        if (ev.presence.channel_id === this.channelId) {
          for (const member of ev.presence.members) {
            this.setMember(member)
            this.ensureOffer(member.user.id)
          }
        }
        break
      case 'voice_presence':
        if (ev.presence.channel_id === this.channelId) {
          for (const member of ev.presence.members) {
            this.setMember(member)
            this.ensureOffer(member.user.id)
          }
        }
        break
      case 'voice_left':
        if (ev.channel_id === this.channelId && this.pcs.get(ev.user_id)?.pc.connectionState !== 'connected') {
          this.drop(ev.user_id)
        }
        break
      default:
        break
    }
  }

  private ensureOffer(id: Id): void {
    if (id === this.selfId || this.pcs.has(id)) return
    if (this.selfId < id) void this.startOffer(id)
  }

  private setMember(member: VoiceMember): void {
    if (member.user.id === this.selfId) {
      this.muted = member.muted
      this.deafened = member.deafened
      this.video = member.video
      this.screen = member.screen
      if (this.micTrack) this.micTrack.enabled = !member.muted
      this.notify()
      return
    }
    const state = this.peerState(member.user.id)
    state.muted = member.muted
    state.video = member.video
    state.screen = member.screen
    this.notify()
  }

  private peerState(id: Id): PeerState {
    let state = this.peers.get(id)
    if (!state) {
      state = { id, stream: null, video: false, screen: false, muted: false, conn: '' }
      this.peers.set(id, state)
    }
    return state
  }

  private createPeer(id: Id): Peer {
    const pc = new RTCPeerConnection({ iceServers: [...ICE_SERVERS, ...nodeIceServers] })
    const stream = new MediaStream()
    const peer: Peer = {
      id,
      pc,
      stream,
      audioSender: null,
      videoSender: null,
      polite: this.selfId > id,
      makingOffer: false,
      ignoreOffer: false,
      pendingIce: [],
    }
    pc.onicecandidate = (e) => {
      const candidate = e.candidate
      if (!candidate) return
      this.sendSignal(id, {
        kind: 'ice',
        candidate: candidate.candidate,
        sdp_mid: candidate.sdpMid,
        sdp_mline_index: candidate.sdpMLineIndex,
      })
    }
    pc.onconnectionstatechange = () => {
      this.peerState(id).conn = pc.connectionState
      console.log('[rtc] pc', id.slice(0, 8), '->', pc.connectionState, 'ice:', pc.iceConnectionState)
      if (pc.connectionState === 'failed') {
        void this.restartIce(peer)
        return
      }
      this.updateConnected()
      this.notify()
    }
    pc.ontrack = (e) => {
      console.log('[rtc] ontrack', id.slice(0, 8), e.track.kind)
      if (!stream.getTracks().some((t) => t.id === e.track.id)) stream.addTrack(e.track)
      this.peerState(id).stream = stream
      this.notify()
    }
    this.peerState(id).stream = stream
    return peer
  }

  private async restartIce(peer: Peer): Promise<void> {
    peer.pc.restartIce()
    setTimeout(() => {
      if (this.pcs.get(peer.id) === peer && peer.pc.connectionState === 'failed') this.drop(peer.id)
    }, 15000)
    if (this.selfId > peer.id) return
    try {
      peer.makingOffer = true
      const offer = await peer.pc.createOffer({ iceRestart: true })
      await peer.pc.setLocalDescription(offer)
      this.sendSignal(peer.id, { kind: 'offer', sdp: offer.sdp ?? '' })
    } catch (e) {
      console.error('[rtc] restartIce error', e)
    } finally {
      peer.makingOffer = false
    }
  }

  private async startOffer(peerId: Id): Promise<void> {
    if (this.closed || peerId === this.selfId || this.pcs.has(peerId)) return
    const peer = this.createPeer(peerId)
    this.pcs.set(peerId, peer)
    const audio = peer.pc.addTransceiver('audio', { direction: 'sendrecv' })
    const video = peer.pc.addTransceiver('video', { direction: 'sendrecv' })
    peer.audioSender = audio.sender
    peer.videoSender = video.sender
    if (this.micTrack) await audio.sender.replaceTrack(this.micTrack)
    const outgoing = this.screenTrack ?? this.videoTrack
    if (outgoing) await video.sender.replaceTrack(outgoing)
    if (this.closed) return
    try {
      peer.makingOffer = true
      const offer = await peer.pc.createOffer()
      await peer.pc.setLocalDescription(offer)
      console.log('[rtc] offer ->', peerId.slice(0, 8), 'sdp', (offer.sdp ?? '').length, 'bytes')
      this.sendSignal(peerId, { kind: 'offer', sdp: offer.sdp ?? '' })
    } catch (e) {
      console.error('[rtc] startOffer error', e)
    } finally {
      peer.makingOffer = false
    }
  }

  private async onSignal(from: Id, data: SignalData): Promise<void> {
    if (this.closed || from === this.selfId) return
    if (data.kind !== 'ice') console.log('[rtc] signal in', data.kind, 'from', from.slice(0, 8))
    let peer = this.pcs.get(from)
    if (!peer) {
      peer = this.createPeer(from)
      this.pcs.set(from, peer)
    }
    const pc = peer.pc
    if (data.kind === 'ice') {
      const init: RTCIceCandidateInit = {
        candidate: data.candidate,
        sdpMid: data.sdp_mid ?? undefined,
        sdpMLineIndex: data.sdp_mline_index ?? undefined,
      }
      if (!pc.remoteDescription) {
        peer.pendingIce.push(init)
        return
      }
      try {
        await pc.addIceCandidate(init)
      } catch (e) {
        if (!peer.ignoreOffer) console.warn('[rtc] addIceCandidate', e)
      }
      return
    }
    if (data.kind === 'offer') {
      const collision = peer.makingOffer || pc.signalingState !== 'stable'
      peer.ignoreOffer = !peer.polite && collision
      if (peer.ignoreOffer) return
      try {
        if (collision) await pc.setLocalDescription({ type: 'rollback' })
        await pc.setRemoteDescription({ type: 'offer', sdp: data.sdp })
        this.setSenders(peer)
        if (this.micTrack && peer.audioSender) await peer.audioSender.replaceTrack(this.micTrack)
        const outgoing = this.screenTrack ?? this.videoTrack
        if (outgoing && peer.videoSender) await peer.videoSender.replaceTrack(outgoing)
        await this.flushIce(peer)
        const answer = await pc.createAnswer()
        await pc.setLocalDescription(answer)
        this.sendSignal(from, { kind: 'answer', sdp: answer.sdp ?? '' })
      } catch (e) {
        console.error('[rtc] answer error', e)
      }
      return
    }
    if (pc.signalingState !== 'have-local-offer') return
    try {
      await pc.setRemoteDescription({ type: 'answer', sdp: data.sdp })
      await this.flushIce(peer)
    } catch (e) {
      console.error('[rtc] setAnswer error', e)
    }
  }

  // Transceivers created by setRemoteDescription default to recvonly, and ones we added before a
  // rollback stay unnegotiated (mid null) forever; send on the negotiated ones or the answer is recvonly.
  private setSenders(peer: Peer): void {
    peer.audioSender = null
    peer.videoSender = null
    for (const transceiver of peer.pc.getTransceivers()) {
      if (transceiver.mid == null || transceiver.currentDirection === 'stopped') continue
      const kind = transceiver.receiver.track.kind
      if (kind === 'audio' && !peer.audioSender) peer.audioSender = transceiver.sender
      else if (kind === 'video' && !peer.videoSender) peer.videoSender = transceiver.sender
      else continue
      transceiver.direction = 'sendrecv'
    }
  }

  private async flushIce(peer: Peer): Promise<void> {
    const pending = peer.pendingIce
    peer.pendingIce = []
    for (const candidate of pending) {
      try {
        await peer.pc.addIceCandidate(candidate)
      } catch {
        void 0
      }
    }
  }

  private updateConnected(): void {
    let value = false
    for (const peer of this.pcs.values()) {
      if (peer.pc.connectionState === 'connected') {
        value = true
        break
      }
    }
    this.connected = value
  }

  private drop(id: Id): void {
    const peer = this.pcs.get(id)
    if (peer) {
      try {
        peer.pc.close()
      } catch {
        void 0
      }
      this.pcs.delete(id)
    }
    this.peers.delete(id)
    this.updateConnected()
    this.notify()
  }

  private sendSignal(to: Id, data: SignalData): void {
    ws.send({ op: 'signal', to, data })
  }

  private notify(): void {
    this.onChange?.(this)
  }
}
