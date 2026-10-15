use axum::extract::{Path, State};
use axum::Json;
use shared::{
    Channel, ChannelKind, CreateChannelRequest, CreateWorkspaceRequest, Id, JoinWorkspaceRequest,
    Workspace,
};

use crate::error::{AppError, AppResult};
use crate::state::AppState;
use crate::store::WorkspaceRecord;
use crate::ticket::{WorkspaceTicket, TICKET_VERSION};

pub async fn list(State(state): State<AppState>) -> AppResult<Json<Vec<Workspace>>> {
    Ok(Json(state.store.list().await))
}

pub async fn create(
    State(state): State<AppState>,
    Json(req): Json<CreateWorkspaceRequest>,
) -> AppResult<Json<Workspace>> {
    let name = req.name.trim().to_string();
    if name.is_empty() {
        return Err(AppError::BadRequest("name is required".to_string()));
    }
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
        secret,
        ticket,
        channels,
        peers: Vec::new(),
        owner_endpoint_ticket: Some(endpoint_ticket),
    };
    Ok(Json(state.store.upsert(record).await))
}

pub async fn join(
    State(state): State<AppState>,
    Json(req): Json<JoinWorkspaceRequest>,
) -> AppResult<Json<Workspace>> {
    let ticket =
        WorkspaceTicket::decode(&req.ticket).map_err(|e| AppError::BadRequest(e.to_string()))?;
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
        owner_endpoint_ticket: Some(ticket.endpoint_ticket.clone()),
    };
    Ok(Json(state.store.upsert(record).await))
}

pub async fn get(
    State(state): State<AppState>,
    Path(id): Path<Id>,
) -> AppResult<Json<Workspace>> {
    state
        .store
        .get(&id)
        .await
        .map(Json)
        .ok_or_else(|| AppError::NotFound("workspace not found".to_string()))
}

pub async fn ticket(
    State(state): State<AppState>,
    Path(id): Path<Id>,
) -> AppResult<Json<serde_json::Value>> {
    let record = state
        .store
        .record(&id)
        .await
        .ok_or_else(|| AppError::NotFound("workspace not found".to_string()))?;
    Ok(Json(serde_json::json!({ "ticket": record.ticket })))
}

pub async fn create_channel(
    State(state): State<AppState>,
    Path(id): Path<Id>,
    Json(req): Json<CreateChannelRequest>,
) -> AppResult<Json<Channel>> {
    let name = req.name.trim();
    if name.is_empty() {
        return Err(AppError::BadRequest("name is required".to_string()));
    }
    if state.store.get(&id).await.is_none() {
        return Err(AppError::NotFound("workspace not found".to_string()));
    }
    let channel = state
        .store
        .add_channel(&id, name, req.kind)
        .await
        .map_err(|e| AppError::Internal(e.to_string()))?;
    Ok(Json(channel))
}
