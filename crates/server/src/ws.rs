use std::collections::HashSet;
use std::time::Duration;

use axum::extract::ws::{Message as WsMessage, WebSocket, WebSocketUpgrade};
use axum::extract::{Query, State};
use axum::response::{IntoResponse, Response};
use futures_util::{SinkExt, StreamExt};
use iroh_gossip::api::{Event, GossipReceiver};
use serde::Deserialize;
use shared::{ChannelKind, ClientEvent, Id, Message, ServerEvent, UserPublic, VoiceMember};
use tokio::sync::mpsc;

use crate::error::AppError;
use crate::hub::ConnId;
use crate::iroh_net::{Wire, OP_TIMEOUT};
use crate::state::AppState;
use crate::voice::Envelope;

#[derive(Debug, Deserialize)]
pub struct TokenQuery {
    pub token: String,
}

pub async fn handler(
    State(state): State<AppState>,
    Query(query): Query<TokenQuery>,
    ws: WebSocketUpgrade,
) -> Response {
    if query.token != state.session_token {
        return AppError::Unauthorized.into_response();
    }
    ws.on_upgrade(move |socket| handle_socket(socket, state))
}

async fn handle_socket(socket: WebSocket, state: AppState) {
    let (mut sender, mut receiver) = socket.split();
    let (tx, mut rx) = mpsc::unbounded_channel::<ServerEvent>();
    let conn = state.hub.register(tx);

    let workspaces = state.store.list().await;
    let workspace_ids: Vec<Id> = workspaces.iter().map(|w| w.id.clone()).collect();
    let ready = ServerEvent::Ready {
        identity: state.identity.shared(),
        workspaces,
        voice: state.voice.snapshot(),
        sounds: state.sounds.list().await,
    };
    if sender
        .send(WsMessage::Text(ready.to_json().into()))
        .await
        .is_err()
    {
        state.hub.unregister(conn);
        return;
    }

    for workspace_id in workspace_ids {
        ensure_workspace_subscription(&state, &workspace_id).await;
    }

    let mut heartbeat = tokio::time::interval(Duration::from_secs(60));
    heartbeat.tick().await;
    let mut joined_voice: HashSet<Id> = HashSet::new();

    loop {
        tokio::select! {
            _ = heartbeat.tick() => {
                tracing::debug!("heartbeat timeout for conn {}", conn);
                break;
            }
            incoming = receiver.next() => {
                match incoming {
                    Some(Ok(WsMessage::Text(text))) => {
                        heartbeat.reset();
                        match ClientEvent::parse(text.as_str()) {
                            Ok(event) => handle_client_event(&state, conn, event, &mut joined_voice).await,
                            Err(e) => state.hub.send_to_conn(
                                conn,
                                ServerEvent::Error { message: format!("invalid event: {e}") },
                            ),
                        }
                    }
                    Some(Ok(WsMessage::Close(_))) | None => break,
                    Some(Ok(_)) => {
                        heartbeat.reset();
                    }
                    Some(Err(_)) => break,
                }
            }
            outgoing = rx.recv() => {
                match outgoing {
                    Some(event) => {
                        if sender.send(WsMessage::Text(event.to_json().into())).await.is_err() {
                            break;
                        }
                    }
                    None => break,
                }
            }
        }
    }

    for channel_id in joined_voice {
        leave_voice(&state, &channel_id).await;
    }
    state.hub.unregister(conn);
}

