use axum::body::Body;
use axum::extract::{Path, State};
use axum::http::{header, StatusCode};
use axum::response::Response;

use crate::error::{AppError, AppResult};
use crate::media::sniff_content_type;
use crate::state::AppState;

pub async fn get(
    State(state): State<AppState>,
    Path(hash): Path<String>,
) -> AppResult<Response> {
    let net = state.net().await;
    let bytes = net
        .get_blob(&hash)
        .await
        .map_err(|_| AppError::NotFound("blob not found".to_string()))?;
    let content_type = sniff_content_type(&bytes);
    Response::builder()
        .status(StatusCode::OK)
        .header(header::CONTENT_TYPE, content_type)
        .header(header::CACHE_CONTROL, "private, max-age=86400")
        .body(Body::from(bytes))
        .map_err(|e| AppError::Internal(e.to_string()))
}
