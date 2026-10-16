use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::Json;
use shared::{
    Channel, ChannelKind, CreateChannelRequest, CreateWorkspaceRequest, Id, JoinWorkspaceRequest,
    RenameRequest, Workspace,
};

use crate::error::{AppError, AppResult};
use crate::meta::{self, Target};
use crate::state::AppState;
use crate::store::WorkspaceRecord;
use crate::ticket::{WorkspaceTicket, TICKET_VERSION};

fn with_secret(mut wsp: Workspace, secret: String) -> Workspace {
    wsp.secret = Some(secret);
    wsp
}

pub fn clean_name(raw: &str) -> AppResult<String> {
    let name: String = raw.trim().chars().take(100).collect();
    if name.is_empty() {
        return Err(AppError::BadRequest("name is required".to_string()));
    }
    Ok(name)
}

fn not_found() -> AppError {
    AppError::NotFound("workspace not found".to_string())
}

async fn live_record(state: &AppState, id: &Id) -> AppResult<WorkspaceRecord> {
    state
        .store
        .record(id)
        .await
        .filter(|r| !r.deleted)
        .ok_or_else(not_found)
}

pub async fn list(State(state): State<AppState>) -> AppResult<Json<Vec<Workspace>>> {
    Ok(Json(state.store.list_with_secrets().await))
}

pub async fn create(
    State(state): State<AppState>,
    Json(req): Json<CreateWorkspaceRequest>,
) -> AppResult<Json<Workspace>> {
    let name = clean_name(&req.name)?;
    let net = state.net().await;
    let endpoint_ticket = net.endpoint_ticket();
    let id = uuid::Uuid::new_v4().to_string();
    let secret = hex::encode(rand::random::<[u8; 32]>());
    let owner_id = state.identity.public_id();
    let owner_name = state.identity.display_name();

    let channels = vec![
        Channel {
            id: uuid::Uuid::new_v4().to_string(),
            workspace_id: id.clone(),
            name: "geral".to_string(),
            kind: ChannelKind::Text,
            position: 0,
        },
        Channel {
            id: uuid::Uuid::new_v4().to_string(),
            workspace_id: id.clone(),
            name: "Geral".to_string(),
            kind: ChannelKind::Voice,
            position: 1,
        },
    ];

    let ticket = WorkspaceTicket {
        v: TICKET_VERSION,
        id: id.clone(),
        name: name.clone(),
        owner_id: owner_id.clone(),
        owner_name: owner_name.clone(),
        secret: secret.clone(),
        channels: channels.clone(),
        endpoint_ticket: endpoint_ticket.clone(),
    }
    .encode();

    let record = WorkspaceRecord {
        id,
        name,
        icon_url: None,
        owner_id,
        owner_name,
        secret: secret.clone(),
        ticket,
        channels,
        peers: Vec::new(),
        peer_tickets: Default::default(),
        owner_endpoint_ticket: Some(endpoint_ticket),
        clock: 0,
        deleted: false,
        ops: Vec::new(),
    };
    Ok(Json(with_secret(state.store.upsert(record).await, secret)))
}

pub async fn join(
    State(state): State<AppState>,
    Json(req): Json<JoinWorkspaceRequest>,
) -> AppResult<Json<Workspace>> {
    let ticket =
        WorkspaceTicket::decode(&req.ticket).map_err(|e| AppError::BadRequest(e.to_string()))?;
    // Re-joining must not reset the merged state (and its tombstones) back to the ticket's copy.
    if let Some(existing) = state.store.record(&ticket.id).await {
        if existing.deleted {
            return Err(AppError::BadRequest("this workspace was deleted".to_string()));
        }
        return Ok(Json(with_secret(existing.to_workspace(), existing.secret)));
    }
    let net = state.net().await;
    let owner_peer = net
        .add_peer_ticket(&ticket.endpoint_ticket)
        .map_err(|e| AppError::BadRequest(e.to_string()))?;

    let record = WorkspaceRecord {
        id: ticket.id.clone(),
        name: ticket.name.clone(),
        icon_url: None,
        owner_id: ticket.owner_id.clone(),
        owner_name: ticket.owner_name.clone(),
        secret: ticket.secret.clone(),
        ticket: req.ticket.clone(),
        channels: ticket.channels.clone(),
        peers: vec![owner_peer.to_string()],
        peer_tickets: Default::default(),
        owner_endpoint_ticket: Some(ticket.endpoint_ticket.clone()),
        clock: 0,
        deleted: false,
        ops: Vec::new(),
    };
    Ok(Json(with_secret(
        state.store.upsert(record).await,
        ticket.secret.clone(),
    )))
}

