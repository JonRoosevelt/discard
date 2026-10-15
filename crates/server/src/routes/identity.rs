use axum::extract::State;
use axum::http::header;
use axum::response::{IntoResponse, Response};
use axum::Json;
use shared::{CreateIdentityRequest, Identity, ImportIdentityRequest};

use crate::error::{AppError, AppResult};
use crate::state::AppState;

pub async fn get(State(state): State<AppState>) -> AppResult<Response> {
    if !state.identity.exists() {
        return Err(AppError::NotFound("identity not created".to_string()));
    }
    Ok(Json(state.identity.shared()).into_response())
}

pub async fn create(
    State(state): State<AppState>,
    Json(req): Json<CreateIdentityRequest>,
) -> AppResult<Json<Identity>> {
    let name = req.display_name.trim();
    if name.is_empty() {
        return Err(AppError::BadRequest("display_name is required".to_string()));
    }
    state.identity.set_display_name(name)?;
    Ok(Json(state.identity.shared()))
}

pub async fn backup(State(state): State<AppState>) -> AppResult<Response> {
    if !state.identity.exists() {
        return Err(AppError::NotFound("identity not created".to_string()));
    }
    let body = state.identity.backup_hex();
    Ok((
        [
            (header::CONTENT_TYPE, "text/plain; charset=utf-8"),
            (
                header::CONTENT_DISPOSITION,
                "attachment; filename=\"cadence-identity-backup.txt\"",
            ),
        ],
        body,
    )
        .into_response())
}

pub async fn import(
    State(state): State<AppState>,
    Json(req): Json<ImportIdentityRequest>,
) -> AppResult<Json<Identity>> {
    state
        .identity
        .import(&req.backup)
        .map_err(|e| AppError::BadRequest(e.to_string()))?;
    if state.identity.display_name().is_empty() {
        let _ = state.identity.set_display_name("Cadence");
    }
    state
        .rebuild_net()
        .await
        .map_err(|e| AppError::Internal(e.to_string()))?;
    Ok(Json(state.identity.shared()))
}
