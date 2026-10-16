use std::path::PathBuf;
use std::sync::Arc;

use serde::{Deserialize, Serialize};
use shared::{Channel, ChannelKind, Id, Workspace};
use tokio::sync::Mutex;

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
        db.workspaces.iter().map(|w| w.to_workspace()).collect()
    }

    pub async fn list_with_secrets(&self) -> Vec<Workspace> {
        let db = self.db.lock().await;
        db.workspaces
            .iter()
            .map(|w| Workspace {
                secret: Some(w.secret.clone()),
                ..w.to_workspace()
            })
            .collect()
    }

    pub async fn get(&self, id: &Id) -> Option<Workspace> {
        let db = self.db.lock().await;
        db.workspaces
            .iter()
            .find(|w| &w.id == id)
            .map(|w| w.to_workspace())
    }

    pub async fn record(&self, id: &Id) -> Option<WorkspaceRecord> {
        let db = self.db.lock().await;
        db.workspaces.iter().find(|w| &w.id == id).cloned()
    }

    pub async fn channel(&self, channel_id: &Id) -> Option<Channel> {
        let db = self.db.lock().await;
        db.workspaces
            .iter()
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

    pub async fn add_channel(
        &self,
        workspace_id: &Id,
        name: &str,
        kind: ChannelKind,
        id: Option<Id>,
    ) -> anyhow::Result<Channel> {
        let mut db = self.db.lock().await;
        if let Some(id) = &id {
            let existing = db
                .workspaces
                .iter()
                .find_map(|w| w.channels.iter().find(|c| &c.id == id));
            match existing {
                Some(c) if &c.workspace_id == workspace_id => return Ok(c.clone()),
                Some(_) => anyhow::bail!("channel id already in use"),
                None => {}
            }
        }
        let record = db
            .workspaces
            .iter_mut()
            .find(|w| &w.id == workspace_id)
            .ok_or_else(|| anyhow::anyhow!("workspace not found"))?;
        let position = record.channels.iter().map(|c| c.position).max().unwrap_or(-1) + 1;
        let channel = Channel {
            id: id.unwrap_or_else(|| uuid::Uuid::new_v4().to_string()),
            workspace_id: workspace_id.clone(),
            name: name.to_string(),
            kind,
            position,
        };
        record.channels.push(channel.clone());
        record.ticket = record.rebuild_ticket();
        Self::persist(&self.path, &db);
        Ok(channel)
    }

    pub async fn secret_for_channel(&self, channel_id: &Id) -> Option<(Id, String)> {
        let db = self.db.lock().await;
        for record in &db.workspaces {
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
