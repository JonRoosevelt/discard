use std::path::PathBuf;
use std::sync::Arc;

use iroh::SecretKey;
use serde::{Deserialize, Serialize};
use shared::{Channel, Id, Workspace};
use tokio::sync::Mutex;

use crate::meta::{merge, Applied, Op, Target};
use crate::signed::{Payload, Signed};
use crate::ticket::WorkspaceTicket;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkspaceRecord {
    pub id: Id,
    pub name: String,
    pub icon_url: Option<String>,
    pub owner_id: Id,
    pub owner_name: String,
    pub secret: String,
    pub ticket: String,
    pub channels: Vec<Channel>,
    #[serde(default)]
    pub peers: Vec<String>,
    #[serde(default)]
    pub owner_endpoint_ticket: Option<String>,
    #[serde(default)]
    pub clock: u64,
    #[serde(default)]
    pub deleted: bool,
    // The winning signed op per object, kept so any member can re-send it to peers that missed it.
    #[serde(default)]
    pub ops: Vec<Applied>,
}

impl WorkspaceRecord {
    pub fn to_workspace(&self) -> Workspace {
        Workspace {
            id: self.id.clone(),
            name: self.name.clone(),
            icon_url: self.icon_url.clone(),
            owner_id: self.owner_id.clone(),
            ticket: self.ticket.clone(),
            secret: None,
            channels: self.channels.clone(),
        }
    }

    pub fn rebuild_ticket(&self) -> String {
        WorkspaceTicket {
            v: crate::ticket::TICKET_VERSION,
            id: self.id.clone(),
            name: self.name.clone(),
            owner_id: self.owner_id.clone(),
            owner_name: self.owner_name.clone(),
            secret: self.secret.clone(),
            channels: self.channels.clone(),
            endpoint_ticket: self.owner_endpoint_ticket.clone().unwrap_or_default(),
        }
        .encode()
    }
}

#[derive(Default, Serialize, Deserialize)]
struct Db {
    #[serde(default)]
    workspaces: Vec<WorkspaceRecord>,
}

pub struct Store {
    path: PathBuf,
    db: Mutex<Db>,
}

impl Store {
    pub fn load(path: PathBuf) -> Arc<Self> {
        let db = std::fs::read(&path)
            .ok()
            .and_then(|bytes| serde_json::from_slice(&bytes).ok())
            .unwrap_or_default();
        Arc::new(Self {
            path,
            db: Mutex::new(db),
        })
    }

    fn persist(path: &PathBuf, db: &Db) {
        if let Some(parent) = path.parent() {
            let _ = std::fs::create_dir_all(parent);
        }
        if let Ok(bytes) = serde_json::to_vec_pretty(db) {
            let tmp = path.with_extension("json.tmp");
            if std::fs::write(&tmp, bytes).is_ok() {
                let _ = std::fs::rename(&tmp, path);
            }
        }
    }

    pub async fn list(&self) -> Vec<Workspace> {
        let db = self.db.lock().await;
        db.workspaces
            .iter()
            .filter(|w| !w.deleted)
            .map(|w| w.to_workspace())
            .collect()
    }

    pub async fn list_with_secrets(&self) -> Vec<Workspace> {
        let db = self.db.lock().await;
        db.workspaces
            .iter()
            .filter(|w| !w.deleted)
            .map(|w| Workspace {
                secret: Some(w.secret.clone()),
                ..w.to_workspace()
            })
            .collect()
    }

    pub async fn record(&self, id: &Id) -> Option<WorkspaceRecord> {
        let db = self.db.lock().await;
        db.workspaces.iter().find(|w| &w.id == id).cloned()
    }

    pub async fn channel(&self, channel_id: &Id) -> Option<Channel> {
        let db = self.db.lock().await;
        db.workspaces
            .iter()
            .filter(|w| !w.deleted)
            .flat_map(|w| w.channels.iter())
            .find(|c| &c.id == channel_id)
            .cloned()
    }

