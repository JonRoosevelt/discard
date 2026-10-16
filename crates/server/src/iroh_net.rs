use std::path::Path;
use std::sync::Arc;
use std::time::{Duration, Instant};

use anyhow::Context;
use dashmap::DashMap;
use iroh::address_lookup::memory::MemoryLookup;
use iroh::endpoint::presets;
use iroh::protocol::Router;
use iroh::{Endpoint, EndpointAddr, EndpointId, SecretKey};
use iroh_blobs::store::fs::FsStore;
use iroh_blobs::BlobsProtocol;
use iroh_gossip::api::{GossipReceiver, GossipSender};
use iroh_gossip::{Gossip, TopicId};
use iroh_tickets::endpoint::EndpointTicket;
use serde::{Deserialize, Serialize};
use shared::{Id, Message};

use crate::turn::Turn;

pub const OP_TIMEOUT: Duration = Duration::from_secs(20);

// iroh-gossip's default is 4096; an oversized frame makes its send loop exit without closing
// the connection, so the actor blocks on the full per-peer queue and every topic goes dead.
// SDP offers with audio+video are ~5-10 KB. All peers must agree: the reader rejects larger frames.
const GOSSIP_MAX_MESSAGE_SIZE: usize = 64 * 1024;
const MAX_PAYLOAD: usize = GOSSIP_MAX_MESSAGE_SIZE - 1024;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "t", rename_all = "snake_case")]
pub enum Wire {
    Message { message: Message },
}

pub struct IrohNet {
    endpoint: Endpoint,
    gossip: Gossip,
    store: FsStore,
    lookup: MemoryLookup,
    router: Router,
    subscriptions: DashMap<Id, GossipSender>,
    workspace_subscriptions: DashMap<Id, GossipSender>,
    pending: DashMap<String, Pending>,
    seen: DashMap<String, Instant>,
}

struct Pending {
    bytes: Vec<u8>,
    workspace_id: Id,
    attempts: u32,
    last: Instant,
}

impl IrohNet {
    pub async fn start(
        secret_key: SecretKey,
        offline: bool,
        no_relay: bool,
        bind_addr: Option<std::net::SocketAddr>,
        blobs_dir: &Path,
        turn: Arc<Turn>,
    ) -> anyhow::Result<Arc<Self>> {
        let lookup = MemoryLookup::new();
        let mut builder = if offline {
            Endpoint::builder(presets::Minimal)
        } else if no_relay {
            Endpoint::builder(presets::N0DisableRelay)
        } else {
            Endpoint::builder(presets::N0)
        };
        if let Some(addr) = bind_addr {
            builder = builder
                .bind_addr(addr)
                .map_err(|e| anyhow::anyhow!("invalid iroh bind addr: {e}"))?;
        }
        let endpoint = builder
            .address_lookup(lookup.clone())
            .secret_key(secret_key)
            .bind()
            .await
            .context("bind iroh endpoint")?;

        let gossip = Gossip::builder()
            .max_message_size(GOSSIP_MAX_MESSAGE_SIZE)
            .spawn(endpoint.clone());

        std::fs::create_dir_all(blobs_dir)?;
        let store = FsStore::load(blobs_dir)
            .await
            .context("load blob store")?;

        let router = Router::builder(endpoint.clone())
            .accept(iroh_gossip::ALPN, gossip.clone())
            .accept(iroh_blobs::ALPN, BlobsProtocol::new(&store, None))
            .accept(crate::turn::ALPN, turn.clone())
            .spawn();
        turn.set_endpoint(endpoint.clone());

        let net = Arc::new(Self {
            endpoint,
            gossip,
            store,
            lookup,
            router,
            subscriptions: DashMap::new(),
            workspace_subscriptions: DashMap::new(),
            pending: DashMap::new(),
            seen: DashMap::new(),
        });
        tokio::spawn(net.clone().retry_loop());
        Ok(net)
    }

    pub fn endpoint_ticket(&self) -> String {
        EndpointTicket::new(self.endpoint.addr()).to_string()
    }

    pub fn endpoint_id(&self) -> EndpointId {
        self.endpoint.id()
    }

    #[allow(dead_code)]
    pub fn add_peer_addr(&self, addr: EndpointAddr) {
        self.lookup.add_endpoint_info(addr);
    }

    pub fn add_peer_ticket(&self, ticket: &str) -> anyhow::Result<EndpointId> {
        let parsed: EndpointTicket = ticket
            .parse()
            .map_err(|e| anyhow::anyhow!("invalid endpoint ticket: {e}"))?;
        let addr = parsed.endpoint_addr().clone();
        let id = addr.id;
        self.lookup.add_endpoint_info(addr);
        Ok(id)
    }

