use axum::extract::{Multipart, State};
use axum::Json;
use shared::Attachment;

use crate::error::{AppError, AppResult};
use crate::media::{image_dimensions, sniff_content_type};
use crate::state::AppState;

pub async fn upload(
    State(state): State<AppState>,
    mut multipart: Multipart,
) -> AppResult<Json<Attachment>> {
    while let Some(field) = multipart
        .next_field()
        .await
        .map_err(|e| AppError::BadRequest(e.to_string()))?
    {
        let filename = field
            .file_name()
            .map(|s| s.to_string())
            .unwrap_or_else(|| "arquivo".to_string());
        let data = field
            .bytes()
            .await
            .map_err(|e| AppError::BadRequest(e.to_string()))?
            .to_vec();
        if data.is_empty() {
            continue;
        }
        let content_type = data_mime(&data, &filename);
        let net = state.net().await;
        let (hash, tag) = net
            .add_blob(data.clone())
            .await
            .map_err(|e| AppError::Internal(e.to_string()))?;
        let (width, height) = image_dimensions(&data);
        let attachment = Attachment {
            id: hash.clone(),
            filename,
            url: format!("/files/{hash}"),
            content_type,
            size: data.len() as i64,
            width,
            height,
        };
        state.retention.register_upload(attachment.clone(), tag);
        return Ok(Json(attachment));
    }
    Err(AppError::BadRequest("no file field".to_string()))
}

pub fn data_mime(data: &[u8], filename: &str) -> String {
    let sniffed = sniff_content_type(data);
    if sniffed != "application/octet-stream" {
        return sniffed;
    }
    mime_guess::from_path(filename)
        .first_or_octet_stream()
        .to_string()
}
