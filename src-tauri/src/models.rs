use std::{
    os::unix::fs::PermissionsExt,
    path::PathBuf,
    sync::{atomic::Ordering, Arc},
    time::{Duration, Instant},
};

use anyhow::{anyhow, Result};
use md5::Md5;
use serde::Serialize;
use sha2::{Digest, Sha256};
use tauri::{AppHandle, Emitter, Manager};
use tokio::io::AsyncWriteExt;

use crate::{error::AppError, state::AppState};

pub struct ModelSpec {
    pub key: &'static str,
    pub label: &'static str,
    pub asset: &'static str,
    pub sha256: Option<&'static str>,
    pub md5: Option<&'static str>,
    pub input_size: u32,
    pub mean: [f32; 3],
    pub std: [f32; 3],
    pub sigmoid: bool,
    pub clip: bool,
    pub approx_bytes: u64,
}

pub const MODELS: &[ModelSpec] = &[
    ModelSpec {
        key: "bria-rmbg",
        label: "Bria RMBG 2.0 (best)",
        asset: "bria-rmbg-2.0.onnx",
        sha256: Some("5b486f08200f513f460da46dd701db5fbb47d79b4be4b708a19444bcd4e79958"),
        md5: None,
        input_size: 1024,
        mean: [0.485, 0.456, 0.406],
        std: [0.229, 0.224, 0.225],
        sigmoid: false,
        clip: false,
        approx_bytes: 1_024_331_469,
    },
    ModelSpec {
        key: "birefnet-portrait",
        label: "BiRefNet portrait",
        asset: "BiRefNet-portrait-epoch_150.onnx",
        sha256: None,
        md5: Some("c3a64a6abf20250d090cd055f12a3b67"),
        input_size: 1024,
        mean: [0.485, 0.456, 0.406],
        std: [0.229, 0.224, 0.225],
        sigmoid: true,
        clip: false,
        approx_bytes: 972_666_916,
    },
    ModelSpec {
        key: "isnet-general-use",
        label: "ISNet general use",
        asset: "isnet-general-use.onnx",
        sha256: None,
        md5: Some("fc16ebd8b0c10d971d3513d564d01e29"),
        input_size: 1024,
        mean: [0.5, 0.5, 0.5],
        std: [1.0, 1.0, 1.0],
        sigmoid: false,
        clip: false,
        approx_bytes: 178_648_008,
    },
    ModelSpec {
        key: "u2net",
        label: "U²-Net (fast)",
        asset: "u2net.onnx",
        sha256: None,
        md5: Some("60024c5c889badc19c04ad937298a77b"),
        input_size: 320,
        mean: [0.485, 0.456, 0.406],
        std: [0.229, 0.224, 0.225],
        sigmoid: false,
        clip: true,
        approx_bytes: 176_000_000,
    },
];

const BASE_URL: &str = "https://github.com/danielgatis/rembg/releases/download/v0.0.0";

pub fn spec(key: &str) -> Result<&'static ModelSpec> {
    MODELS
        .iter()
        .find(|m| m.key == key)
        .ok_or_else(|| anyhow!("unknown model: {key}"))
}

/// Shared, cross-user model cache — this is a personal macOS app, and
/// /Users/Shared is the standard world-writable location. One download then
/// serves every account on the machine. Falls back to per-user app data
/// when Shared isn't usable.
fn models_root(app: &AppHandle) -> PathBuf {
    let shared = PathBuf::from("/Users/Shared/noground/models");
    if ensure_writable_dir(&shared) {
        return shared;
    }
    app.path()
        .app_data_dir()
        .map(|d| d.join("models"))
        .unwrap_or(shared)
}

fn ensure_writable_dir(dir: &std::path::Path) -> bool {
    if std::fs::create_dir_all(dir).is_err() {
        return false;
    }
    // The first creator owns the dir; open it up so other accounts can use
    // it. Sticky (like /tmp): only a file's owner can rename over it, so a
    // different account can't swap the model out from under this app.
    let _ = std::fs::set_permissions(dir, std::fs::Permissions::from_mode(0o1777));
    let probe = dir.join(".writable-probe");
    let ok = std::fs::OpenOptions::new()
        .write(true)
        .create(true)
        .truncate(false)
        .open(&probe)
        .is_ok();
    let _ = std::fs::remove_file(&probe);
    ok
}

/// First existing location wins: the shared/per-user cache this app manages,
/// the legacy app-data location, the rembg cache (probed read-only, same
/// `<dir>/<key>/<key>.onnx` layout), legacy flat cache.
pub fn resolve_model_path(app: &AppHandle, key: &str) -> Option<PathBuf> {
    let mut candidates: Vec<PathBuf> = vec![models_root(app).join(key).join(format!("{key}.onnx"))];
    if let Ok(app_data) = app.path().app_data_dir() {
        candidates.push(app_data.join("models").join(key).join(format!("{key}.onnx")));
    }
    if let Some(home) = dirs::home_dir() {
        candidates.push(home.join(".rembg").join("models").join(key).join(format!("{key}.onnx")));
        candidates.push(home.join(".u2net").join(format!("{key}.onnx")));
    }
    candidates.into_iter().find(|p| p.exists())
}

fn download_target(app: &AppHandle, key: &str) -> Result<PathBuf> {
    Ok(models_root(app).join(key).join(format!("{key}.onnx")))
}

#[derive(Debug, Clone, Serialize)]
#[serde(tag = "status", rename_all = "kebab-case")]
pub enum ModelState {
    Ready { path: String },
    InProgress,
}

