use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::Json;
use shared::{Channel, Id, RenameRequest};

use crate::error::{AppError, AppResult};
use crate::meta::{self, Target};
use crate::routes::workspaces::clean_name;
use crate::state::AppState;

fn not_found() -> AppError {
    AppError::NotFound("channel not found".to_string())
}

pub async fn get(
    State(state): State<AppState>,
    Path(id): Path<Id>,
) -> AppResult<Json<Channel>> {
    state.store.channel(&id).await.map(Json).ok_or_else(not_found)
}

pub async fn update(
    State(state): State<AppState>,
    Path(id): Path<Id>,
    Json(req): Json<RenameRequest>,
) -> AppResult<Json<Channel>> {
    let name = clean_name(&req.name)?;
    let channel = state.store.channel(&id).await.ok_or_else(not_found)?;
    let record = meta::change(&state, &channel.workspace_id, |r| {
        let current = r.channels.iter().find(|c| c.id == id)?;
        (current.name != name).then(|| Target::Channel {
            channel: Channel {
                name: name.clone(),
                ..current.clone()
            },
            deleted: false,
        })
    })
    .await
    .ok_or_else(not_found)?;
    record
        .channels
        .into_iter()
        .find(|c| c.id == id)
        .map(Json)
        .ok_or_else(not_found)
}

pub async fn delete(State(state): State<AppState>, Path(id): Path<Id>) -> AppResult<StatusCode> {
    let channel = state.store.channel(&id).await.ok_or_else(not_found)?;
    meta::change(&state, &channel.workspace_id, |r| {
        let current = r.channels.iter().find(|c| c.id == id)?;
        Some(Target::Channel {
            channel: current.clone(),
            deleted: true,
        })
    })
    .await
    .ok_or_else(not_found)?;
    Ok(StatusCode::NO_CONTENT)
}