async fn handle_client_event(
    state: &AppState,
    conn: ConnId,
    event: ClientEvent,
    joined_voice: &mut HashSet<Id>,
) {
    match event {
        ClientEvent::Ping => state.hub.send_to_conn(conn, ServerEvent::Pong),
        ClientEvent::Subscribe { channel_id } => {
            state.hub.subscribe(conn, channel_id.clone());
            ensure_subscription(state, &channel_id).await;
        }
        ClientEvent::Unsubscribe { channel_id } => state.hub.unsubscribe(conn, &channel_id),
        ClientEvent::SendMessage {
            channel_id,
            content,
            attachment_ids,
        } => {
            if state.store.channel(&channel_id).await.is_none() {
                state.hub.send_to_conn(
                    conn,
                    ServerEvent::Error {
                        message: "channel not found".to_string(),
                    },
                );
                return;
            }
            let message = build_message(state, &channel_id, &content, attachment_ids).await;
            publish(state, message).await;
        }
        ClientEvent::Typing { channel_id } => {
            let event = ServerEvent::Typing {
                channel_id: channel_id.clone(),
                user: local_user(state),
            };
            for other in state.hub.subscribers_of(&channel_id) {
                if other != conn {
                    state.hub.send_to_conn(other, event.clone());
                }
            }
        }
        ClientEvent::VoiceJoin {
            channel_id,
            muted,
            video,
        } => {
            let Some(channel) = state.store.channel(&channel_id).await else {
                state.hub.send_to_conn(
                    conn,
                    ServerEvent::Error {
                        message: "channel not found".to_string(),
                    },
                );
                return;
            };
            if channel.kind != ChannelKind::Voice {
                state.hub.send_to_conn(
                    conn,
                    ServerEvent::Error {
                        message: "not a voice channel".to_string(),
                    },
                );
                return;
            }
            let member = VoiceMember {
                user: local_user(state),
                muted,
                deafened: false,
                video,
                screen: false,
            };
            joined_voice.insert(channel_id.clone());
            let presence = state
                .voice
                .join(&channel_id, &channel.workspace_id, member.clone());
            ensure_workspace_subscription(state, &channel.workspace_id).await;
            let sync = Envelope::PresenceSync {
                channel_id: channel_id.clone(),
                from: state.identity.public_id(),
            };
            broadcast_envelope(state, &channel_id, sync).await;
            let announce = Envelope::PresenceJoin {
                channel_id: channel_id.clone(),
                member,
                ts_ms: now_ms(),
            };
            broadcast_envelope(state, &channel_id, announce).await;
            state
                .hub
                .broadcast_all(ServerEvent::VoicePresence { presence });
        }
        ClientEvent::VoiceUpdate {
            muted,
            deafened,
            video,
            screen,
        } => {
            let Some(channel_id) = state.voice.active_voice_channel() else {
                return;
            };
            let member = VoiceMember {
                user: local_user(state),
                muted,
                deafened,
                video,
                screen,
            };
            if state.voice.update_local(&channel_id, member.clone()).is_none() {
                return;
            }
            let envelope = Envelope::PresenceUpdate {
                channel_id: channel_id.clone(),
                member: member.clone(),
            };
            broadcast_envelope(state, &channel_id, envelope).await;
            state.hub.broadcast_all(ServerEvent::VoiceUpdated {
                channel_id,
                member,
            });
        }
        ClientEvent::VoiceLeave { channel_id } => {
            joined_voice.remove(&channel_id);
            leave_voice(state, &channel_id).await;
        }
        ClientEvent::Signal { to, data } => {
            let Some(channel_id) = state.voice.active_voice_channel() else {
                state.hub.send_to_conn(
                    conn,
                    ServerEvent::Error {
                        message: "not in a voice channel".to_string(),
                    },
                );
                return;
            };
            let envelope = Envelope::Signal {
                channel_id: channel_id.clone(),
                to,
                from: state.identity.public_id(),
                data,
            };
            broadcast_envelope(state, &channel_id, envelope).await;
        }
        ClientEvent::SoundPlay {
            channel_id,
            sound_id,
        } => {
            let Some(channel) = state.store.channel(&channel_id).await else {
                state.hub.send_to_conn(
                    conn,
                    ServerEvent::Error {
                        message: "channel not found".to_string(),
                    },
                );
                return;
            };
            if channel.kind != ChannelKind::Voice {
                state.hub.send_to_conn(
                    conn,
                    ServerEvent::Error {
                        message: "not a voice channel".to_string(),
                    },
                );
                return;
            }
            ensure_workspace_subscription(state, &channel.workspace_id).await;
            let by = state.identity.public_id();
            let envelope = Envelope::Sound {
                channel_id: channel_id.clone(),
                sound_id: sound_id.clone(),
                by: by.clone(),
            };
            broadcast_envelope(state, &channel_id, envelope).await;
            state.hub.broadcast_all(ServerEvent::SoundPlay {
                channel_id,
                sound_id,
                by,
            });
        }
    }
}

async fn leave_voice(state: &AppState, channel_id: &Id) {
    let user_id = state.identity.public_id();
    let Some(workspace_id) = state.voice.leave(channel_id, &user_id) else {
        return;
    };
    let envelope = Envelope::PresenceLeave {
        channel_id: channel_id.clone(),
        user_id: user_id.clone(),
    };
    broadcast_envelope(state, channel_id, envelope).await;
    state.hub.broadcast_all(ServerEvent::VoiceLeft {
        channel_id: channel_id.clone(),
        workspace_id,
        user_id,
    });
}

pub async fn broadcast_envelope(state: &AppState, channel_id: &Id, envelope: Envelope) {
    let Some(channel) = state.store.channel(channel_id).await else {
        return;
    };
    let workspace_id = channel.workspace_id;
    ensure_workspace_subscription(state, &workspace_id).await;
    let net = state.net().await;
    if !net.has_workspace(&workspace_id) {
        return;
    }
    match serde_json::to_vec(&envelope) {
        Ok(bytes) => {
            if let Err(e) = net.broadcast_workspace(&workspace_id, bytes).await {
                tracing::debug!("voice gossip broadcast failed: {e}");
            }
        }
        Err(e) => tracing::debug!("voice envelope encode failed: {e}"),
    }
}