    pub fn parse_peer(&self, id: &str) -> Option<EndpointId> {
        id.parse::<EndpointId>().ok()
    }

    pub async fn subscribe(
        &self,
        topic: TopicId,
        bootstrap: Vec<EndpointId>,
    ) -> anyhow::Result<(GossipSender, GossipReceiver)> {
        let sub = tokio::time::timeout(OP_TIMEOUT, self.gossip.subscribe(topic, bootstrap))
            .await
            .context("gossip subscribe timeout")?
            .context("gossip subscribe failed")?;
        Ok(sub.split())
    }

    pub fn has_channel(&self, channel_id: &Id) -> bool {
        self.subscriptions.contains_key(channel_id)
    }

    pub fn channel_sender(&self, channel_id: &Id) -> Option<GossipSender> {
        self.subscriptions.get(channel_id).map(|s| s.clone())
    }

    pub async fn subscribe_channel(
        &self,
        channel_id: &Id,
        secret_hex: &str,
        bootstrap: Vec<EndpointId>,
    ) -> anyhow::Result<(GossipSender, GossipReceiver)> {
        if self.channel_sender(channel_id).is_some() {
            anyhow::bail!("already subscribed to channel {channel_id}");
        }
        let topic = crate::ticket::topic_id(secret_hex, channel_id)?;
        let parts = self.subscribe(topic, bootstrap).await?;
        self.subscriptions
            .insert(channel_id.clone(), parts.0.clone());
        Ok(parts)
    }

    #[allow(dead_code)]
    pub async fn broadcast_channel(&self, channel_id: &Id, bytes: Vec<u8>) -> anyhow::Result<()> {
        if bytes.len() > MAX_PAYLOAD {
            anyhow::bail!("payload too large for gossip: {} bytes", bytes.len());
        }
        let sender = self
            .channel_sender(channel_id)
            .ok_or_else(|| anyhow::anyhow!("not subscribed to channel {channel_id}"))?;
        tokio::time::timeout(OP_TIMEOUT, sender.broadcast(bytes.into()))
            .await
            .context("gossip broadcast timeout")?
            .context("gossip broadcast failed")?;
        Ok(())
    }

    pub fn drop_channel(&self, channel_id: &Id) {
        self.subscriptions.remove(channel_id);
    }

    pub fn drop_workspace(&self, workspace_id: &Id) {
        self.workspace_subscriptions.remove(workspace_id);
    }

    pub fn has_workspace(&self, workspace_id: &Id) -> bool {
        self.workspace_subscriptions.contains_key(workspace_id)
    }

    pub fn workspace_sender(&self, workspace_id: &Id) -> Option<GossipSender> {
        self.workspace_subscriptions
            .get(workspace_id)
            .map(|s| s.clone())
    }

    pub async fn subscribe_workspace(
        &self,
        workspace_id: &Id,
        secret_hex: &str,
        bootstrap: Vec<EndpointId>,
    ) -> anyhow::Result<(GossipSender, GossipReceiver)> {
        if self.workspace_sender(workspace_id).is_some() {
            anyhow::bail!("already subscribed to workspace {workspace_id}");
        }
        let topic = crate::ticket::workspace_topic_id(secret_hex)?;
        let parts = self.subscribe(topic, bootstrap).await?;
        self.workspace_subscriptions
            .insert(workspace_id.clone(), parts.0.clone());
        Ok(parts)
    }

    pub async fn broadcast_workspace(
        &self,
        workspace_id: &Id,
        bytes: Vec<u8>,
    ) -> anyhow::Result<()> {
        if bytes.len() > MAX_PAYLOAD {
            anyhow::bail!("payload too large for gossip: {} bytes", bytes.len());
        }
        let sender = self
            .workspace_sender(workspace_id)
            .ok_or_else(|| anyhow::anyhow!("not subscribed to workspace {workspace_id}"))?;
        tokio::time::timeout(OP_TIMEOUT, sender.broadcast(bytes.into()))
            .await
            .context("gossip broadcast timeout")?
            .context("gossip broadcast failed")?;
        Ok(())
    }

    pub async fn broadcast_reliable(&self, workspace_id: &Id, id: String, bytes: Vec<u8>) {
        if bytes.len() > MAX_PAYLOAD {
            tracing::warn!("dropping {} byte frame: over gossip limit", bytes.len());
            return;
        }
        self.pending.insert(
            id,
            Pending {
                bytes: bytes.clone(),
                workspace_id: workspace_id.clone(),
                attempts: 0,
                last: Instant::now(),
            },
        );
        if let Err(e) = self.broadcast_workspace(workspace_id, bytes).await {
            tracing::debug!("reliable broadcast failed: {e}");
        }
    }

