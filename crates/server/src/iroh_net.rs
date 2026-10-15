use std::path::Path;
use std::sync::Arc;
use std::time::Duration;

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

pub const OP_TIMEOUT: Duration = Duration::from_secs(20);

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
}

impl IrohNet {
    pub async fn start(
        secret_key: SecretKey,
        offline: bool,
        blobs_dir: &Path,
    ) -> anyhow::Result<Arc<Self>> {
        let lookup = MemoryLookup::new();
        let builder = if offline {
            Endpoint::builder(presets::Minimal)
        } else {
            Endpoint::builder(presets::N0)
        };
        let endpoint = builder
            .address_lookup(lookup.clone())
            .secret_key(secret_key)
            .bind()
            .await
            .context("bind iroh endpoint")?;

        let gossip = Gossip::builder().spawn(endpoint.clone());

        std::fs::create_dir_all(blobs_dir)?;
        let store = FsStore::load(blobs_dir)
            .await
            .context("load blob store")?;

        let router = Router::builder(endpoint.clone())
            .accept(iroh_gossip::ALPN, gossip.clone())
            .accept(iroh_blobs::ALPN, BlobsProtocol::new(&store, None))
            .spawn();

        Ok(Arc::new(Self {
            endpoint,
            gossip,
            store,
            lookup,
            router,
            subscriptions: DashMap::new(),
        }))
    }

    pub fn endpoint_ticket(&self) -> String {
        EndpointTicket::new(self.endpoint.addr()).to_string()
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

    pub async fn broadcast_channel(&self, channel_id: &Id, bytes: Vec<u8>) -> anyhow::Result<()> {
        let sender = self
            .channel_sender(channel_id)
            .ok_or_else(|| anyhow::anyhow!("not subscribed to channel {channel_id}"))?;
        tokio::time::timeout(OP_TIMEOUT, sender.broadcast(bytes.into()))
            .await
            .context("gossip broadcast timeout")?
            .context("gossip broadcast failed")?;
        Ok(())
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
