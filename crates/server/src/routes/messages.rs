use axum::extract::{Path, State};
use axum::Json;
use shared::{Id, Message, SendMessageRequest};

use crate::error::{AppError, AppResult};
use crate::state::AppState;
use crate::ws;

pub async fn list(
    State(state): State<AppState>,
    Path(channel_id): Path<Id>,
) -> AppResult<Json<Vec<Message>>> {
    if state.store.channel(&channel_id).await.is_none() {
        return Err(AppError::NotFound("channel not found".to_string()));
    }
    Ok(Json(state.retention.messages(&channel_id)))
}

pub async fn create(
    State(state): State<AppState>,
    Path(channel_id): Path<Id>,
    Json(req): Json<SendMessageRequest>,
) -> AppResult<Json<Message>> {
    if state.store.channel(&channel_id).await.is_none() {
        return Err(AppError::NotFound("channel not found".to_string()));
    }
    let message =
        ws::build_message(&state, &channel_id, &req.content, req.attachment_ids).await;
    ws::publish(&state, message.clone()).await;
    Ok(Json(message))
}
