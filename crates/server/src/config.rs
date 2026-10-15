use std::path::PathBuf;

#[derive(Debug, Clone)]
pub struct Config {
    pub addr: String,
    pub data_dir: PathBuf,
    pub offline: bool,
    pub no_relay: bool,
    pub no_open: bool,
    pub token_override: Option<String>,
}

fn env_bool(name: &str) -> bool {
    std::env::var(name)
        .map(|v| v == "1" || v.eq_ignore_ascii_case("true") || v.eq_ignore_ascii_case("yes"))
        .unwrap_or(false)
}

impl Config {
    pub fn from_env() -> Self {
        let addr = std::env::var("DISCARD_ADDR").unwrap_or_else(|_| "127.0.0.1:8787".to_string());
        let data_dir = std::env::var("DISCARD_DATA_DIR")
            .map(PathBuf::from)
            .unwrap_or_else(|_| PathBuf::from("data"));
        let offline = env_bool("DISCARD_OFFLINE");
        let no_relay = env_bool("DISCARD_NO_RELAY");
        let no_open = env_bool("DISCARD_NO_OPEN");
        let token_override = std::env::var("DISCARD_TOKEN").ok().filter(|v| !v.is_empty());
        Config {
            addr,
            data_dir,
            offline,
            no_relay,
            no_open,
            token_override,
        }
    }

    pub fn workspaces_path(&self) -> PathBuf {
        self.data_dir.join("workspaces.json")
    }

    pub fn blobs_dir(&self) -> PathBuf {
        self.data_dir.join("blobs")
    }

    #[allow(dead_code)]
    pub fn web_dist(&self) -> PathBuf {
        if let Ok(dir) = std::env::var("DISCARD_WEB_DIST") {
            return PathBuf::from(dir);
        }
        let local = PathBuf::from("web/dist");
        if local.exists() {
            return local;
        }
        let next_to_exe = std::env::current_exe()
            .ok()
            .and_then(|p| p.parent().map(|d| d.join("web/dist")))
            .unwrap_or_else(|| PathBuf::from("web/dist"));
        next_to_exe
    }

    pub fn display_host(&self) -> String {
        if self.addr.starts_with("0.0.0.0") || self.addr.starts_with("[::]") {
            let port = self.addr.rsplit(':').next().unwrap_or("8787");
            format!("127.0.0.1:{port}")
        } else {
            self.addr.clone()
        }
    }
}
