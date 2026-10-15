pub mod channels;
pub mod files;
pub mod identity;
pub mod messages;
pub mod sounds;
pub mod uploads;
pub mod workspaces;

use axum::routing::{get, post};
use axum::{middleware, Json, Router};
use tower_http::cors::CorsLayer;
use tower_http::trace::TraceLayer;

#[cfg(not(feature = "embed-ui"))]
use tower_http::services::{ServeDir, ServeFile};

use crate::auth::require_auth;
use crate::state::AppState;
use crate::ws;

pub fn router(state: AppState) -> Router {
    #[cfg(not(feature = "embed-ui"))]
    let dist = state.config.web_dist();

    let api = Router::new()
        .route("/health", get(health))
        .route("/identity", get(identity::get).post(identity::create))
        .route("/identity/backup", get(identity::backup))
        .route("/identity/import", post(identity::import))
        .route("/workspaces", get(workspaces::list).post(workspaces::create))
        .route("/workspaces/join", post(workspaces::join))
        .route("/workspaces/{id}", get(workspaces::get))
        .route("/workspaces/{id}/ticket", get(workspaces::ticket))
        .route(
            "/workspaces/{id}/channels",
            post(workspaces::create_channel),
        )
        .route("/channels/{id}", get(channels::get))
        .route(
            "/channels/{id}/messages",
            get(messages::list).post(messages::create),
        )
        .route("/upload", post(uploads::upload))
        .route("/sounds", get(sounds::list).post(sounds::create))
        .layer(middleware::from_fn_with_state(
            state.clone(),
            require_auth,
        ));

    let app = Router::new()
        .nest("/api", api)
        .route("/ws", get(ws::handler))
        .route("/files/{hash}", get(files::get))
        .layer(TraceLayer::new_for_http())
        .layer(CorsLayer::permissive())
        .with_state(state);

    #[cfg(feature = "embed-ui")]
    let app = app.fallback(crate::assets::serve);

    #[cfg(not(feature = "embed-ui"))]
    let app = {
        let index = dist.join("index.html");
        app.fallback_service(ServeDir::new(dist).fallback(ServeFile::new(index)))
    };

    app
}

async fn health() -> Json<serde_json::Value> {
    Json(serde_json::json!({ "ok": true }))
}
