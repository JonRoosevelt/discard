use std::collections::{HashMap, HashSet, VecDeque};
use std::sync::Mutex;
use std::time::Duration;

use chrono::{DateTime, Utc};
use shared::{Attachment, Id, Message};

pub const RETENTION: Duration = Duration::from_secs(24 * 60 * 60);

fn window() -> chrono::Duration {
    chrono::Duration::seconds(RETENTION.as_secs() as i64)
}

struct BlobRef {
    tag: Vec<u8>,
    expiry: DateTime<Utc>,
}

struct Pending {
    attachment: Attachment,
    tag: Vec<u8>,
}

#[derive(Default)]
struct Inner {
    channels: HashMap<Id, VecDeque<Message>>,
    seen: HashMap<Id, DateTime<Utc>>,
    blobs: HashMap<Id, BlobRef>,
    pending: HashMap<Id, Pending>,
}

pub struct Retention {
    inner: Mutex<Inner>,
}

impl Default for Retention {
    fn default() -> Self {
        Self {
            inner: Mutex::new(Inner::default()),
        }
    }
}

impl Retention {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn seen(&self, id: &Id) -> bool {
        self.inner
            .lock()
            .map(|inner| inner.seen.contains_key(id))
            .unwrap_or(false)
    }

    pub fn insert_message(&self, message: Message) -> bool {
        let now = Utc::now();
        let mut inner = match self.inner.lock() {
            Ok(inner) => inner,
            Err(_) => return false,
        };
        if inner.seen.contains_key(&message.id) {
            return false;
        }
        inner
            .seen
            .insert(message.id.clone(), message.created_at + window());
        for attachment in &message.attachments {
            inner.blobs.entry(attachment.id.clone()).or_insert(BlobRef {
                tag: Vec::new(),
                expiry: now + window(),
            });
        }
        let queue = inner.channels.entry(message.channel_id.clone()).or_default();
        let pos = queue
            .iter()
            .position(|m| m.created_at > message.created_at)
            .unwrap_or(queue.len());
        queue.insert(pos, message);
        true
    }

    pub fn messages(&self, channel_id: &Id) -> Vec<Message> {
        let cutoff = Utc::now() - window();
        let inner = match self.inner.lock() {
            Ok(inner) => inner,
            Err(_) => return Vec::new(),
        };
        inner
            .channels
            .get(channel_id)
            .map(|queue| {
                queue
                    .iter()
                    .filter(|m| m.created_at > cutoff)
                    .cloned()
                    .collect()
            })
            .unwrap_or_default()
    }

    pub fn register_upload(&self, attachment: Attachment, tag: Vec<u8>) {
        if let Ok(mut inner) = self.inner.lock() {
            inner.pending.insert(
                attachment.id.clone(),
                Pending { attachment, tag },
            );
        }
    }

    pub fn resolve_attachments(&self, ids: &[Id]) -> Vec<Attachment> {
        let now = Utc::now();
        let mut out = Vec::with_capacity(ids.len());
        let mut inner = match self.inner.lock() {
            Ok(inner) => inner,
            Err(_) => return out,
        };
        for id in ids {
            if let Some(pending) = inner.pending.remove(id) {
                inner.blobs.insert(
                    id.clone(),
                    BlobRef {
                        tag: pending.tag,
                        expiry: now + window(),
                    },
                );
                out.push(pending.attachment);
            } else {
                out.push(Attachment {
                    id: id.clone(),
                    filename: id.clone(),
                    url: format!("/files/{id}"),
                    content_type: "application/octet-stream".to_string(),
                    size: 0,
                    width: None,
                    height: None,
                });
            }
        }
        out
    }

    pub fn register_blob(&self, hash: Id, tag: Vec<u8>) {
        if let Ok(mut inner) = self.inner.lock() {
            let expiry = Utc::now() + window();
            inner
                .blobs
                .entry(hash)
                .and_modify(|b| {
                    if !tag.is_empty() {
                        b.tag = tag.clone();
                    }
                    b.expiry = expiry;
                })
                .or_insert(BlobRef { tag, expiry });
        }
    }

    pub fn purge(&self) -> Vec<Vec<u8>> {
        let now = Utc::now();
        let cutoff = now - window();
        let mut inner = match self.inner.lock() {
            Ok(inner) => inner,
            Err(_) => return Vec::new(),
        };

        let mut referenced: HashSet<Id> = HashSet::new();
        for queue in inner.channels.values_mut() {
            queue.retain(|m| m.created_at > cutoff);
            for message in queue.iter() {
                for attachment in &message.attachments {
                    referenced.insert(attachment.id.clone());
                }
            }
        }
        inner.channels.retain(|_, queue| !queue.is_empty());
        inner.seen.retain(|_, expiry| *expiry > now);

        let mut tags = Vec::new();
        inner.blobs.retain(|hash, blob| {
            if referenced.contains(hash) {
                return true;
            }
            if blob.expiry > now {
                return true;
            }
            if !blob.tag.is_empty() {
                tags.push(blob.tag.clone());
            }
            false
        });
        tags
    }
}