pub async fn voice_tick(state: &AppState) {
    for workspace in state.store.list().await {
        ensure_workspace_subscription(state, &workspace.id).await;
    }
    for (channel_id, workspace_id, user_id) in state.voice.expire_stale() {
        state.hub.broadcast_all(ServerEvent::VoiceLeft {
            channel_id,
            workspace_id,
            user_id,
        });
    }
    for channel_id in state.voice.local_channels() {
        let Some(member) = state.voice.local_member(&channel_id) else {
            continue;
        };
        let envelope = Envelope::PresenceJoin {
            channel_id: channel_id.clone(),
            member,
            ts_ms: now_ms(),
        };
        broadcast_envelope(state, &channel_id, envelope).await;
    }
}

fn now_ms() -> i64 {
    chrono::Utc::now().timestamp_millis()
}

pub fn local_user(state: &AppState) -> UserPublic {
    UserPublic {
        id: state.identity.public_id(),
        display_name: state.identity.display_name(),
        avatar_url: None,
    }
}

pub async fn build_message(
    state: &AppState,
    channel_id: &Id,
    content: &str,
    attachment_ids: Vec<Id>,
) -> Message {
    let author = local_user(state);
    let created_at = chrono::Utc::now();
    let short: String = author.id.chars().take(8).collect();
    let attachments = state.retention.resolve_attachments(&attachment_ids);
    Message {
        id: format!("{}-{}", created_at.timestamp_millis(), short),
        channel_id: channel_id.clone(),
        author,
        content: content.to_string(),
        created_at,
        attachments,
    }
}

pub async fn publish(state: &AppState, message: Message) {
    state.retention.insert_message(message.clone());
    ensure_subscription(state, &message.channel_id).await;
    let net = state.net().await;
    if let Ok(bytes) = serde_json::to_vec(&Wire::Message {
        message: message.clone(),
    }) {
        if let Err(e) = net.broadcast_channel(&message.channel_id, bytes).await {
            tracing::debug!("gossip broadcast failed: {e}");
        }
    }
    state.hub.broadcast_message(&message);
}

pub async fn ensure_subscription(state: &AppState, channel_id: &Id) {
    let net = state.net().await;
    if net.has_channel(channel_id) {
        return;
    }
    let Some((workspace_id, secret)) = state.store.secret_for_channel(channel_id).await else {
        tracing::debug!("channel {} is not part of a known workspace", channel_id);
        return;
    };
    let bootstrap: Vec<_> = state
        .store
        .peers(&workspace_id)
        .await
        .iter()
        .filter_map(|peer| net.parse_peer(peer))
        .collect();
    match net.subscribe_channel(channel_id, &secret, bootstrap).await {
        Ok((_sender, receiver)) => {
            spawn_receiver(state.clone(), channel_id.clone(), workspace_id, receiver)
        }
        Err(e) => tracing::debug!("could not subscribe to channel {}: {e}", channel_id),
    }
}

pub async fn ensure_workspace_subscription(state: &AppState, workspace_id: &Id) {
    let net = state.net().await;
    if net.has_workspace(workspace_id) {
        return;
    }
    let Some(record) = state.store.record(workspace_id).await else {
        return;
    };
    let bootstrap: Vec<_> = state
        .store
        .peers(workspace_id)
        .await
        .iter()
        .filter_map(|peer| net.parse_peer(peer))
        .collect();
    match net
        .subscribe_workspace(workspace_id, &record.secret, bootstrap)
        .await
    {
        Ok((_sender, receiver)) => {
            spawn_workspace_receiver(state.clone(), workspace_id.clone(), receiver)
        }
        Err(e) => tracing::debug!("could not subscribe to workspace {}: {e}", workspace_id),
    }
}

fn spawn_receiver(
    state: AppState,
    channel_id: Id,
    workspace_id: Id,
    mut receiver: GossipReceiver,
) {
    tokio::spawn(async move {
        if tokio::time::timeout(OP_TIMEOUT, receiver.joined()).await.is_err() {
            tracing::debug!("gossip join timed out for channel {channel_id}");
        }
        loop {
            match tokio::time::timeout(Duration::from_secs(120), receiver.next()).await {
                Ok(Some(Ok(Event::Received(msg)))) => {
                    handle_payload(&state, &channel_id, &workspace_id, msg.content.to_vec()).await;
                }
                Ok(Some(Ok(Event::NeighborUp(peer)))) => {
                    state.store.add_peer(&workspace_id, &peer.to_string()).await;
                }
                Ok(Some(Ok(_))) => {}
                Ok(Some(Err(e))) => {
                    tracing::debug!("gossip stream error: {e}");
                    break;
                }
                Ok(None) => break,
                Err(_) => continue,
            }
        }
    });
}

