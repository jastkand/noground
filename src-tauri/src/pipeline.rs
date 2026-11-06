use std::{
    fs,
    path::{Path, PathBuf},
    sync::atomic::Ordering,
};

use anyhow::{anyhow, Context, Result};
use serde::Serialize;
use tauri::{AppHandle, Emitter, Manager};

use crate::{
    imaging,
    models,
    settings::Settings,
    state::AppState,
};

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct JobProgress {
    stage: &'static str,
    index: usize,
    total: usize,
    file: String,
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct JobOutput {
    input: String,
    output: String,
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct JobFailure {
    input: String,
    message: String,
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct SkippedOutput {
    input: String,
    output: String,
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct JobDone {
    outputs: Vec<JobOutput>,
    skipped: Vec<SkippedOutput>,
    failed: Vec<JobFailure>,
    cancelled: bool,
    out_dir: Option<String>,
}

const IMAGE_EXTS: &[&str] = &["png", "jpg", "jpeg", "webp", "tif", "tiff", "bmp", "gif"];

pub fn start_job(app: &AppHandle, paths: Vec<String>, overwrite: Option<bool>) -> Result<()> {
    let state = app.state::<AppState>();
    if state.job_running.swap(true, Ordering::SeqCst) {
        return Err(anyhow!("a job is already running"));
    }
    state.job_cancel.store(false, Ordering::SeqCst);

    let app = app.clone();
    tauri::async_runtime::spawn_blocking(move || {
        let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            run_job_inner(app.clone(), paths, overwrite)
        }));
        app.state::<AppState>().job_running.store(false, Ordering::SeqCst);
        if result.is_err() {
            let _ = app.emit(
                "noground://job-failed",
                serde_json::json!({ "message": "internal error while processing" }),
            );
        }
    });
    Ok(())
}

fn run_job_inner(app: AppHandle, paths: Vec<String>, overwrite: Option<bool>) {
    if let Err(e) = run_job_fallible(&app, &paths, overwrite) {
        let _ = app.emit(
            "noground://job-failed",
            serde_json::json!({ "message": e.to_string() }),
        );
    }
}

fn run_job_fallible(app: &AppHandle, paths: &[String], overwrite: Option<bool>) -> Result<()> {
    let state = app.state::<AppState>();
    let settings: Settings = state.settings.lock().unwrap().clone();
    let overwrite = overwrite.unwrap_or(settings.overwrite);
    let cancel = state.job_cancel.clone();

    let files = expand_inputs(paths)?;
    if files.is_empty() {
        return Err(anyhow!("no images found in the dropped paths"));
    }

    // A single dropped folder gets an upscayl-style sibling output dir.
    let single_dir = paths.len() == 1 && Path::new(&paths[0]).is_dir();
    let batch_out_dir = if single_dir {
        let dir = Path::new(&paths[0]);
        let name = dir
            .file_name()
            .context("folder has no name")?
            .to_string_lossy();
        Some(
            dir.parent()
                .context("folder has no parent")?
                .join(format!("{name}-noground")),
        )
    } else {
        None
    };

    emit_progress(app, "model", 0, files.len(), String::new());
    let spec = models::spec(&settings.model)?;
    let Some(model_path) = models::resolve_model_path(app, spec.key) else {
        return Err(anyhow!(
            "model “{}” is not installed — download it in Settings first",
            spec.label
        ));
    };
    let loaded = state.sessions.get_or_load(spec.key, &model_path)?;

    let total = files.len();
    let mut outputs: Vec<JobOutput> = Vec::new();
    let mut skipped: Vec<SkippedOutput> = Vec::new();
    let mut failed: Vec<JobFailure> = Vec::new();

    for (index, input) in files.iter().enumerate() {
        if cancel.load(Ordering::SeqCst) {
            let _ = app.emit(
                "noground://job-done",
                JobDone {
                    outputs,
                    skipped,
                    failed,
                    cancelled: true,
                    out_dir: batch_out_dir
                        .as_ref()
                        .map(|d| d.display().to_string()),
                },
            );
            return Ok(());
        }

        let file_name = input.display().to_string();

        // decide the output path before doing any work — an existing result
        // skips instantly instead of after a full inference
        let out_path = match compute_output_path(input, &batch_out_dir, &settings) {
            Ok(p) => p,
            Err(e) => {
                failed.push(JobFailure {
                    input: file_name,
                    message: e.to_string(),
                });
                continue;
            }
        };
        if !overwrite && out_path.exists() {
            skipped.push(SkippedOutput {
                input: file_name,
                output: out_path.display().to_string(),
            });
            continue;
        }

        emit_progress(app, "decode", index, total, file_name.clone());

        let img = match imaging::load_oriented(input) {
            Ok(img) => img,
            Err(e) => {
                failed.push(JobFailure {
                    input: file_name,
                    message: e.to_string(),
                });
                continue;
            }
        };

        emit_progress(app, "infer", index, total, file_name.clone());
        let tensor = imaging::preprocess(&img, spec);
        let s = spec.input_size as usize;
        let plane_len = s * s;
        // ort::Error is not Send + Sync — stringify inside the closure.
        let mask_data: Result<Vec<f32>, String> = {
            let mut session = loaded.session.lock().unwrap();
            (|| {
                let input_tensor = ort::value::Tensor::from_array(([1usize, 3, s, s], tensor))
                    .map_err(|e| e.to_string())?;
                let outputs = session
                    .run(ort::inputs![loaded.input_name.as_str() => input_tensor])
                    .map_err(|e| e.to_string())?;
                let (_, data) = outputs[0]
                    .try_extract_tensor::<f32>()
                    .map_err(|e| e.to_string())?;
                Ok(data[..plane_len].to_vec())
            })()
        };
        let mask_data = match mask_data {
            Ok(data) => data,
            Err(message) => {
                failed.push(JobFailure {
                    input: file_name,
                    message,
                });
                continue;
            }
        };

        emit_progress(app, "encode", index, total, file_name.clone());
        let result = (|| -> Result<PathBuf> {
            let mask = imaging::postprocess(&mask_data, plane_len, img.width(), img.height(), spec)?;
            let composed = imaging::compose(&img, &mask);
            imaging::encode_and_write(&composed, &out_path, settings.format)?;
            Ok(out_path.clone())
        })();
        match result {
            Ok(written) => outputs.push(JobOutput {
                input: file_name.clone(),
                output: written.display().to_string(),
            }),
            Err(message) => failed.push(JobFailure {
                input: file_name,
                message: message.to_string(),
            }),
        }
    }

    let _ = app.emit(
        "noground://job-done",
        JobDone {
            outputs,
            skipped,
            failed,
            cancelled: false,
            out_dir: batch_out_dir
                .as_ref()
                .map(|d| d.display().to_string())
                .or_else(|| settings.output_dir.clone()),
        },
    );
    Ok(())
}

fn emit_progress(app: &AppHandle, stage: &'static str, index: usize, total: usize, file: String) {
    let _ = app.emit(
        "noground://job-progress",
        JobProgress {
            stage,
            index,
            total,
            file,
        },
    );
}

fn compute_output_path(
    input: &Path,
    batch_out_dir: &Option<PathBuf>,
    settings: &Settings,
) -> Result<PathBuf> {
    let stem = input
        .file_stem()
        .context("file has no stem")?
        .to_string_lossy();
    let ext = match settings.format {
        crate::settings::OutputFormat::Png => "png",
        crate::settings::OutputFormat::Jpeg => "jpg",
    };
    let dir = if let Some(d) = &settings.output_dir {
        PathBuf::from(d)
    } else if let Some(d) = batch_out_dir {
        d.clone()
    } else {
        input.parent().context("file has no parent")?.to_path_buf()
    };
    Ok(dir.join(format!("{stem}-noground.{ext}")))
}

fn expand_inputs(paths: &[String]) -> Result<Vec<PathBuf>> {
    let mut files: Vec<PathBuf> = Vec::new();
    for p in paths {
        let path = PathBuf::from(p);
        if path.is_dir() {
            let mut dir_files: Vec<PathBuf> = fs::read_dir(&path)?
                .filter_map(|entry| entry.ok().map(|e| e.path()))
                .filter(|f| f.is_file() && is_image_path(f))
                .collect();
            files.append(&mut dir_files);
        } else if path.is_file() {
            files.push(path);
        }
    }
    files.sort_by(|a, b| a.file_name().cmp(&b.file_name()));
    files.dedup();
    Ok(files)
}

fn is_image_path(p: &Path) -> bool {
    p.extension()
        .and_then(|e| e.to_str())
        .map(|e| e.to_lowercase())
        .is_some_and(|e| IMAGE_EXTS.contains(&e.as_str()))
}
