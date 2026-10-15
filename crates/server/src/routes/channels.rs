use axum::extract::{Path, State};
use axum::Json;
use shared::{Channel, Id};

use crate::error::{AppError, AppResult};
use crate::state::AppState;

pub async fn get(
    State(state): State<AppState>,
    Path(id): Path<Id>,
) -> AppResult<Json<Channel>> {
    state
        .store
        .channel(&id)
        .await
        .map(Json)
        .ok_or_else(|| AppError::NotFound("channel not found".to_string()))
}
