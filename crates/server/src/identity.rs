use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::sync::RwLock;

use iroh::SecretKey;
use serde::{Deserialize, Serialize};
use shared::Identity;

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
struct Profile {
    #[serde(default)]
    display_name: String,
}

pub struct LocalIdentity {
    secret_key: RwLock<SecretKey>,
    key_path: PathBuf,
    profile_path: PathBuf,
    profile: RwLock<Profile>,
}

fn write_private(path: &Path, bytes: &[u8]) -> anyhow::Result<()> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    std::fs::write(path, bytes)?;
    let mut perms = std::fs::metadata(path)?.permissions();
    perms.set_mode(0o600);
    std::fs::set_permissions(path, perms)?;
    Ok(())
}

impl LocalIdentity {
    pub fn load_or_create(data_dir: &Path) -> anyhow::Result<Self> {
        std::fs::create_dir_all(data_dir)?;
        let key_path = data_dir.join("identity.key");
        let profile_path = data_dir.join("profile.json");

        let secret_key = if key_path.exists() {
            let bytes = std::fs::read(&key_path)?;
            let arr: [u8; 32] = bytes
                .as_slice()
                .try_into()
                .map_err(|_| anyhow::anyhow!("identity.key must be 32 bytes"))?;
            SecretKey::from_bytes(&arr)
        } else {
            let key = SecretKey::generate();
            write_private(&key_path, &key.to_bytes())?;
            key
        };

        let profile = if profile_path.exists() {
            serde_json::from_slice(&std::fs::read(&profile_path)?).unwrap_or_default()
        } else {
            Profile::default()
        };

        Ok(Self {
            secret_key: RwLock::new(secret_key),
            key_path,
            profile_path,
            profile: RwLock::new(profile),
        })
    }

    pub fn public_id(&self) -> String {
        self.secret_key
            .read()
            .map(|k| k.public().to_string())
            .unwrap_or_default()
    }

    pub fn secret_key(&self) -> SecretKey {
        self.secret_key
            .read()
            .map(|k| k.clone())
            .unwrap_or_else(|_| SecretKey::generate())
    }

    pub fn exists(&self) -> bool {
        self.profile_path.exists()
    }

    pub fn display_name(&self) -> String {
        self.profile
            .read()
            .map(|p| p.display_name.clone())
            .unwrap_or_default()
    }

    pub fn set_display_name(&self, name: &str) -> anyhow::Result<()> {
        {
            let mut profile = self
                .profile
                .write()
                .map_err(|_| anyhow::anyhow!("identity lock poisoned"))?;
            profile.display_name = name.to_string();
        }
        let bytes = serde_json::to_vec_pretty(
            &*self
                .profile
                .read()
                .map_err(|_| anyhow::anyhow!("identity lock poisoned"))?,
        )?;
        if let Some(parent) = self.profile_path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        std::fs::write(&self.profile_path, bytes)?;
        Ok(())
    }

    pub fn shared(&self) -> Identity {
        Identity {
            public_id: self.public_id(),
            display_name: self.display_name(),
            has_backup: self.exists(),
        }
    }

    pub fn backup_hex(&self) -> String {
        hex::encode(self.secret_key().to_bytes())
    }

    pub fn import(&self, backup_hex: &str) -> anyhow::Result<()> {
        let raw = hex::decode(backup_hex.trim())
            .map_err(|_| anyhow::anyhow!("backup is not valid hex"))?;
        let arr: [u8; 32] = raw
            .as_slice()
            .try_into()
            .map_err(|_| anyhow::anyhow!("backup must be 32 bytes"))?;
        let key = SecretKey::from_bytes(&arr);
        write_private(&self.key_path, &key.to_bytes())?;
        let mut guard = self
            .secret_key
            .write()
            .map_err(|_| anyhow::anyhow!("identity lock poisoned"))?;
        *guard = key;
        Ok(())
    }
}
