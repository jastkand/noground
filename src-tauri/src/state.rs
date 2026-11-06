use std::{
    collections::HashMap,
    path::PathBuf,
    sync::{
        atomic::AtomicBool,
        Arc, Mutex,
    },
};

use crate::{session::SessionCache, settings::Settings};

pub struct AppState {
    pub settings: Mutex<Settings>,
    pub settings_path: PathBuf,
    pub sessions: SessionCache,
    pub downloads: Mutex<HashMap<String, Arc<AtomicBool>>>,
    pub job_running: AtomicBool,
    pub job_cancel: Arc<AtomicBool>,
}
