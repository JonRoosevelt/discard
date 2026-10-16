use axum::extract::{Multipart, Path, State};
use axum::http::StatusCode;
use axum::Json;
use serde::Deserialize;
use shared::Sound;

use crate::error::{AppError, AppResult};
use crate::media::sniff_content_type;
use crate::sounds::MAX_BYTES;
use crate::state::AppState;
use crate::ws;

#[derive(Deserialize)]
pub struct Edit {
    name: String,
    emoji: String,
}

pub async fn list(State(state): State<AppState>) -> AppResult<Json<Vec<Sound>>> {
    Ok(Json(state.sounds.list().await))
}

fn is_audio(data: &[u8], filename: &str) -> bool {
    let sniffed = sniff_content_type(data);
    if sniffed.starts_with("audio/") {
        return true;
    }
    // sniff_content_type doesn't know flac/aac/m4a/webm, so trust the extension for unknowns
    (sniffed == "application/octet-stream" || sniffed == "video/mp4")
        && mime_guess::from_path(filename)
            .first()
            .is_some_and(|m| m.type_() == "audio")
}

fn clean(name: &str, emoji: &str) -> AppResult<(String, String)> {
    let name = name.trim();
    let emoji = emoji.trim();
    if name.is_empty() || name.chars().count() > 32 || emoji.len() > 16 {
        return Err(AppError::BadRequest("invalid name or emoji".to_string()));
    }
    Ok((name.to_string(), emoji.to_string()))
}

pub async fn create(
    State(state): State<AppState>,
    mut multipart: Multipart,
) -> AppResult<Json<Sound>> {
    let mut data = Vec::new();
    let mut filename = String::new();
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
                filename = field.file_name().unwrap_or_default().to_string();
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
    if data.len() > MAX_BYTES {
        return Err(AppError::BadRequest("sound must be 1 MB or smaller".to_string()));
    }
    if !is_audio(&data, &filename) {
        return Err(AppError::BadRequest("file must be audio".to_string()));
    }
    let name = if name.trim().is_empty() { "sound" } else { &name };
    let (name, emoji) = clean(name, &emoji)?;

    let net = state.net().await;
    let (hash, _tag) = net
        .add_blob(data)
        .await
        .map_err(|e| AppError::Internal(e.to_string()))?;
    let sound = Sound {
        id: hex::encode(rand::random::<[u8; 8]>()),
        name,
        url: format!("/files/{hash}"),
        emoji,
        builtin: false,
        owner: state.identity.public_id(),
        hash,
        version: 1,
        editor: state.identity.public_id(),
        deleted: false,
    };
    state.sounds.merge(sound.clone(), None).await;
    ws::announce_sound(&state, &sound).await;
    Ok(Json(sound))
}

fn not_found() -> AppError {
    AppError::NotFound("sound not found".to_string())
}

pub async fn update(
    State(state): State<AppState>,
    Path(id): Path<String>,
    Json(edit): Json<Edit>,
) -> AppResult<Json<Sound>> {
    let (name, emoji) = clean(&edit.name, &edit.emoji)?;
    let sound = state
        .sounds
        .edit(&id, &state.identity.public_id(), name, emoji)
        .await
        .ok_or_else(not_found)?;
    ws::announce_sound(&state, &sound).await;
    Ok(Json(sound))
}

pub async fn delete(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> AppResult<StatusCode> {
    let sound = state
        .sounds
        .delete(&id, &state.identity.public_id())
        .await
        .ok_or_else(not_found)?;
    ws::untag_unused(&state, &sound.hash).await;
    ws::announce_sound(&state, &sound).await;
    Ok(StatusCode::NO_CONTENT)
}
