use std::sync::Arc;

use tokio::sync::RwLock;

use crate::config::Config;
use crate::hub::Hub;
use crate::identity::LocalIdentity;
use crate::iroh_net::IrohNet;
use crate::retention::Retention;
use crate::sounds::SoundStore;
use crate::store::Store;
use crate::turn::Turn;
use crate::voice::Voice;

#[derive(Clone)]
pub struct AppState {
    pub identity: Arc<LocalIdentity>,
    pub net: Arc<RwLock<Arc<IrohNet>>>,
    pub store: Arc<Store>,
    pub retention: Arc<Retention>,
    pub hub: Arc<Hub>,
    pub voice: Arc<Voice>,
    pub turn: Arc<Turn>,
    pub sounds: Arc<SoundStore>,
    pub config: Arc<Config>,
    pub session_token: String,
}

impl AppState {
    pub async fn net(&self) -> Arc<IrohNet> {
        self.net.read().await.clone()
    }

    pub async fn rebuild_net(&self) -> anyhow::Result<Arc<IrohNet>> {
        let net = IrohNet::start(
            self.identity.secret_key(),
            self.config.offline,
            self.config.no_relay,
            self.config.iroh_bind,
            &self.config.blobs_dir(),
            self.turn.clone(),
        )
        .await?;
        let old = {
            let mut guard = self.net.write().await;
            std::mem::replace(&mut *guard, net.clone())
        };
        old.shutdown().await;
        Ok(net)
    }
}