    pub async fn upsert(&self, record: WorkspaceRecord) -> Workspace {
        let mut db = self.db.lock().await;
        if let Some(existing) = db.workspaces.iter_mut().find(|w| w.id == record.id) {
            *existing = record.clone();
        } else {
            db.workspaces.push(record.clone());
        }
        Self::persist(&self.path, &db);
        record.to_workspace()
    }

    pub async fn all(&self) -> Vec<WorkspaceRecord> {
        self.db.lock().await.workspaces.clone()
    }

    pub async fn remove(&self, id: &Id) -> Option<WorkspaceRecord> {
        let mut db = self.db.lock().await;
        let index = db.workspaces.iter().position(|w| &w.id == id)?;
        let record = db.workspaces.remove(index);
        Self::persist(&self.path, &db);
        Some(record)
    }

    fn foreign_channels(db: &Db, workspace_id: &Id) -> Vec<Id> {
        db.workspaces
            .iter()
            .filter(|w| &w.id != workspace_id)
            .flat_map(|w| w.channels.iter().map(|c| c.id.clone()))
            .collect()
    }

    /// Signs and merges a local op built by `target` (None = nothing to change).
    /// Returns the channels before, the record after, and the op if it took effect.
    pub async fn commit(
        &self,
        workspace_id: &Id,
        key: &SecretKey,
        target: impl FnOnce(&WorkspaceRecord) -> Option<Target>,
    ) -> Option<(Vec<Channel>, WorkspaceRecord, Option<Signed>)> {
        let mut db = self.db.lock().await;
        let foreign = Self::foreign_channels(&db, workspace_id);
        let record = db.workspaces.iter_mut().find(|w| &w.id == workspace_id)?;
        let before = record.channels.clone();
        let Some(target) = target(record) else {
            return Some((before, record.clone(), None));
        };
        let op = Op {
            workspace_id: workspace_id.clone(),
            lamport: record.clock + 1,
            target,
        };
        let frame_id = hex::encode(rand::random::<[u8; 8]>());
        let signed = Signed::new(key, frame_id, &Payload::Op { op: op.clone() })?;
        let applied = Applied {
            op,
            author: key.public().to_string(),
            signed: signed.clone(),
        };
        if !merge(record, applied, &foreign) {
            return Some((before, record.clone(), None));
        }
        record.ticket = record.rebuild_ticket();
        let record = record.clone();
        Self::persist(&self.path, &db);
        Some((before, record, Some(signed)))
    }

    pub async fn apply(&self, applied: Applied) -> Option<(Vec<Channel>, WorkspaceRecord)> {
        let mut db = self.db.lock().await;
        let foreign = Self::foreign_channels(&db, &applied.op.workspace_id);
        let record = db
            .workspaces
            .iter_mut()
            .find(|w| w.id == applied.op.workspace_id)?;
        let before = record.channels.clone();
        if !merge(record, applied, &foreign) {
            return None;
        }
        record.ticket = record.rebuild_ticket();
        let record = record.clone();
        Self::persist(&self.path, &db);
        Some((before, record))
    }

    pub async fn secret_for_channel(&self, channel_id: &Id) -> Option<(Id, String)> {
        let db = self.db.lock().await;
        for record in db.workspaces.iter().filter(|w| !w.deleted) {
            if record.channels.iter().any(|c| &c.id == channel_id) {
                return Some((record.id.clone(), record.secret.clone()));
            }
        }
        None
    }

    #[allow(dead_code)]
    pub async fn peers(&self, workspace_id: &Id) -> Vec<String> {
        let db = self.db.lock().await;
        db.workspaces
            .iter()
            .find(|w| &w.id == workspace_id)
            .map(|w| w.peers.clone())
            .unwrap_or_default()
    }

    pub async fn add_peer(&self, workspace_id: &Id, endpoint_id: &str) {
        let mut db = self.db.lock().await;
        if let Some(record) = db.workspaces.iter_mut().find(|w| &w.id == workspace_id) {
            if !record.peers.iter().any(|p| p == endpoint_id) {
                record.peers.push(endpoint_id.to_string());
                Self::persist(&self.path, &db);
            }
        }
    }
}
