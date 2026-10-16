use std::ffi::OsString;
use std::path::{Path, PathBuf};
use std::sync::Mutex;
use std::time::{Duration, Instant};

use anyhow::{anyhow, bail, ensure, Context, Result};
use axum::extract::{Query, State};
use axum::http::StatusCode;
use axum::Json;
use serde::Deserialize;
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use tokio::io::AsyncWriteExt;
use tokio::process::Command;

use crate::error::{AppError, AppResult};
use crate::state::AppState;

pub const CURRENT: &str = match option_env!("DISCARD_VERSION") {
    Some(v) => v,
    None => "dev",
};

const LATEST_URL: &str = "https://api.github.com/repos/JonRoosevelt/discard/releases/latest";
const RELEASES_URL: &str = "https://github.com/JonRoosevelt/discard/releases";
const DOWNLOAD_PREFIX: &str = "https://github.com/JonRoosevelt/discard/releases/download/";
const MAX_ASSET: u64 = 300 * 1024 * 1024;
const OK_TTL: Duration = Duration::from_secs(3600);
const ERR_TTL: Duration = Duration::from_secs(300);

#[derive(Clone, Deserialize)]
struct Release {
    tag_name: String,
    html_url: String,
    assets: Vec<Asset>,
}

#[derive(Clone, Deserialize)]
struct Asset {
    name: String,
    size: u64,
    digest: Option<String>,
    browser_download_url: String,
}

#[derive(Clone)]
enum Status {
    Idle,
    Downloading,
    Installing,
    Restarting,
    Failed(String),
}

struct Cached {
    at: Instant,
    ttl: Duration,
    release: Option<Release>,
}

static CACHE: Mutex<Option<Cached>> = Mutex::new(None);
static STATUS: Mutex<Status> = Mutex::new(Status::Idle);

fn set_status(s: Status) {
    *STATUS.lock().unwrap() = s;
}

fn parse_version(s: &str) -> Option<[u64; 3]> {
    let core = s.trim_start_matches('v').split(['-', '+']).next()?;
    let mut parts = core.split('.');
    let v = [
        parts.next()?.parse().ok()?,
        parts.next()?.parse().ok()?,
        parts.next()?.parse().ok()?,
    ];
    parts.next().is_none().then_some(v)
}

fn is_newer(current: &str, latest: &str) -> bool {
    matches!((parse_version(current), parse_version(latest)), (Some(c), Some(l)) if l > c)
}

fn pick_asset<'a>(assets: &'a [Asset], os: &str, arch: &str) -> Option<&'a Asset> {
    let suffix = match os {
        "macos" => "-macos.dmg".to_string(),
        "linux" => format!("-linux-{arch}.tar.gz"),
        _ => return None,
    };
    assets.iter().find(|a| a.name.ends_with(&suffix))
}

fn expected_sha256(asset: &Asset) -> Option<String> {
    let hex = asset.digest.as_deref()?.strip_prefix("sha256:")?;
    (hex.len() == 64).then(|| hex.to_ascii_lowercase())
}

fn client() -> Result<reqwest::Client> {
    // Err just means a provider is already installed process-wide.
    let _ = rustls::crypto::ring::default_provider().install_default();
    reqwest::Client::builder()
        .user_agent(format!("discard/{CURRENT}"))
        .connect_timeout(Duration::from_secs(10))
        .redirect(reqwest::redirect::Policy::custom(|attempt| {
            if attempt.url().scheme() == "https" && attempt.previous().len() < 5 {
                attempt.follow()
            } else {
                attempt.stop()
            }
        }))
        .build()
        .map_err(Into::into)
}

async fn fetch_latest() -> Result<Release> {
    let resp = client()?
        .get(LATEST_URL)
        .header("Accept", "application/vnd.github+json")
        .timeout(Duration::from_secs(10))
        .send()
        .await?
        .error_for_status()?;
    Ok(serde_json::from_slice(&resp.bytes().await?)?)
}

async fn latest_release(offline: bool, force: bool) -> Option<Release> {
    if offline {
        return None;
    }
    if !force {
        if let Some(c) = CACHE.lock().unwrap().as_ref() {
            if c.at.elapsed() < c.ttl {
                return c.release.clone();
            }
        }
    }
    let fetched = fetch_latest()
        .await
        .map_err(|e| tracing::debug!("update check failed: {e}"))
        .ok();
    let mut cache = CACHE.lock().unwrap();
    let release = fetched.or_else(|| cache.as_ref().and_then(|c| c.release.clone()));
    *cache = Some(Cached {
        at: Instant::now(),
        ttl: if release.is_some() { OK_TTL } else { ERR_TTL },
        release: release.clone(),
    });
    release
}