    pub fn ack(&self, id: &str) {
        self.pending.remove(id);
    }

    pub fn mark_seen(&self, id: &str) -> bool {
        if self.seen.contains_key(id) {
            return false;
        }
        self.seen.insert(id.to_string(), Instant::now());
        true
    }

    async fn retry_loop(self: Arc<Self>) {
        let mut ticker = tokio::time::interval(Duration::from_secs(2));
        loop {
            ticker.tick().await;
            let now = Instant::now();
            let due: Vec<(String, Vec<u8>, Id, u32)> = self
                .pending
                .iter()
                .filter(|e| now.duration_since(e.last) >= Duration::from_secs(2))
                .map(|e| {
                    (
                        e.key().clone(),
                        e.bytes.clone(),
                        e.workspace_id.clone(),
                        e.attempts,
                    )
                })
                .collect();
            for (id, bytes, workspace_id, attempts) in due {
                if attempts >= 8 {
                    self.pending.remove(&id);
                    continue;
                }
                let _ = self
                    .broadcast_workspace(&workspace_id, with_attempt(&bytes, attempts + 1))
                    .await;
                if let Some(mut p) = self.pending.get_mut(&id) {
                    p.attempts += 1;
                    p.last = now;
                }
            }
            self.seen
                .retain(|_, t| now.duration_since(*t) < Duration::from_secs(120));
        }
    }

    pub async fn add_blob(&self, bytes: Vec<u8>) -> anyhow::Result<(String, Vec<u8>)> {
        let tag = tokio::time::timeout(OP_TIMEOUT, self.store.blobs().add_bytes(bytes))
            .await
            .context("add blob timeout")?
            .context("add blob failed")?;
        Ok((tag.hash.to_string(), tag.name.0.to_vec()))
    }

    pub async fn get_blob(&self, hash: &str) -> anyhow::Result<Vec<u8>> {
        let hash: iroh_blobs::Hash = hash
            .parse()
            .map_err(|_| anyhow::anyhow!("invalid blob hash"))?;
        let bytes = tokio::time::timeout(OP_TIMEOUT, self.store.blobs().get_bytes(hash))
            .await
            .context("get blob timeout")?
            .context("get blob failed")?;
        Ok(bytes.to_vec())
    }

    pub async fn download(&self, hash: &str, peer: EndpointId) -> anyhow::Result<()> {
        let hash: iroh_blobs::Hash = hash
            .parse()
            .map_err(|_| anyhow::anyhow!("invalid blob hash"))?;
        let downloader = self.store.downloader(&self.endpoint);
        tokio::time::timeout(OP_TIMEOUT, downloader.download(hash, Some(peer)))
            .await
            .context("download timeout")?
            .context("download failed")?;
        Ok(())
    }

    pub async fn tag_blob(&self, hash: &str) -> anyhow::Result<Vec<u8>> {
        let parsed: iroh_blobs::Hash = hash
            .parse()
            .map_err(|_| anyhow::anyhow!("invalid blob hash"))?;
        let name = hash.as_bytes().to_vec();
        tokio::time::timeout(
            OP_TIMEOUT,
            self.store.tags().set(
                &name,
                iroh_blobs::HashAndFormat {
                    hash: parsed,
                    format: iroh_blobs::BlobFormat::Raw,
                },
            ),
        )
        .await
        .context("tag blob timeout")?
        .context("tag blob failed")?;
        Ok(name)
    }

    pub async fn delete_blob_tag(&self, name: Vec<u8>) -> anyhow::Result<()> {
        tokio::time::timeout(OP_TIMEOUT, self.store.tags().delete(&name))
            .await
            .context("delete tag timeout")?
            .context("delete tag failed")?;
        Ok(())
    }

    pub async fn shutdown(self: Arc<Self>) {
        let _ = self.router.shutdown().await;
    }
}

// Plumtree dedups by content hash for 90s, so an identical retransmit never reaches a peer that
// already got the frame and lost our ack; varying the bytes makes it re-ack.
fn with_attempt(bytes: &[u8], attempt: u32) -> Vec<u8> {
    match serde_json::from_slice::<serde_json::Value>(bytes) {
        Ok(serde_json::Value::Object(mut map)) => {
            map.insert("r".into(), attempt.into());
            serde_json::to_vec(&map).unwrap_or_else(|_| bytes.to_vec())
        }
        _ => bytes.to_vec(),
    }
}