fn spawn_workspace_receiver(state: AppState, workspace_id: Id, mut receiver: GossipReceiver) {
    tokio::spawn(async move {
        if tokio::time::timeout(OP_TIMEOUT, receiver.joined()).await.is_err() {
            tracing::debug!("gossip join timed out for workspace {workspace_id}");
        }
        loop {
            match tokio::time::timeout(Duration::from_secs(120), receiver.next()).await {
                Ok(Some(Ok(Event::Received(msg)))) => {
                    handle_workspace_payload(&state, &workspace_id, msg.content.to_vec()).await;
                }
                Ok(Some(Ok(Event::NeighborUp(peer)))) => {
                    state.store.add_peer(&workspace_id, &peer.to_string()).await;
                }
                Ok(Some(Ok(_))) => {}
                Ok(Some(Err(e))) => {
                    tracing::debug!("gossip stream error: {e}");
                    break;
                }
                Ok(None) => break,
                Err(_) => continue,
            }
        }
    });
}

async fn handle_payload(state: &AppState, channel_id: &Id, workspace_id: &Id, bytes: Vec<u8>) {
    if let Ok(Wire::Message { message }) = serde_json::from_slice::<Wire>(&bytes) {
        handle_message(state, channel_id, workspace_id, message).await;
    }
}

async fn handle_workspace_payload(state: &AppState, workspace_id: &Id, bytes: Vec<u8>) {
    if let Ok(envelope) = serde_json::from_slice::<Envelope>(&bytes) {
        handle_voice(state, workspace_id, envelope).await;
    }
}

async fn handle_message(state: &AppState, channel_id: &Id, workspace_id: &Id, message: Message) {
    if &message.channel_id != channel_id {
        return;
    }
    if state.retention.seen(&message.id) {
        return;
    }
    let net = state.net().await;
    if let Some(peer) = net.parse_peer(&message.author.id) {
        state.store.add_peer(workspace_id, &message.author.id).await;
        for attachment in &message.attachments {
            match net.download(&attachment.id, peer).await {
                Ok(()) => {
                    if let Ok(tag) = net.tag_blob(&attachment.id).await {
                        state
                            .retention
                            .register_blob(attachment.id.clone(), tag);
                    }
                }
                Err(e) => tracing::debug!("blob download failed: {e}"),
            }
        }
    }
    if state.retention.insert_message(message.clone()) {
        state.hub.broadcast_message(&message);
    }
}

async fn handle_voice(state: &AppState, workspace_id: &Id, envelope: Envelope) {
    let channel_id = envelope.channel_id().clone();
    match state.store.channel(&channel_id).await {
        Some(channel) if &channel.workspace_id == workspace_id => {}
        _ => return,
    }
    match envelope {
        Envelope::PresenceJoin { member, .. } => {
            let presence = state.voice.remote_join(&channel_id, workspace_id, member);
            state
                .hub
                .broadcast_all(ServerEvent::VoiceJoined { presence });
        }
        Envelope::PresenceUpdate { member, .. } => {
            state
                .voice
                .remote_update(&channel_id, workspace_id, member.clone());
            state.hub.broadcast_all(ServerEvent::VoiceUpdated {
                channel_id: channel_id.clone(),
                member,
            });
        }
        Envelope::PresenceLeave { user_id, .. } => {
            if let Some(workspace_id) = state.voice.remote_leave(&channel_id, &user_id) {
                state.hub.broadcast_all(ServerEvent::VoiceLeft {
                    channel_id: channel_id.clone(),
                    workspace_id,
                    user_id,
                });
            }
        }
        Envelope::PresenceSync { .. } => {
            let members = state.voice.members_of(&channel_id);
            if !members.is_empty() {
                let state_envelope = Envelope::PresenceState {
                    channel_id: channel_id.clone(),
                    members,
                };
                broadcast_envelope(state, &channel_id, state_envelope).await;
            }
        }
        Envelope::PresenceState { members, .. } => {
            state
                .voice
                .merge_state(&channel_id, workspace_id, members);
            if let Some(presence) = state.voice.presence(&channel_id) {
                state
                    .hub
                    .broadcast_all(ServerEvent::VoicePresence { presence });
            }
        }
        Envelope::Signal { to, from, data, .. } => {
            if to == state.identity.public_id() {
                state.hub.broadcast_all(ServerEvent::Signal { from, data });
            }
        }
        Envelope::Sound { sound_id, by, .. } => {
            if state.voice.local_member(&channel_id).is_some() {
                state.hub.broadcast_all(ServerEvent::SoundPlay {
                    channel_id: channel_id.clone(),
                    sound_id,
                    by,
                });
            }
        }
    }
}