enum Target {
    App(PathBuf),
    Binary(PathBuf),
}

fn install_target(exe: &Path, os: &str) -> Result<Target> {
    let name = |p: &Path| p.file_name().map(|n| n.to_string_lossy().into_owned());
    match os {
        "macos" => {
            let app = exe
                .parent()
                .filter(|d| name(d).as_deref() == Some("MacOS"))
                .and_then(Path::parent)
                .filter(|d| name(d).as_deref() == Some("Contents"))
                .and_then(Path::parent)
                .filter(|d| d.extension().is_some_and(|e| e == "app"));
            app.map(|a| Target::App(a.to_path_buf()))
                .ok_or_else(|| anyhow!("not running from an installed Discard.app; update manually"))
        }
        "linux" => {
            ensure!(
                name(exe).as_deref() == Some("discard")
                    && !exe.components().any(|c| c.as_os_str() == "target"),
                "not running from an installed discard binary; update manually"
            );
            Ok(Target::Binary(exe.to_path_buf()))
        }
        _ => bail!("in-app update is not supported on {os}"),
    }
}

fn current_target() -> Result<Target> {
    let exe = std::env::current_exe()?.canonicalize()?;
    install_target(&exe, std::env::consts::OS)
}

async fn run(program: &str, args: &[&Path]) -> Result<()> {
    let out = Command::new(program).args(args).output().await?;
    ensure!(
        out.status.success(),
        "{program} failed: {}",
        String::from_utf8_lossy(&out.stderr).trim()
    );
    Ok(())
}

async fn download(asset: &Asset, dest: &Path) -> Result<()> {
    ensure!(
        asset.browser_download_url.starts_with(DOWNLOAD_PREFIX),
        "unexpected download URL"
    );
    ensure!(asset.size > 0 && asset.size <= MAX_ASSET, "unexpected asset size");
    let mut resp = client()?
        .get(&asset.browser_download_url)
        .timeout(Duration::from_secs(900))
        .send()
        .await?
        .error_for_status()?;
    if let Some(len) = resp.content_length() {
        ensure!(len == asset.size, "Content-Length {len} differs from asset size {}", asset.size);
    }
    let mut file = tokio::fs::File::create(dest).await?;
    let mut hasher = Sha256::new();
    let mut got = 0u64;
    while let Some(chunk) = resp.chunk().await? {
        got += chunk.len() as u64;
        ensure!(got <= asset.size, "download larger than announced");
        hasher.update(&chunk);
        file.write_all(&chunk).await?;
    }
    file.flush().await?;
    ensure!(got == asset.size, "download truncated ({got} of {} bytes)", asset.size);
    if let Some(expected) = expected_sha256(asset) {
        ensure!(hex::encode(hasher.finalize()) == expected, "sha256 mismatch");
    }
    Ok(())
}

async fn install_macos(dmg: &Path, work: &Path, app: &Path) -> Result<()> {
    let mnt = work.join("mnt");
    tokio::fs::create_dir_all(&mnt).await?;
    run(
        "hdiutil",
        &[
            Path::new("attach"),
            Path::new("-nobrowse"),
            Path::new("-readonly"),
            Path::new("-mountpoint"),
            &mnt,
            dmg,
        ],
    )
    .await?;
    let swapped = swap_app(&mnt.join("Discard.app"), app).await;
    let _ = run("hdiutil", &[Path::new("detach"), Path::new("-force"), &mnt]).await;
    swapped
}

async fn swap_app(src: &Path, app: &Path) -> Result<()> {
    ensure!(
        src.join("Contents/MacOS/discard-bin").is_file(),
        "dmg does not contain Discard.app"
    );
    let sibling = |suffix: &str| {
        let mut n = app.file_name().unwrap_or_default().to_os_string();
        n.push(suffix);
        app.with_file_name(n)
    };
    let (staged, old) = (sibling(".update"), sibling(".old"));
    let _ = tokio::fs::remove_dir_all(&staged).await;
    let _ = tokio::fs::remove_dir_all(&old).await;
    run("ditto", &[src, &staged]).await.context("copying the new bundle")?;
    let _ = run("xattr", &[Path::new("-dr"), Path::new("com.apple.quarantine"), &staged]).await;
    tokio::fs::rename(app, &old).await?;
    if let Err(e) = tokio::fs::rename(&staged, app).await {
        let _ = tokio::fs::rename(&old, app).await;
        return Err(e.into());
    }
    let _ = tokio::fs::remove_dir_all(&old).await;
    Ok(())
}

