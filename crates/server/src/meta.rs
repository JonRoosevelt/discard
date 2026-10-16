use serde::{Deserialize, Serialize};
use shared::{Channel, Id, ServerEvent};

use crate::signed::Signed;
use crate::state::AppState;
use crate::store::WorkspaceRecord;
use crate::ws;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Op {
    pub workspace_id: Id,
    pub lamport: u64,
    pub target: Target,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "on", rename_all = "snake_case")]
pub enum Target {
    Workspace { name: String, deleted: bool },
    Channel { channel: Channel, deleted: bool },
}

impl Target {
    fn key(&self) -> &str {
        match self {
            Target::Workspace { .. } => "",
            Target::Channel { channel, .. } => &channel.id,
        }
    }

    fn deleted(&self) -> bool {
        match self {
            Target::Workspace { deleted, .. } | Target::Channel { deleted, .. } => *deleted,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Applied {
    pub op: Op,
    pub author: Id,
    pub signed: Signed,
}

impl Applied {
    // Deletes outrank every edit so a tombstone can never be undone by a stale or concurrent
    // rename; the author id breaks lamport ties so every node picks the same winner.
    fn rank(&self) -> (bool, u64, &str) {
        (self.op.target.deleted(), self.op.lamport, &self.author)
    }
}

pub fn merge(record: &mut WorkspaceRecord, applied: Applied, foreign: &[Id]) -> bool {
    if applied.op.workspace_id != record.id {
        return false;
    }
    record.clock = record.clock.max(applied.op.lamport);
    // A channel id already used by another workspace would make channel lookups ambiguous.
    if foreign.iter().any(|id| id == applied.op.target.key()) {
        return false;
    }
    let slot = record
        .ops
        .iter()
        .position(|a| a.op.target.key() == applied.op.target.key());
    match slot {
        Some(i) if record.ops[i].rank() >= applied.rank() => return false,
        Some(i) => record.ops[i] = applied.clone(),
        None => record.ops.push(applied.clone()),
    }
    match applied.op.target {
        Target::Workspace { name, deleted } => {
            record.name = name;
            record.deleted = deleted;
        }
        Target::Channel { channel, deleted } => {
            let channel = Channel {
                workspace_id: record.id.clone(),
                ..channel
            };
            let at = record.channels.iter().position(|c| c.id == channel.id);
            match (at, deleted) {
                (Some(i), true) => {
                    record.channels.remove(i);
                }
                (Some(i), false) => record.channels[i] = channel,
                (None, false) => record.channels.push(channel),
                (None, true) => {}
            }
        }
    }
    true
}

pub async fn change(
    state: &AppState,
    workspace_id: &Id,
    target: impl FnOnce(&WorkspaceRecord) -> Option<Target>,
) -> Option<WorkspaceRecord> {
    let key = state.identity.secret_key();
    let (before, record, signed) = state.store.commit(workspace_id, &key, target).await?;
    if let Some(signed) = signed {
        notify(state, &before, &record).await;
        ws::ensure_workspace_subscription(state, workspace_id).await;
        if let Some(bytes) = ws::encode_frame(signed.clone()) {
            let net = state.net().await;
            net.broadcast_reliable(workspace_id, signed.id, bytes).await;
        }
    }
    Some(record)
}

pub async fn apply(state: &AppState, workspace_id: &Id, author: Id, op: Op, signed: Signed) {
    if &op.workspace_id != workspace_id {
        return;
    }
    let applied = Applied { op, author, signed };
    if let Some((before, record)) = state.store.apply(applied).await {
        notify(state, &before, &record).await;
    }
}

// Re-sending every op we hold is the full-state exchange: receivers merge with the same rule,
// so a peer that was away catches up and stale peers cannot win.
// ponytail: one gossip frame per op; batch them if workspaces grow to hundreds of channels.
pub async fn relay(state: &AppState, workspace_id: &Id, only_author: Option<&str>) {
    let Some(record) = state.store.record(workspace_id).await else {
        return;
    };
    let net = state.net().await;
    for applied in record.ops {
        if only_author.is_some_and(|a| a != applied.author) {
            continue;
        }
        if let Some(bytes) = ws::encode_frame(applied.signed) {
            let bytes = crate::iroh_net::with_attempt(&bytes, rand::random());
            let _ = net.broadcast_workspace(workspace_id, bytes).await;
        }
    }
}

// Deleted workspaces stay subscribed so their tombstone keeps reaching members that were away.
// ponytail: tombstones are never collected; add an expiry if they ever pile up.
pub async fn tick(state: &AppState) {
    let me = state.identity.public_id();
    for record in state.store.all().await {
        ws::ensure_workspace_subscription(state, &record.id).await;
        relay(state, &record.id, Some(&me)).await;
    }
}

pub async fn leave(state: &AppState, workspace_id: &Id) {
    let Some(record) = state.store.record(workspace_id).await else {
        return;
    };
    for channel in &record.channels {
        if state.voice.local_member(&channel.id).is_some() {
            ws::leave_voice(state, &channel.id).await;
        }
    }
    state.store.remove(workspace_id).await;
    state.net().await.drop_workspace(workspace_id);
    state.hub.broadcast_all(ServerEvent::WorkspaceRemoved {
        workspace_id: workspace_id.clone(),
    });
}

async fn notify(state: &AppState, before: &[Channel], record: &WorkspaceRecord) {
    for channel in before {
        let gone = record.deleted || !record.channels.iter().any(|c| c.id == channel.id);
        if gone && state.voice.local_member(&channel.id).is_some() {
            ws::leave_voice(state, &channel.id).await;
        }
    }
    let event = if record.deleted {
        ServerEvent::WorkspaceRemoved {
            workspace_id: record.id.clone(),
        }
    } else {
        ServerEvent::WorkspaceUpdated {
            workspace: record.to_workspace(),
        }
    };
    state.hub.broadcast_all(event);
}

#[cfg(test)]
mod tests {
    use super::*;
    use iroh::SecretKey;
    use shared::ChannelKind;

    fn record() -> WorkspaceRecord {
        WorkspaceRecord {
            id: "w".into(),
            name: "w".into(),
            icon_url: None,
            owner_id: String::new(),
            owner_name: String::new(),
            secret: String::new(),
            ticket: String::new(),
            channels: Vec::new(),
            peers: Vec::new(),
            owner_endpoint_ticket: None,
            clock: 0,
            deleted: false,
            ops: Vec::new(),
        }
    }

    fn op(key: &SecretKey, lamport: u64, name: &str, deleted: bool) -> Applied {
        let op = Op {
            workspace_id: "w".into(),
            lamport,
            target: Target::Channel {
                channel: Channel {
                    id: "c".into(),
                    workspace_id: "w".into(),
                    name: name.into(),
                    kind: ChannelKind::Text,
                    position: 0,
                },
                deleted,
            },
        };
        let signed = Signed::new(key, lamport.to_string(), &op).unwrap();
        Applied {
            op,
            author: key.public().to_string(),
            signed,
        }
    }

    fn names(r: &WorkspaceRecord) -> Vec<String> {
        r.channels.iter().map(|c| c.name.clone()).collect()
    }

    #[test]
    fn last_writer_wins() {
        let a = SecretKey::generate();
        let b = SecretKey::generate();

        let mut r = record();
        assert!(merge(&mut r, op(&a, 1, "one", false), &[]));
        assert!(merge(&mut r, op(&b, 3, "three", false), &[]), "newer");
        assert!(!merge(&mut r, op(&a, 2, "two", false), &[]), "older");
        assert_eq!(names(&r), ["three"]);
        assert_eq!(r.clock, 3);

        let (x, y) = (op(&a, 5, "from-a", false), op(&b, 5, "from-b", false));
        let (mut r1, mut r2) = (record(), record());
        merge(&mut r1, x.clone(), &[]);
        merge(&mut r1, y.clone(), &[]);
        merge(&mut r2, y, &[]);
        merge(&mut r2, x, &[]);
        assert_eq!(names(&r1), names(&r2), "tie converges either order");

        assert!(merge(&mut r, op(&a, 4, "", true), &[]), "delete");
        assert!(!merge(&mut r, op(&b, 99, "zombie", false), &[]), "no resurrection");
        assert!(r.channels.is_empty());

        assert!(!merge(&mut record(), op(&a, 1, "x", false), &["c".into()]), "foreign id");
    }
}
