use std::collections::HashMap;
use std::sync::Mutex;
use std::time::{Duration, Instant};

use serde::{Deserialize, Serialize};
use shared::{Id, SignalData, VoiceMember, VoicePresence};

pub const HEARTBEAT: Duration = Duration::from_secs(10);
pub const MEMBER_TTL: Duration = Duration::from_secs(30);

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum Envelope {
    PresenceJoin {
        channel_id: Id,
        member: VoiceMember,
        ts_ms: i64,
    },
    PresenceUpdate {
        channel_id: Id,
        member: VoiceMember,
    },
    PresenceLeave {
        channel_id: Id,
        user_id: Id,
    },
    PresenceSync {
        channel_id: Id,
        from: Id,
    },
    PresenceState {
        channel_id: Id,
        members: Vec<VoiceMember>,
    },
    Signal {
        channel_id: Id,
        to: Id,
        from: Id,
        data: SignalData,
        #[serde(default)]
        session: Option<String>,
    },
    Sound {
        channel_id: Id,
        sound_id: String,
        by: Id,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        sound: Option<crate::signed::Signed>,
    },
}

impl Envelope {
    pub fn channel_id(&self) -> &Id {
        match self {
            Envelope::PresenceJoin { channel_id, .. }
            | Envelope::PresenceUpdate { channel_id, .. }
            | Envelope::PresenceLeave { channel_id, .. }
            | Envelope::PresenceSync { channel_id, .. }
            | Envelope::PresenceState { channel_id, .. }
            | Envelope::Signal { channel_id, .. }
            | Envelope::Sound { channel_id, .. } => channel_id,
        }
    }
}

struct Entry {
    member: VoiceMember,
    local: bool,
    last_seen: Instant,
}

#[derive(Default)]
struct Channel {
    workspace_id: Id,
    members: HashMap<Id, Entry>,
}

#[derive(Default)]
pub struct Voice {
    inner: Mutex<HashMap<Id, Channel>>,
}

impl Voice {
    pub fn new() -> Self {
        Self::default()
    }

