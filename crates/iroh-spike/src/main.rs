use std::time::Duration;

use anyhow::{anyhow, Context, Result};
use futures_util::StreamExt;
use iroh::{
    address_lookup::memory::MemoryLookup, endpoint::presets, protocol::Router, Endpoint,
};
use iroh_blobs::{store::mem::MemStore, BlobsProtocol};
use iroh_gossip::{api::Event, Gossip, TopicId};
use iroh_tickets::endpoint::EndpointTicket;

const DUR: Duration = Duration::from_secs(20);

#[tokio::main]
async fn main() -> Result<()> {
    let lookup_a = MemoryLookup::new();
    let lookup_b = MemoryLookup::new();

    let ep_a = Endpoint::builder(presets::Minimal)
        .address_lookup(lookup_a.clone())
        .bind()
        .await
        .context("bind A")?;
    let ep_b = Endpoint::builder(presets::Minimal)
        .address_lookup(lookup_b.clone())
        .bind()
        .await
        .context("bind B")?;

    println!("[id] A = {}", ep_a.id());
    println!("[id] B = {}", ep_b.id());

    let addr_a = ep_a.addr();
    let addr_b = ep_b.addr();
    println!("[addr] A = {addr_a:?}");
    println!("[addr] B = {addr_b:?}");
    println!("[addr] A relay_urls = {}", addr_a.relay_urls().count());
    println!("[addr] B relay_urls = {}", addr_b.relay_urls().count());

    lookup_a.add_endpoint_info(addr_b.clone());
    lookup_b.add_endpoint_info(addr_a.clone());

    let store_a = MemStore::new();
    let store_b = MemStore::new();

    let gossip_a = Gossip::builder().spawn(ep_a.clone());
    let gossip_b = Gossip::builder().spawn(ep_b.clone());

    let router_a = Router::builder(ep_a.clone())
        .accept(iroh_gossip::ALPN, gossip_a.clone())
        .accept(iroh_blobs::ALPN, BlobsProtocol::new(&store_a, None))
        .spawn();
    let router_b = Router::builder(ep_b.clone())
        .accept(iroh_gossip::ALPN, gossip_b.clone())
        .accept(iroh_blobs::ALPN, BlobsProtocol::new(&store_b, None))
        .spawn();

    let topic = TopicId::from_bytes([7u8; 32]);
    let (tx_a, mut rx_a) = gossip_a.subscribe(topic, vec![ep_b.id()]).await?.split();
    let (tx_b, mut rx_b) = gossip_b.subscribe(topic, vec![ep_a.id()]).await?.split();
    let _keep_b = tx_b;

    tokio::time::timeout(DUR, rx_a.joined())
        .await
        .context("A joined timeout")??;
    tokio::time::timeout(DUR, rx_b.joined())
        .await
        .context("B joined timeout")??;
    println!("[gossip] A joined; B joined");

    let payload = b"cadence-ping-from-A".to_vec();
    tx_a.broadcast(payload.clone().into()).await?;
    println!("[gossip] A broadcast {} bytes", payload.len());

    let received = tokio::time::timeout(DUR, async {
        while let Some(ev) = rx_b.next().await {
            if let Event::Received(msg) = ev? {
                return Ok::<_, anyhow::Error>(msg.content);
            }
        }
        Err(anyhow!("gossip stream ended"))
    })
    .await
    .context("B receive timeout")??;
    println!(
        "[gossip] B received {} bytes: {:?}",
        received.len(),
        String::from_utf8_lossy(&received)
    );
    assert_eq!(received.as_ref(), payload.as_slice());
    println!("[gossip] OK: bytes received from A are equal");

    let blob = b"cadence blob payload for integrity check".to_vec();
    let tag = store_a.blobs().add_bytes(blob.clone()).await?;
    println!("[blob] A added blob hash={} size={}", tag.hash, blob.len());

    let downloader = store_b.downloader(&ep_b);
    downloader.download(tag.hash, Some(ep_a.id())).await?;
    println!("[blob] B downloaded hash={}", tag.hash);

    let got = store_b.blobs().get_bytes(tag.hash).await?;
    println!("[blob] B read {} bytes", got.len());
    assert_eq!(got.as_ref(), blob.as_slice());
    println!("[blob] OK: bytes downloaded from A are equal (integrity verified)");

    let ticket = EndpointTicket::new(ep_a.addr());
    let ticket_str = ticket.to_string();
    println!("[ticket] {ticket_str}");
    let parsed: EndpointTicket = ticket_str.parse()?;
    println!("[ticket] parsed id = {}", parsed.endpoint_addr().id);
    assert_eq!(parsed.endpoint_addr().id, ep_a.id());
    println!("[ticket] OK: roundtrip id matches A");

    router_a.shutdown().await?;
    router_b.shutdown().await?;
    println!("[done] all primitives verified");
    Ok(())
}
