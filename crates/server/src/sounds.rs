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
            version: 0,
            editor: String::new(),
            deleted: false,
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
            if (sound.version, &sound.editor) <= (existing.version, &existing.editor) {
                return false;
            }
            *existing = sound;
            return true;
        }
        if !sound.deleted && self.sounds.iter().filter(|s| !s.deleted).count() >= MAX_SOUNDS {
            return false;
        }
        self.sounds.push(sound);
        true
    }
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

    pub async fn all(&self) -> Vec<Sound> {
        self.db.lock().await.sounds.clone()
    }

    pub async fn list(&self) -> Vec<Sound> {
        let mut sounds = builtin();
        sounds.extend(self.all().await.into_iter().filter(|s| !s.deleted));
        sounds
    }

    pub async fn hash_in_use(&self, hash: &str) -> bool {
        self.db.lock().await.sounds.iter().any(|s| !s.deleted && s.hash == hash)
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

    pub async fn edit(&self, id: &str, me: &str, name: String, emoji: String) -> Option<Sound> {
        self.bump(id, me, |s| {
            s.name = name;
            s.emoji = emoji;
        })
        .await
    }

    pub async fn delete(&self, id: &str, me: &str) -> Option<Sound> {
        self.bump(id, me, |s| s.deleted = true).await
    }

    async fn bump(&self, id: &str, me: &str, f: impl FnOnce(&mut Sound)) -> Option<Sound> {
        let mut db = self.db.lock().await;
        let mut sound = db.sounds.iter().find(|s| s.id == id && !s.deleted)?.clone();
        f(&mut sound);
        sound.version += 1;
        sound.editor = me.to_string();
        db.merge(sound.clone());
        Self::persist(&self.path, &db);
        Some(sound)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sound(id: &str, editor: &str, version: u64, name: &str) -> Sound {
        Sound {
            id: id.into(),
            name: name.into(),
            url: String::new(),
            emoji: String::new(),
            builtin: false,
            owner: "alice".into(),
            hash: "a".repeat(64),
            version,
            editor: editor.into(),
            deleted: false,
        }
    }

    #[test]
    fn last_writer_wins() {
        let mut db = Db::default();
        assert!(db.merge(sound("1", "alice", 1, "v1")));
        assert!(db.merge(sound("1", "bob", 2, "newer")));
        assert!(!db.merge(sound("1", "alice", 1, "older")));
        assert!(!db.merge(sound("1", "bob", 2, "newer")));
        assert!(db.merge(sound("1", "carol", 2, "tie-break")));
        assert!(!db.merge(sound("1", "bob", 2, "loses tie")));
        assert_eq!(db.sounds[0].name, "tie-break");
    }

    #[test]
    fn tombstone_blocks_resurrection() {
        let mut db = Db::default();
        assert!(db.merge(sound("1", "alice", 1, "x")));
        let mut dead = sound("1", "bob", 2, "x");
        dead.deleted = true;
        assert!(db.merge(dead));
        assert!(!db.merge(sound("1", "alice", 1, "x")));
        assert!(db.sounds[0].deleted);
    }

    #[test]
    fn rejects_bad_metadata() {
        assert!(valid(&sound("1", "alice", 0, "ok")));
        assert!(!valid(&sound("pop", "alice", 0, "ok")));
        assert!(!valid(&sound("1", "alice", 0, " ")));
        let mut s = sound("1", "alice", 0, "ok");
        s.hash = "zz".into();
        assert!(!valid(&s));
    }
}
