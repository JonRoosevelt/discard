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
        })
        .collect()
}

#[derive(Default, Serialize, Deserialize)]
struct Db {
    #[serde(default)]
    sounds: Vec<Sound>,
}

pub struct SoundStore {
    path: PathBuf,
    db: Mutex<Db>,
}

impl SoundStore {
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

    pub async fn uploads(&self) -> Vec<Sound> {
        self.db.lock().await.sounds.clone()
    }

    pub async fn list(&self) -> Vec<Sound> {
        let mut sounds = builtin();
        sounds.extend(self.uploads().await);
        sounds
    }

    pub async fn add(&self, sound: Sound) -> Sound {
        let mut db = self.db.lock().await;
        db.sounds.retain(|s| s.id != sound.id);
        db.sounds.push(sound.clone());
        Self::persist(&self.path, &db);
        sound
    }
}
