use std::sync::atomic::Ordering;

use tauri::{AppHandle, State};

use crate::{
    cli,
    error::{AppError, CmdError},
    models,
    pipeline,
    settings::Settings,
    state::AppState,
};

#[tauri::command]
pub fn get_settings(state: State<'_, AppState>) -> Settings {
    state.settings.lock().unwrap().clone()
}

#[tauri::command]
pub fn set_settings(state: State<'_, AppState>, settings: Settings) -> Result<Settings, CmdError> {
    let mut current = state.settings.lock().unwrap();
    *current = settings;
    let snapshot = current.clone();
    snapshot
        .save(&state.settings_path)
        .map_err(|e| CmdError::from(AppError::Io(e)))?;
    Ok(snapshot)
}

#[tauri::command]
pub fn inspect_paths(paths: Vec<String>) -> Vec<PathEntry> {
    paths
        .into_iter()
        .filter_map(|p| {
            let meta = std::fs::metadata(&p).ok()?;
            Some(PathEntry {
                path: p.clone(),
                name: std::path::Path::new(&p)
                    .file_name()
                    .map(|n| n.to_string_lossy().to_string())
                    .unwrap_or(p),
                is_dir: meta.is_dir(),
            })
        })
        .collect()
}

#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PathEntry {
    pub path: String,
    pub name: String,
    pub is_dir: bool,
}

#[tauri::command]
pub fn process_paths(
    app: AppHandle,
    paths: Vec<String>,
    overwrite: Option<bool>,
) -> Result<(), CmdError> {
    pipeline::start_job(&app, paths, overwrite).map_err(|e| CmdError {
        message: e.to_string(),
        cancelled: false,
    })
}

#[tauri::command]
pub fn cancel_job(state: State<'_, AppState>) -> bool {
    state.job_cancel.store(true, Ordering::SeqCst);
    true
}

#[tauri::command]
pub fn list_models() -> Vec<models::ModelInfo> {
    models::list_models()
}

#[tauri::command]
pub fn ensure_model(app: AppHandle, key: String) -> Result<models::ModelState, CmdError> {
    models::ensure_model(&app, &key).map_err(|e| CmdError {
        message: e.to_string(),
        cancelled: false,
    })
}

#[tauri::command]
pub fn cancel_model_download(state: State<'_, AppState>, key: String) -> bool {
    models::cancel_model_download(&state, &key)
}

#[tauri::command]
pub fn delete_model(state: State<'_, AppState>, key: String) -> Result<bool, CmdError> {
    models::delete_model(&state, &key).map_err(|e| CmdError {
        message: e.to_string(),
        cancelled: false,
    })?;
    Ok(true)
}

#[tauri::command]
pub fn cli_status() -> cli::CliStatus {
    cli::status()
}

#[tauri::command]
pub fn install_cli() -> Result<cli::CliStatus, CmdError> {
    cli::install().map_err(|e| CmdError {
        message: e.to_string(),
        cancelled: false,
    })
}

#[tauri::command]
pub fn uninstall_cli() -> Result<cli::CliStatus, CmdError> {
    cli::uninstall().map_err(|e| CmdError {
        message: e.to_string(),
        cancelled: false,
    })
}