pub fn ensure_model(app: &AppHandle, key: &str) -> Result<ModelState> {
    let spec = spec(key)?;
    if let Some(path) = resolve_model_path(app, key) {
        return Ok(ModelState::Ready {
            path: path.display().to_string(),
        });
    }

    let state = app.state::<AppState>();
    {
        let mut downloads = state.downloads.lock().unwrap();
        if downloads.contains_key(key) {
            return Ok(ModelState::InProgress);
        }
        downloads.insert(key.to_string(), Arc::new(Default::default()));
    }

    let target = download_target(app, key)?;
    let cancel = state.downloads.lock().unwrap()[key].clone();
    let app2 = app.clone();
    let key2 = key.to_string();
    let spec2 = spec;
    tauri::async_runtime::spawn(async move {
        let result = download_model(&app2, spec2, &target, &cancel).await;
        app2.state::<AppState>().downloads.lock().unwrap().remove(&key2);
        if let Err(e) = result {
            let _ = app2.emit(
                "noground://model-error",
                serde_json::json!({ "key": key2, "message": e.to_string() }),
            );
        }
    });

    Ok(ModelState::InProgress)
}

enum PinHasher {
    Sha256(Sha256),
    Md5(Md5),
}

impl PinHasher {
    fn new(spec: &ModelSpec) -> Self {
        if spec.sha256.is_some() {
            Self::Sha256(Sha256::new())
        } else {
            Self::Md5(Md5::new())
        }
    }

    fn update(&mut self, bytes: &[u8]) {
        match self {
            Self::Sha256(h) => h.update(bytes),
            Self::Md5(h) => h.update(bytes),
        }
    }

    fn finalize_hex(self) -> String {
        match self {
            Self::Sha256(h) => hex(h.finalize()),
            Self::Md5(h) => hex(h.finalize()),
        }
    }
}

fn hex(bytes: impl AsRef<[u8]>) -> String {
    bytes
        .as_ref()
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect::<String>()
}

async fn download_model(
    app: &AppHandle,
    spec: &ModelSpec,
    target: &std::path::Path,
    cancel: &std::sync::atomic::AtomicBool,
) -> Result<()> {
    // bounded connect/read: a stalled connection surfaces as an error instead
    // of hanging forever past the cancel flag
    let client = reqwest::Client::builder()
        .connect_timeout(Duration::from_secs(15))
        .read_timeout(Duration::from_secs(30))
        .build()?;
    let url = format!("{BASE_URL}/{}", spec.asset);
    let response = client.get(&url).send().await?.error_for_status()?;
    let total_bytes = response.content_length();
    // pid suffix: two accounts downloading the same model into the shared
    // cache must not fight over one .part file
    let part = target.with_extension(format!("onnx.{}.part", std::process::id()));
    if let Some(dir) = target.parent() {
        tokio::fs::create_dir_all(dir).await?;
        ensure_writable_dir(dir);
    }

    // any failure (network, checksum, cancel) leaves no orphan .part behind
    let result = stream_to_part(app, spec, response, total_bytes, &part, target, cancel).await;
    if result.is_err() {
        let _ = tokio::fs::remove_file(&part).await;
    }
    result
}

async fn stream_to_part(
    app: &AppHandle,
    spec: &ModelSpec,
    response: reqwest::Response,
    total_bytes: Option<u64>,
    part: &std::path::Path,
    target: &std::path::Path,
    cancel: &std::sync::atomic::AtomicBool,
) -> Result<()> {
    use futures_util::StreamExt;

    let mut hasher = PinHasher::new(spec);
    let mut file = tokio::io::BufWriter::new(tokio::fs::File::create(part).await?);
    let mut stream = response.bytes_stream();
    let mut downloaded: u64 = 0;
    let mut last_emit = Instant::now();

    while let Some(chunk) = stream.next().await {
        if cancel.load(Ordering::Relaxed) {
            return Err(AppError::Cancelled.into());
        }
        let chunk = chunk?;
        hasher.update(&chunk);
        downloaded += chunk.len() as u64;
        file.write_all(&chunk).await?;
        if last_emit.elapsed() >= Duration::from_millis(250) {
            last_emit = Instant::now();
            let _ = app.emit(
                "noground://model-progress",
                ModelProgress {
                    key: spec.key,
                    downloaded_bytes: downloaded,
                    total_bytes,
                    done: false,
                },
            );
        }
    }
    file.flush().await?;
    drop(file);

    let actual = hasher.finalize_hex();
    let expected = spec.sha256.or(spec.md5).unwrap_or_default();
    if !expected.is_empty() && actual != expected {
        return Err(anyhow!(
            "checksum mismatch for {} (expected {expected}, got {actual}) — download aborted",
            spec.key
        ));
    }

    tokio::fs::rename(part, target).await?;
    let _ = tokio::fs::set_permissions(target, std::fs::Permissions::from_mode(0o644)).await;
    let _ = app.emit(
        "noground://model-progress",
        ModelProgress {
            key: spec.key,
            downloaded_bytes: downloaded,
            total_bytes: Some(downloaded),
            done: true,
        },
    );
    Ok(())
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ModelProgress {
    pub key: &'static str,
    pub downloaded_bytes: u64,
    pub total_bytes: Option<u64>,
    pub done: bool,
}

pub fn cancel_model_download(state: &AppState, key: &str) -> bool {
    if let Some(flag) = state.downloads.lock().unwrap().get(key) {
        flag.store(true, Ordering::Relaxed);
        true
    } else {
        false
    }
}

pub fn list_models(app: &AppHandle) -> Vec<ModelInfo> {
    MODELS
        .iter()
        .map(|m| ModelInfo {
            key: m.key,
            label: m.label.to_string(),
            approx_bytes: m.approx_bytes,
            installed: resolve_model_path(app, m.key).is_some(),
        })
        .collect()
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ModelInfo {
    pub key: &'static str,
    pub label: String,
    pub approx_bytes: u64,
    pub installed: bool,
}