async fn install_linux(archive: &Path, stem: &str, work: &Path, exe: &Path) -> Result<()> {
    let out = work.join("extracted");
    tokio::fs::create_dir_all(&out).await?;
    run("tar", &[Path::new("-xzf"), archive, Path::new("-C"), &out]).await?;
    let new = out.join(stem).join("discard");
    ensure!(
        tokio::fs::symlink_metadata(&new).await?.is_file(),
        "archive does not contain a discard binary"
    );
    let staged = exe.with_file_name(".discard.update");
    tokio::fs::copy(&new, &staged).await?;
    if let Err(e) = tokio::fs::rename(&staged, exe).await {
        let _ = tokio::fs::remove_file(&staged).await;
        return Err(e.into());
    }
    Ok(())
}

fn relaunch(target: &Target) {
    if cfg!(debug_assertions) && std::env::var_os("DISCARD_UPDATE_NO_RELAUNCH").is_some() {
        return;
    }
    let cmd: Vec<OsString> = match target {
        Target::App(app) => vec!["open".into(), "-n".into(), app.into()],
        Target::Binary(exe) => {
            let launcher = exe.with_file_name("discard-app");
            if launcher.is_file() {
                vec![launcher.into()]
            } else {
                std::iter::once(exe.into()).chain(std::env::args_os().skip(1)).collect()
            }
        }
    };
    let _ = std::process::Command::new("sh")
        .args(["-c", "sleep 1; exec \"$@\"", "sh"])
        .args(cmd)
        .stdin(std::process::Stdio::null())
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .spawn();
}

async fn perform(offline: bool, target: &Target, work: &Path) -> Result<()> {
    let release = latest_release(offline, false)
        .await
        .context("could not fetch the latest release")?;
    ensure!(
        CURRENT == "dev" || is_newer(CURRENT, release.tag_name.trim_start_matches('v')),
        "already on the latest version"
    );
    let asset = pick_asset(&release.assets, std::env::consts::OS, std::env::consts::ARCH)
        .context("no release asset for this OS/architecture")?;
    let archive = work.join(&asset.name);
    download(asset, &archive).await?;
    set_status(Status::Installing);
    match target {
        Target::App(app) => install_macos(&archive, work, app).await,
        Target::Binary(exe) => {
            let stem = asset.name.strip_suffix(".tar.gz").unwrap_or(&asset.name);
            install_linux(&archive, stem, work, exe).await
        }
    }
}

#[derive(Deserialize)]
pub struct VersionQuery {
    #[serde(default)]
    refresh: bool,
}

pub async fn version(State(state): State<AppState>, Query(q): Query<VersionQuery>) -> Json<Value> {
    let release = latest_release(state.config.offline, q.refresh).await;
    let latest = release
        .as_ref()
        .map(|r| r.tag_name.trim_start_matches('v').to_string());
    let update_available = latest
        .as_deref()
        .is_some_and(|l| is_newer(CURRENT, l) || (q.refresh && parse_version(CURRENT).is_none()));
    let (status, error) = match STATUS.lock().unwrap().clone() {
        Status::Idle => ("idle", None),
        Status::Downloading => ("downloading", None),
        Status::Installing => ("installing", None),
        Status::Restarting => ("restarting", None),
        Status::Failed(e) => ("error", Some(e)),
    };
    Json(json!({
        "current": CURRENT,
        "latest": latest,
        "update_available": update_available,
        "notes_url": release.map_or_else(|| RELEASES_URL.to_string(), |r| r.html_url),
        "installable": current_target().is_ok(),
        "status": status,
        "error": error,
    }))
}