    fn lock(&self) -> std::sync::MutexGuard<'_, HashMap<Id, Channel>> {
        self.inner.lock().unwrap_or_else(|e| e.into_inner())
    }

    fn channel<'a>(
        inner: &'a mut HashMap<Id, Channel>,
        channel_id: &Id,
        workspace_id: &Id,
    ) -> &'a mut Channel {
        let ch = inner.entry(channel_id.clone()).or_default();
        if ch.workspace_id.is_empty() {
            ch.workspace_id = workspace_id.clone();
        }
        ch
    }

    pub fn join(&self, channel_id: &Id, workspace_id: &Id, member: VoiceMember) -> VoicePresence {
        let mut inner = self.lock();
        let ch = Self::channel(&mut inner, channel_id, workspace_id);
        ch.members.insert(
            member.user.id.clone(),
            Entry {
                member,
                local: true,
                last_seen: Instant::now(),
            },
        );
        presence_of(&ch.workspace_id, channel_id, ch)
    }

    pub fn update_local(&self, channel_id: &Id, member: VoiceMember) -> Option<VoicePresence> {
        let mut inner = self.lock();
        let ch = inner.get_mut(channel_id)?;
        ch.members.insert(
            member.user.id.clone(),
            Entry {
                member,
                local: true,
                last_seen: Instant::now(),
            },
        );
        Some(presence_of(&ch.workspace_id, channel_id, ch))
    }

    pub fn leave(&self, channel_id: &Id, user_id: &Id) -> Option<Id> {
        let mut inner = self.lock();
        let ch = inner.get_mut(channel_id)?;
        ch.members.remove(user_id);
        let workspace_id = ch.workspace_id.clone();
        if ch.members.is_empty() {
            inner.remove(channel_id);
        }
        Some(workspace_id)
    }

    pub fn remote_join(
        &self,
        channel_id: &Id,
        workspace_id: &Id,
        member: VoiceMember,
    ) -> VoicePresence {
        let mut inner = self.lock();
        let ch = Self::channel(&mut inner, channel_id, workspace_id);
        ch.members.insert(
            member.user.id.clone(),
            Entry {
                member,
                local: false,
                last_seen: Instant::now(),
            },
        );
        presence_of(&ch.workspace_id, channel_id, ch)
    }

    pub fn remote_update(&self, channel_id: &Id, workspace_id: &Id, member: VoiceMember) {
        let mut inner = self.lock();
        let ch = Self::channel(&mut inner, channel_id, workspace_id);
        match ch.members.get_mut(&member.user.id) {
            Some(entry) if !entry.local => {
                entry.member = member;
                entry.last_seen = Instant::now();
            }
            Some(_) => {}
            None => {
                ch.members.insert(
                    member.user.id.clone(),
                    Entry {
                        member,
                        local: false,
                        last_seen: Instant::now(),
                    },
                );
            }
        }
    }

    pub fn remote_leave(&self, channel_id: &Id, user_id: &Id) -> Option<Id> {
        let mut inner = self.lock();
        let ch = inner.get_mut(channel_id)?;
        let entry = ch.members.get(user_id)?;
        if entry.local {
            return None;
        }
        ch.members.remove(user_id);
        let workspace_id = ch.workspace_id.clone();
        if ch.members.is_empty() {
            inner.remove(channel_id);
        }
        Some(workspace_id)
    }

    pub fn merge_state(&self, channel_id: &Id, workspace_id: &Id, members: Vec<VoiceMember>) {
        let mut inner = self.lock();
        let ch = Self::channel(&mut inner, channel_id, workspace_id);
        for member in members {
            if ch
                .members
                .get(&member.user.id)
                .map(|entry| entry.local)
                .unwrap_or(false)
            {
                continue;
            }
            ch.members.insert(
                member.user.id.clone(),
                Entry {
                    member,
                    local: false,
                    last_seen: Instant::now(),
                },
            );
        }
    }

    pub fn members_of(&self, channel_id: &Id) -> Vec<VoiceMember> {
        let inner = self.lock();
        inner
            .get(channel_id)
            .map(|ch| ch.members.values().map(|e| e.member.clone()).collect())
            .unwrap_or_default()
    }

    pub fn presence(&self, channel_id: &Id) -> Option<VoicePresence> {
        let inner = self.lock();
        inner
            .get(channel_id)
            .map(|ch| presence_of(&ch.workspace_id, channel_id, ch))
    }

    pub fn snapshot(&self) -> Vec<VoicePresence> {
        let inner = self.lock();
        inner
            .iter()
            .map(|(channel_id, ch)| presence_of(&ch.workspace_id, channel_id, ch))
            .collect()
    }

    pub fn local_channels(&self) -> Vec<Id> {
        let inner = self.lock();
        inner
            .iter()
            .filter(|(_, ch)| ch.members.values().any(|e| e.local))
            .map(|(channel_id, _)| channel_id.clone())
            .collect()
    }

    pub fn local_member(&self, channel_id: &Id) -> Option<VoiceMember> {
        let inner = self.lock();
        inner
            .get(channel_id)
            .and_then(|ch| ch.members.values().find(|e| e.local).map(|e| e.member.clone()))
    }

    pub fn active_voice_channel(&self) -> Option<Id> {
        let inner = self.lock();
        inner
            .iter()
            .find(|(_, ch)| ch.members.values().any(|e| e.local))
            .map(|(channel_id, _)| channel_id.clone())
    }

    pub fn expire_stale(&self) -> Vec<(Id, Id, Id)> {
        let now = Instant::now();
        let mut expired = Vec::new();
        let mut inner = self.lock();
        let mut empty = Vec::new();
        for (channel_id, ch) in inner.iter_mut() {
            let workspace_id = ch.workspace_id.clone();
            let stale: Vec<Id> = ch
                .members
                .iter()
                .filter(|(_, e)| !e.local && now.duration_since(e.last_seen) > MEMBER_TTL)
                .map(|(id, _)| id.clone())
                .collect();
            for user_id in stale {
                ch.members.remove(&user_id);
                expired.push((channel_id.clone(), workspace_id.clone(), user_id));
            }
            if ch.members.is_empty() {
                empty.push(channel_id.clone());
            }
        }
        for channel_id in empty {
            inner.remove(&channel_id);
        }
        expired
    }
}

fn presence_of(workspace_id: &Id, channel_id: &Id, ch: &Channel) -> VoicePresence {
    let mut entries: Vec<(&bool, &VoiceMember)> = ch
        .members
        .values()
        .map(|e| (&e.local, &e.member))
        .collect();
    entries.sort_by(|a, b| {
        b.0.cmp(a.0)
            .then_with(|| a.1.user.display_name.cmp(&b.1.user.display_name))
    });
    let members = entries.into_iter().map(|(_, m)| m.clone()).collect();
    VoicePresence {
        channel_id: channel_id.clone(),
        workspace_id: workspace_id.clone(),
        members,
    }
}
