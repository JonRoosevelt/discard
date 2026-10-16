export type Id = string

export type ChannelKind = 'text' | 'voice'

export interface Identity {
  public_id: string
  display_name: string
  has_backup: boolean
}

export interface UserPublic {
  id: Id
  display_name: string
  avatar_url: string | null
}

export interface Workspace {
  id: Id
  name: string
  icon_url: string | null
  owner_id: Id
  ticket: string | null
  secret: string | null
  channels: Channel[]
}

export interface Channel {
  id: Id
  workspace_id: Id
  name: string
  kind: ChannelKind
  position: number
}

export interface Attachment {
  id: Id
  filename: string
  url: string
  content_type: string
  size: number
  width: number | null
  height: number | null
}

export interface Message {
  id: Id
  channel_id: Id
  author: UserPublic
  content: string
  created_at: string
  attachments: Attachment[]
}

export interface Sound {
  id: string
  name: string
  url: string
  emoji: string
  builtin: boolean
  owner: string
  hash: string
}

export interface VoiceMember {
  user: UserPublic
  muted: boolean
  deafened: boolean
  video: boolean
  screen: boolean
}

export interface VoicePresence {
  channel_id: Id
  workspace_id: Id
  members: VoiceMember[]
}

export interface CreateIdentityRequest {
  display_name: string
}

export interface ImportIdentityRequest {
  backup: string
  passphrase?: string | null
}

export interface CreateWorkspaceRequest {
  name: string
}

export interface JoinWorkspaceRequest {
  ticket: string
}

export interface CreateChannelRequest {
  name: string
  kind: ChannelKind
  id?: Id
}

export interface RenameRequest {
  name: string
}

export interface SendMessageRequest {
  content: string
  attachment_ids: Id[]
}

export type SignalData =
  | { kind: 'offer'; sdp: string }
  | { kind: 'answer'; sdp: string }
  | { kind: 'ice'; candidate: string; sdp_mid?: string | null; sdp_mline_index?: number | null }
  | { kind: 'hello' }

export type ClientEvent =
  | { op: 'subscribe'; channel_id: Id }
  | { op: 'unsubscribe'; channel_id: Id }
  | { op: 'send_message'; channel_id: Id; content: string; attachment_ids: Id[] }
  | { op: 'typing'; channel_id: Id }
  | { op: 'voice_join'; channel_id: Id; muted: boolean; video: boolean }
  | { op: 'voice_update'; muted: boolean; deafened: boolean; video: boolean; screen: boolean }
  | { op: 'voice_leave'; channel_id: Id }
  | { op: 'signal'; to: Id; data: SignalData; session?: string }
  | { op: 'sound_play'; channel_id: Id; sound_id: string }
  | { op: 'ping' }

export type ServerEvent =
  | {
      op: 'ready'
      identity: Identity
      workspaces: Workspace[]
      voice: VoicePresence[]
      sounds: Sound[]
      ice_servers?: RTCIceServer[]
    }
  | { op: 'message'; message: Message }
  | { op: 'typing'; channel_id: Id; user: UserPublic }
  | { op: 'voice_presence'; presence: VoicePresence }
  | { op: 'voice_joined'; presence: VoicePresence }
  | { op: 'voice_left'; channel_id: Id; workspace_id: Id; user_id: Id }
  | { op: 'voice_updated'; channel_id: Id; member: VoiceMember }
  | { op: 'signal'; from: Id; data: SignalData; session?: string | null }
  | { op: 'sound_play'; channel_id: Id; sound_id: string; by: Id }
  | { op: 'sounds'; sounds: Sound[] }
  | { op: 'error'; message: string }
  | { op: 'pong' }
  | { op: 'workspace_updated'; workspace: Workspace }
  | { op: 'workspace_removed'; workspace_id: Id }
