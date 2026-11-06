use std::{
    path::PathBuf,
    sync::{atomic::Ordering, Arc},
    time::{Duration, Instant},
};

use anyhow::{anyhow, Context, Result};
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

/// First existing location wins: our app-data cache, the rembg cache
/// (probed read-only, same `<dir>/<key>/<key>.onnx` layout), legacy flat cache.
pub fn resolve_model_path(app: &AppHandle, key: &str) -> Option<PathBuf> {
    let mut candidates: Vec<PathBuf> = Vec::new();
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
    Ok(app
        .path()
        .app_data_dir()
        .context("no app data dir")?
        .join("models")
        .join(key)
        .join(format!("{key}.onnx")))
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
    use futures_util::StreamExt;

    let url = format!("{BASE_URL}/{}", spec.asset);
    let response = reqwest::get(&url).await?.error_for_status()?;
    let total_bytes = response.content_length();
    let part = target.with_extension("onnx.part");
    if let Some(dir) = target.parent() {
        tokio::fs::create_dir_all(dir).await?;
    }

    let mut hasher = PinHasher::new(spec);
    let mut file = tokio::io::BufWriter::new(tokio::fs::File::create(&part).await?);
    let mut stream = response.bytes_stream();
    let mut downloaded: u64 = 0;
    let mut last_emit = Instant::now();

    while let Some(chunk) = stream.next().await {
        if cancel.load(Ordering::Relaxed) {
            let _ = tokio::fs::remove_file(&part).await;
            return Err(AppError::Cancelled.into());
        }
        let chunk = chunk?;
        hasher.update(&chunk);
        downloaded += chunk.len() as u64;
        file.write_all(&chunk).await?;
        if last_elapsed(&last_emit) >= Duration::from_millis(250) {
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
        let _ = tokio::fs::remove_file(&part).await;
        return Err(anyhow!(
            "checksum mismatch for {} (expected {expected}, got {actual}) — download aborted",
            spec.key
        ));
    }

    tokio::fs::rename(&part, target).await?;
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

fn last_elapsed(since: &Instant) -> Duration {
    since.elapsed()
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
