use base64::Engine;
use serde::{Deserialize, Serialize};
use shared::Channel;

pub const TICKET_VERSION: u32 = 1;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkspaceTicket {
    pub v: u32,
    pub id: String,
    pub name: String,
    pub owner_id: String,
    pub owner_name: String,
    pub secret: String,
    pub channels: Vec<Channel>,
    pub endpoint_ticket: String,
}

impl WorkspaceTicket {
    pub fn encode(&self) -> String {
        let json = serde_json::to_vec(self).expect("ticket serialization");
        base64::engine::general_purpose::STANDARD.encode(json)
    }

    pub fn decode(text: &str) -> anyhow::Result<Self> {
        let bytes = base64::engine::general_purpose::STANDARD
            .decode(text.trim())
            .map_err(|_| anyhow::anyhow!("ticket is not valid base64"))?;
        let ticket: WorkspaceTicket = serde_json::from_slice(&bytes)
            .map_err(|_| anyhow::anyhow!("ticket is not valid json"))?;
        if ticket.v != TICKET_VERSION {
            anyhow::bail!("unsupported ticket version {}", ticket.v);
        }
        let secret = hex::decode(&ticket.secret)
            .map_err(|_| anyhow::anyhow!("ticket secret is not valid hex"))?;
        if secret.len() != 32 {
            anyhow::bail!("ticket secret must be 32 bytes");
        }
        if ticket.endpoint_ticket.trim().is_empty() {
            anyhow::bail!("ticket has no endpoint ticket");
        }
        Ok(ticket)
    }
}

pub fn topic_id(secret_hex: &str, channel_id: &str) -> anyhow::Result<iroh_gossip::TopicId> {
    let secret = hex::decode(secret_hex)
        .map_err(|_| anyhow::anyhow!("secret is not valid hex"))?;
    let mut input = Vec::with_capacity(secret.len() + channel_id.len());
    input.extend_from_slice(&secret);
    input.extend_from_slice(channel_id.as_bytes());
    let hash = blake3::hash(&input);
    Ok(iroh_gossip::TopicId::from_bytes(*hash.as_bytes()))
}
