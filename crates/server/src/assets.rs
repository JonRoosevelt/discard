use axum::body::Body;
use axum::http::{header, StatusCode, Uri};
use axum::response::{IntoResponse, Response};

#[derive(rust_embed::RustEmbed)]
#[folder = "$CARGO_MANIFEST_DIR/../../web/dist"]
pub struct Assets;

pub async fn serve(uri: Uri) -> Response {
    let path = uri.path().trim_start_matches('/');
    let path = if path.is_empty() { "index.html" } else { path };
    if let Some(res) = lookup(path) {
        return res;
    }
    if let Some(res) = lookup("index.html") {
        return res;
    }
    (StatusCode::NOT_FOUND, "not found").into_response()
}

fn lookup(path: &str) -> Option<Response> {
    let file = Assets::get(path)?;
    let mime = mime_guess::from_path(path).first_or_octet_stream();
    let cache = if path.starts_with("assets/") {
        "public, max-age=31536000, immutable"
    } else {
        "no-cache"
    };
    Some(
        Response::builder()
            .header(header::CONTENT_TYPE, mime.to_string())
            .header(header::CACHE_CONTROL, cache)
            .body(Body::from(file.data.into_owned()))
            .unwrap(),
    )
}
