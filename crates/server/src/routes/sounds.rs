use axum::extract::{Multipart, State};
use axum::Json;
use shared::Sound;

use crate::error::{AppError, AppResult};
use crate::state::AppState;

pub async fn list(State(state): State<AppState>) -> AppResult<Json<Vec<Sound>>> {
    Ok(Json(state.sounds.list().await))
}

pub async fn create(
    State(state): State<AppState>,
    mut multipart: Multipart,
) -> AppResult<Json<Sound>> {
    let mut data = Vec::new();
    let mut name = String::new();
    let mut emoji = String::new();

    while let Some(field) = multipart
        .next_field()
        .await
        .map_err(|e| AppError::BadRequest(e.to_string()))?
    {
        let field_name = field.name().map(|s| s.to_string()).unwrap_or_default();
        match field_name.as_str() {
            "name" => {
                name = field
                    .text()
                    .await
                    .map_err(|e| AppError::BadRequest(e.to_string()))?
            }
            "emoji" => {
                emoji = field
                    .text()
                    .await
                    .map_err(|e| AppError::BadRequest(e.to_string()))?
            }
            "file" => {
                data = field
                    .bytes()
                    .await
                    .map_err(|e| AppError::BadRequest(e.to_string()))?
                    .to_vec()
            }
            _ => {}
        }
    }

    if data.is_empty() {
        return Err(AppError::BadRequest("file is required".to_string()));
    }
    let name = if name.trim().is_empty() {
        "sound".to_string()
    } else {
        name.trim().to_string()
    };

    let net = state.net().await;
    let (hash, _tag) = net
        .add_blob(data)
        .await
        .map_err(|e| AppError::Internal(e.to_string()))?;
    let sound = Sound {
        id: hash.clone(),
        name,
        url: format!("/files/{hash}"),
        emoji,
        builtin: false,
    };
    Ok(Json(state.sounds.add(sound).await))
}
