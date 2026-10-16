use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

pub type Id = String;

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum ChannelKind {
    Text,
    Voice,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct UserPublic {
    pub id: Id,
    pub display_name: String,
    pub avatar_url: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Identity {
    pub public_id: String,
    pub display_name: String,
    pub has_backup: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Workspace {
    pub id: Id,
    pub name: String,
    pub icon_url: Option<String>,
    pub owner_id: Id,
    pub ticket: String,
    #[serde(default)]
    pub secret: Option<String>,
    pub channels: Vec<Channel>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Channel {
    pub id: Id,
    pub workspace_id: Id,
    pub name: String,
    pub kind: ChannelKind,
    pub position: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Attachment {
    pub id: Id,
    pub filename: String,
    pub url: String,
    pub content_type: String,
    pub size: i64,
    pub width: Option<i32>,
    pub height: Option<i32>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Message {
    pub id: Id,
    pub channel_id: Id,
    pub author: UserPublic,
    pub content: String,
    pub created_at: DateTime<Utc>,
    pub attachments: Vec<Attachment>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Sound {
    pub id: String,
    pub name: String,
    pub url: String,
    pub emoji: String,
    pub builtin: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct VoiceMember {
    pub user: UserPublic,
    pub muted: bool,
    pub deafened: bool,
    pub video: bool,
    pub screen: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct VoicePresence {
    pub channel_id: Id,
    pub workspace_id: Id,
    pub members: Vec<VoiceMember>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateIdentityRequest {
    pub display_name: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImportIdentityRequest {
    pub backup: String,
    #[serde(default)]
    pub passphrase: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateWorkspaceRequest {
    pub name: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JoinWorkspaceRequest {
    pub ticket: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateChannelRequest {
    pub name: String,
    pub kind: ChannelKind,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SendMessageRequest {
    pub content: String,
    #[serde(default)]
    pub attachment_ids: Vec<Id>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IceServer {
    pub urls: Vec<String>,
    pub username: String,
    pub credential: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "lowercase")]
pub enum SignalData {
    Offer { sdp: String },
    Answer { sdp: String },
    Ice {
        candidate: String,
        #[serde(default)]
        sdp_mid: Option<String>,
        #[serde(default)]
        sdp_mline_index: Option<u16>,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "op", rename_all = "snake_case")]
pub enum ClientEvent {
    Subscribe { channel_id: Id },
    Unsubscribe { channel_id: Id },
    SendMessage { channel_id: Id, content: String, attachment_ids: Vec<Id> },
    Typing { channel_id: Id },
    VoiceJoin { channel_id: Id, muted: bool, video: bool },
    VoiceUpdate { muted: bool, deafened: bool, video: bool, screen: bool },
    VoiceLeave { channel_id: Id },
    Signal { to: Id, data: SignalData },
    SoundPlay { channel_id: Id, sound_id: String },
    Ping,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "op", rename_all = "snake_case")]
pub enum ServerEvent {
    Ready {
        identity: Identity,
        workspaces: Vec<Workspace>,
        voice: Vec<VoicePresence>,
        sounds: Vec<Sound>,
        ice_servers: Vec<IceServer>,
    },
    Message { message: Message },
    MessageDeleted { channel_id: Id, id: Id },
    Typing { channel_id: Id, user: UserPublic },
    VoicePresence { presence: VoicePresence },
    VoiceJoined { presence: VoicePresence },
    VoiceLeft { channel_id: Id, workspace_id: Id, user_id: Id },
    VoiceUpdated { channel_id: Id, member: VoiceMember },
    Signal { from: Id, data: SignalData },
    SoundPlay { channel_id: Id, sound_id: String, by: Id },
    Error { message: String },
    Pong,
}

impl ClientEvent {
    pub fn parse(text: &str) -> Result<Self, serde_json::Error> {
        serde_json::from_str(text)
    }
}

impl ServerEvent {
    pub fn to_json(&self) -> String {
        serde_json::to_string(self).unwrap_or_else(|_| "{\"op\":\"error\",\"message\":\"serialize\"}".into())
    }
}
