#[cfg(feature = "embed-ui")]
mod assets;
mod auth;
mod config;
mod error;
mod hub;
mod identity;
mod iroh_net;
mod media;
mod retention;
mod routes;
mod sounds;
mod state;
mod store;
mod ticket;
mod turn;
mod voice;
mod ws;

use std::sync::Arc;
use std::time::Duration;

use tokio::sync::RwLock;
use tracing_subscriber::EnvFilter;

use crate::config::Config;
use crate::identity::LocalIdentity;
use crate::iroh_net::IrohNet;
use crate::state::AppState;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info")),
        )
        .init();

    let config = Arc::new(Config::from_env());
    std::fs::create_dir_all(&config.data_dir)?;

    let identity = Arc::new(LocalIdentity::load_or_create(&config.data_dir)?);
    let turn = turn::Turn::start().await?;
    let net = IrohNet::start(
        identity.secret_key(),
        config.offline,
        config.no_relay,
        config.relay_only,
        config.iroh_bind,
        &config.blobs_dir(),
        turn.clone(),
    )
    .await?;

    let store = store::Store::load(config.workspaces_path());
    let retention = Arc::new(retention::Retention::new());
    let hub = hub::Hub::new();
    let voice = Arc::new(voice::Voice::new());
    let sounds = sounds::SoundStore::load(config.data_dir.join("sounds.json"), &identity.public_id());
    let session_token = config
        .token_override
        .clone()
        .unwrap_or_else(|| hex::encode(rand::random::<[u8; 32]>()));

    let state = AppState {
        identity: identity.clone(),
        net: Arc::new(RwLock::new(net)),
        store,
        retention: retention.clone(),
        hub,
        voice,
        turn,
        sounds,
        config: config.clone(),
        session_token: session_token.clone(),
    };

    let gc_state = state.clone();
    tokio::spawn(async move {
        let mut ticker = tokio::time::interval(Duration::from_secs(300));
        loop {
            ticker.tick().await;
            let tags = gc_state.retention.purge();
            if tags.is_empty() {
                continue;
            }
            let net = gc_state.net().await;
            for tag in tags {
                if let Err(e) = net.delete_blob_tag(tag).await {
                    tracing::debug!("blob gc failed: {e}");
                }
            }
        }
    });

    let voice_state = state.clone();
    tokio::spawn(async move {
        let mut ticker = tokio::time::interval(voice::HEARTBEAT);
        ticker.tick().await;
        loop {
            ticker.tick().await;
            ws::voice_tick(&voice_state).await;
        }
    });

    let app = routes::router(state);
    let listener = tokio::net::TcpListener::bind(&config.addr).await?;
    let url = format!("http://{}/?k={}", config.display_host(), session_token);
    tracing::info!("discard (local node) listening on http://{}", config.addr);
    tracing::info!("endpoint id = {}", identity.public_id());
    println!("\n  Discard pronto: {url}\n");

    if !config.no_open {
        open_browser(&url);
    }

    axum::serve(listener, app).await?;
    Ok(())
}

fn open_browser(url: &str) {
    #[cfg(target_os = "macos")]
    let cmd = "open";
    #[cfg(all(unix, not(target_os = "macos")))]
    let cmd = "xdg-open";
    #[cfg(not(unix))]
    let cmd = "";

    if cmd.is_empty() {
        return;
    }
    let _ = std::process::Command::new(cmd)
        .arg(url)
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .spawn();
}