pub async fn get(
    State(state): State<AppState>,
    Path(id): Path<Id>,
) -> AppResult<Json<Workspace>> {
    let record = live_record(&state, &id).await?;
    Ok(Json(with_secret(
        record.to_workspace(),
        record.secret.clone(),
    )))
}

pub async fn ticket(
    State(state): State<AppState>,
    Path(id): Path<Id>,
) -> AppResult<Json<serde_json::Value>> {
    let record = live_record(&state, &id).await?;
    let net = state.net().await;
    let live = WorkspaceTicket {
        v: TICKET_VERSION,
        id: record.id.clone(),
        name: record.name.clone(),
        owner_id: record.owner_id.clone(),
        owner_name: record.owner_name.clone(),
        secret: record.secret.clone(),
        channels: record.channels.clone(),
        endpoint_ticket: net.endpoint_ticket(),
    };
    Ok(Json(serde_json::json!({ "ticket": live.encode() })))
}

pub async fn update(
    State(state): State<AppState>,
    Path(id): Path<Id>,
    Json(req): Json<RenameRequest>,
) -> AppResult<Json<Workspace>> {
    let name = clean_name(&req.name)?;
    live_record(&state, &id).await?;
    let record = meta::change(&state, &id, |r| {
        (r.name != name).then(|| Target::Workspace {
            name: name.clone(),
            deleted: false,
        })
    })
    .await
    .ok_or_else(not_found)?;
    Ok(Json(with_secret(record.to_workspace(), record.secret.clone())))
}

pub async fn delete(State(state): State<AppState>, Path(id): Path<Id>) -> AppResult<StatusCode> {
    live_record(&state, &id).await?;
    meta::change(&state, &id, |r| {
        Some(Target::Workspace {
            name: r.name.clone(),
            deleted: true,
        })
    })
    .await
    .ok_or_else(not_found)?;
    Ok(StatusCode::NO_CONTENT)
}

pub async fn leave(State(state): State<AppState>, Path(id): Path<Id>) -> AppResult<StatusCode> {
    live_record(&state, &id).await?;
    meta::leave(&state, &id).await;
    Ok(StatusCode::NO_CONTENT)
}

pub async fn create_channel(
    State(state): State<AppState>,
    Path(id): Path<Id>,
    Json(req): Json<CreateChannelRequest>,
) -> AppResult<Json<Channel>> {
    let name = clean_name(&req.name)?;
    live_record(&state, &id).await?;
    // The client picks the id so a retried or double-sent request lands on the same channel.
    let channel_id = match req.id {
        Some(client_id) if uuid::Uuid::parse_str(&client_id).is_err() => {
            return Err(AppError::BadRequest("channel id must be a uuid".to_string()))
        }
        Some(client_id) => client_id,
        None => uuid::Uuid::new_v4().to_string(),
    };
    let record = meta::change(&state, &id, |r| {
        if r.channels.iter().any(|c| c.id == channel_id) {
            return None;
        }
        let position = r.channels.iter().map(|c| c.position).max().unwrap_or(-1) + 1;
        Some(Target::Channel {
            channel: Channel {
                id: channel_id.clone(),
                workspace_id: r.id.clone(),
                name,
                kind: req.kind,
                position,
            },
            deleted: false,
        })
    })
    .await
    .ok_or_else(not_found)?;
    record
        .channels
        .into_iter()
        .find(|c| c.id == channel_id)
        .map(Json)
        .ok_or_else(|| AppError::BadRequest("channel id already in use".to_string()))
}
