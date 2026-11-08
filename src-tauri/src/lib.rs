mod commands;
mod error;
mod imaging;
mod models;
#[cfg(test)]
mod parity_tests;
mod pipeline;
mod session;
mod settings;
mod state;

use std::{
    collections::HashMap,
    sync::{
        atomic::AtomicBool,
        Arc, Mutex,
    },
};

use tauri::Manager;

use crate::{settings::Settings, state::AppState};

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_opener::init())
        .setup(|app| {
            let config_dir = app.path().app_config_dir()?;
            let settings_path = config_dir.join("settings.json");
            let settings = Settings::load(&settings_path);
            app.manage(AppState {
                settings: Mutex::new(settings),
                settings_path,
                sessions: Default::default(),
                downloads: Mutex::new(HashMap::new()),
                job_running: AtomicBool::new(false),
                job_cancel: Arc::new(AtomicBool::new(false)),
            });
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::get_settings,
            commands::set_settings,
            commands::inspect_paths,
            commands::process_paths,
            commands::cancel_job,
            commands::list_models,
            commands::ensure_model,
            commands::cancel_model_download,
            commands::delete_model,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
