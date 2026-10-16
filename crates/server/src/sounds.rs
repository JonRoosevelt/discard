use std::path::PathBuf;
use std::sync::Arc;

use serde::{Deserialize, Serialize};
use shared::Sound;
use tokio::sync::Mutex;

pub fn builtin() -> Vec<Sound> {
    let entries = [
        ("pop", "Pop", "/sounds/pop.mp3", "🔊"),
        ("boom", "Boom", "/sounds/boom.mp3", "💥"),
        ("airhorn", "Airhorn", "/sounds/airhorn.mp3", "📢"),
        ("bell", "Bell", "/sounds/bell.mp3", "🔔"),
        ("drum", "Drum", "/sounds/drum.mp3", "🥁"),
        ("applause", "Applause", "/sounds/applause.mp3", "👏"),
        ("tada", "Tada", "/sounds/tada.mp3", "🎉"),
    ];
    entries
        .into_iter()
        .map(|(id, name, url, emoji)| Sound {
            id: id.to_string(),
            name: name.to_string(),
            url: url.to_string(),
            emoji: emoji.to_string(),
            builtin: true,
            owner: String::new(),
            hash: String::new(),
        })
        .collect()
}

pub const MAX_BYTES: usize = 1024 * 1024;
const MAX_SOUNDS: usize = 100;

pub fn valid(sound: &Sound) -> bool {
    let name = sound.name.trim();
    !name.is_empty()
        && name.chars().count() <= 32
        && sound.emoji.len() <= 16
        && !sound.owner.is_empty()
        && sound.id.len() <= 64
        && !builtin().iter().any(|b| b.id == sound.id)
        && sound.hash.len() == 64
        && sound.hash.bytes().all(|b| b.is_ascii_hexdigit())
}

#[derive(Default, Serialize, Deserialize)]
struct Db {
    #[serde(default)]
    sounds: Vec<Sound>,
}

impl Db {
    fn merge(&mut self, sound: Sound) -> bool {
        if let Some(existing) = self.sounds.iter_mut().find(|s| s.id == sound.id) {
            if existing.owner != sound.owner || *existing == sound {
                return false;
            }
            *existing = sound;
            return true;
        }
        if self.sounds.len() >= MAX_SOUNDS {
            return false;
        }
        self.sounds.push(sound);
        true
    }

    fn remove(&mut self, id: &str, owner: &str) -> bool {
        let before = self.sounds.len();
        self.sounds.retain(|s| !(s.id == id && s.owner == owner));
        self.sounds.len() != before
    }
}

pub enum Denied {
    NotFound,
    NotOwner,
}

pub struct SoundStore {
    path: PathBuf,
    db: Mutex<Db>,
}

impl SoundStore {
    pub fn load(path: PathBuf, me: &str) -> Arc<Self> {
        let mut db: Db = std::fs::read(&path)
            .ok()
            .and_then(|bytes| serde_json::from_slice(&bytes).ok())
            .unwrap_or_default();
        for s in db.sounds.iter_mut().filter(|s| s.owner.is_empty()) {
            s.owner = me.to_string();
            s.hash = s.id.clone();
        }
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

    pub async fn uploads(&self) -> Vec<Sound> {
        self.db.lock().await.sounds.clone()
    }

    pub async fn list(&self) -> Vec<Sound> {
        let mut sounds = builtin();
        sounds.extend(self.uploads().await);
        sounds
    }

    pub async fn get(&self, id: &str) -> Option<Sound> {
        self.db.lock().await.sounds.iter().find(|s| s.id == id).cloned()
    }

    pub async fn merge(&self, sound: Sound) -> bool {
        let mut db = self.db.lock().await;
        let changed = db.merge(sound);
        if changed {
            Self::persist(&self.path, &db);
        }
        changed
    }

    pub async fn remove(&self, id: &str, owner: &str) -> bool {
        let mut db = self.db.lock().await;
        let changed = db.remove(id, owner);
        if changed {
            Self::persist(&self.path, &db);
        }
        changed
    }

    pub async fn edit(&self, id: &str, me: &str, name: String, emoji: String) -> Result<Sound, Denied> {
        let mut sound = self.get(id).await.ok_or(Denied::NotFound)?;
        if sound.owner != me {
            return Err(Denied::NotOwner);
        }
        sound.name = name;
        sound.emoji = emoji;
        self.merge(sound.clone()).await;
        Ok(sound)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sound(id: &str, owner: &str, name: &str) -> Sound {
        Sound {
            id: id.into(),
            name: name.into(),
            url: String::new(),
            emoji: String::new(),
            builtin: false,
            owner: owner.into(),
            hash: "a".repeat(64),
        }
    }

    #[test]
    fn only_owner_can_change_or_remove() {
        let mut db = Db::default();
        assert!(db.merge(sound("1", "alice", "boom")));
        assert!(!db.merge(sound("1", "bob", "hijack")));
        assert!(!db.merge(sound("1", "alice", "boom")));
        assert!(db.merge(sound("1", "alice", "renamed")));
        assert_eq!(db.sounds[0].name, "renamed");
        assert!(!db.remove("1", "bob"));
        assert!(db.remove("1", "alice"));
        assert!(db.sounds.is_empty());
    }

    #[test]
    fn rejects_bad_metadata() {
        assert!(valid(&sound("1", "alice", "ok")));
        assert!(!valid(&sound("pop", "alice", "ok")));
        assert!(!valid(&sound("1", "alice", " ")));
        let mut s = sound("1", "alice", "ok");
        s.hash = "zz".into();
        assert!(!valid(&s));
    }
}