pub async fn install(State(state): State<AppState>) -> AppResult<(StatusCode, Json<Value>)> {
    if state.config.offline {
        return Err(AppError::BadRequest("offline mode: updates are disabled".into()));
    }
    let target = current_target().map_err(|e| AppError::BadRequest(e.to_string()))?;
    {
        let mut status = STATUS.lock().unwrap();
        if matches!(
            *status,
            Status::Downloading | Status::Installing | Status::Restarting
        ) {
            return Err(AppError::BadRequest("an update is already in progress".into()));
        }
        *status = Status::Downloading;
    }
    let offline = state.config.offline;
    let work = state.config.data_dir.join("update");
    tokio::spawn(async move {
        let _ = tokio::fs::remove_dir_all(&work).await;
        let result = match tokio::fs::create_dir_all(&work).await {
            Ok(()) => perform(offline, &target, &work).await,
            Err(e) => Err(e.into()),
        };
        let _ = tokio::fs::remove_dir_all(&work).await;
        match result {
            Ok(()) => {
                set_status(Status::Restarting);
                relaunch(&target);
                tokio::time::sleep(Duration::from_millis(800)).await;
                std::process::exit(0);
            }
            Err(e) => {
                tracing::warn!("update failed: {e:#}");
                set_status(Status::Failed(format!("{e:#}")));
            }
        }
    });
    Ok((StatusCode::ACCEPTED, Json(json!({ "status": "downloading" }))))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn asset(name: &str) -> Asset {
        Asset {
            name: name.into(),
            size: 1,
            digest: None,
            browser_download_url: String::new(),
        }
    }

    #[test]
    fn semver_compare() {
        assert!(is_newer("0.1.0", "v0.2.0"));
        assert!(is_newer("0.2.0", "0.10.0"));
        assert!(is_newer("1.9.9", "2.0.0"));
        assert!(!is_newer("0.2.0", "0.2.0"));
        assert!(!is_newer("0.3.0", "v0.2.9"));
        assert!(!is_newer("dev", "v9.9.9"));
        assert!(!is_newer("0.1.0", "garbage"));
        assert_eq!(parse_version("v1.2.3-rc1+b"), Some([1, 2, 3]));
        assert_eq!(parse_version("1.2"), None);
        assert_eq!(parse_version("1.2.3.4"), None);
    }

    #[test]
    fn asset_selection() {
        let assets = [
            asset("discard-0.2.0-linux-x86_64.tar.gz"),
            asset("discard-0.2.0-linux-aarch64.tar.gz"),
            asset("Discard-0.2.0-macos.dmg"),
        ];
        let pick = |os, arch| pick_asset(&assets, os, arch).map(|a| a.name.as_str());
        assert_eq!(pick("macos", "aarch64"), Some("Discard-0.2.0-macos.dmg"));
        assert_eq!(pick("linux", "aarch64"), Some("discard-0.2.0-linux-aarch64.tar.gz"));
        assert_eq!(pick("linux", "x86_64"), Some("discard-0.2.0-linux-x86_64.tar.gz"));
        assert_eq!(pick("linux", "riscv64"), None);
        assert_eq!(pick("windows", "x86_64"), None);
    }

    #[test]
    fn digest_parsing() {
        let mut a = asset("x");
        assert_eq!(expected_sha256(&a), None);
        a.digest = Some(format!("sha256:{}", "AB".repeat(32)));
        assert_eq!(expected_sha256(&a), Some("ab".repeat(32)));
        a.digest = Some("sha1:abc".into());
        assert_eq!(expected_sha256(&a), None);
    }

    #[test]
    fn target_detection() {
        let t = |p: &str, os| install_target(Path::new(p), os).is_ok();
        assert!(t("/Applications/Discard.app/Contents/MacOS/discard-bin", "macos"));
        assert!(!t("/repo/target/debug/discard", "macos"));
        assert!(t("/home/u/.local/bin/discard", "linux"));
        assert!(!t("/repo/target/release/discard", "linux"));
        assert!(!t("/usr/bin/other", "linux"));
        assert!(!t("C:/discard.exe", "windows"));
    }

    #[tokio::test]
    async fn linux_swap_replaces_binary_and_rejects_bad_archive() {
        let root = std::env::temp_dir().join(format!("discard-update-test-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        let stem = "discard-9.9.9-linux-x86_64";
        let (pkg, work, bin) = (root.join("pkg").join(stem), root.join("work"), root.join("bin"));
        for d in [&pkg, &work, &bin] {
            std::fs::create_dir_all(d).unwrap();
        }
        std::fs::write(pkg.join("discard"), "new").unwrap();
        let exe = bin.join("discard");
        std::fs::write(&exe, "old").unwrap();
        let archive = root.join("a.tar.gz");
        let made = std::process::Command::new("tar")
            .arg("-czf")
            .arg(&archive)
            .arg("-C")
            .arg(root.join("pkg"))
            .arg(stem)
            .status()
            .unwrap();
        assert!(made.success());

        install_linux(&archive, stem, &work, &exe).await.unwrap();
        assert_eq!(std::fs::read_to_string(&exe).unwrap(), "new");
        assert!(!bin.join(".discard.update").exists());

        std::fs::write(&archive, "not gzip").unwrap();
        assert!(install_linux(&archive, stem, &work, &exe).await.is_err());
        assert_eq!(std::fs::read_to_string(&exe).unwrap(), "new");
        let _ = std::fs::remove_dir_all(&root);
    }
}
