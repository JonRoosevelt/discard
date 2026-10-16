use std::collections::HashSet;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;

use dashmap::DashMap;
use shared::{Id, Message, ServerEvent};
use tokio::sync::mpsc::UnboundedSender;

pub type ConnId = u64;

pub struct Conn {
    pub tx: UnboundedSender<ServerEvent>,
    pub channels: HashSet<Id>,
}

pub struct Hub {
    conns: DashMap<ConnId, Conn>,
    subs: DashMap<Id, HashSet<ConnId>>,
    next: AtomicU64,
}

impl Hub {
    pub fn new() -> Arc<Self> {
        Arc::new(Hub {
            conns: DashMap::new(),
            subs: DashMap::new(),
            next: AtomicU64::new(1),
        })
    }

    pub fn register(&self, tx: UnboundedSender<ServerEvent>) -> ConnId {
        let id = self.next.fetch_add(1, Ordering::Relaxed);
        self.conns.insert(
            id,
            Conn {
                tx,
                channels: HashSet::new(),
            },
        );
        id
    }

    pub fn unregister(&self, conn: ConnId) {
        self.conns.remove(&conn);
        self.subs.retain(|_, set| {
            set.remove(&conn);
            !set.is_empty()
        });
    }

    pub fn is_empty(&self) -> bool {
        self.conns.is_empty()
    }

    pub fn subscribe(&self, conn: ConnId, channel_id: Id) {
        if let Some(mut c) = self.conns.get_mut(&conn) {
            c.channels.insert(channel_id.clone());
        }
        self.subs.entry(channel_id).or_default().insert(conn);
    }

    pub fn unsubscribe(&self, conn: ConnId, channel_id: &Id) {
        if let Some(mut c) = self.conns.get_mut(&conn) {
            c.channels.remove(channel_id);
        }
        let mut empty = false;
        if let Some(mut set) = self.subs.get_mut(channel_id) {
            set.remove(&conn);
            empty = set.is_empty();
        }
        if empty {
            self.subs.remove(channel_id);
        }
    }

    pub fn subscribers_of(&self, channel_id: &Id) -> Vec<ConnId> {
        self.subs
            .get(channel_id)
            .map(|set| set.iter().copied().collect())
            .unwrap_or_default()
    }

    pub fn send_to_conn(&self, conn: ConnId, ev: ServerEvent) {
        if let Some(c) = self.conns.get(&conn) {
            let _ = c.tx.send(ev);
        }
    }

    pub fn broadcast_all(&self, ev: ServerEvent) {
        for conn in self.conns.iter() {
            let _ = conn.tx.send(ev.clone());
        }
    }

    pub fn broadcast_message(&self, msg: &Message) {
        let ev = ServerEvent::Message {
            message: msg.clone(),
        };
        for conn in self.subscribers_of(&msg.channel_id) {
            self.send_to_conn(conn, ev.clone());
        }
    }
}
