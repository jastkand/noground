use std::{
    collections::HashMap,
    path::Path,
    sync::{Arc, Mutex},
};

use anyhow::{anyhow, Result};

pub struct LoadedSession {
    pub session: Arc<Mutex<ort::session::Session>>,
    pub input_name: String,
}

#[derive(Default)]
pub struct SessionCache {
    loaded: Mutex<HashMap<String, Arc<LoadedSession>>>,
}

impl SessionCache {
    pub fn get_or_load(&self, key: &str, path: &Path) -> Result<Arc<LoadedSession>> {
        let mut loaded = self.loaded.lock().unwrap();
        if let Some(existing) = loaded.get(key) {
            return Ok(existing.clone());
        }

        // ort::Error is not Send + Sync — stringify at every fallible step.
        let threads = std::thread::available_parallelism()
            .map(|n| n.get())
            .unwrap_or(4);
        let session = ort::session::Session::builder()
            .map_err(|e| anyhow!("{e}"))?
            .with_optimization_level(ort::session::builder::GraphOptimizationLevel::Level3)
            .map_err(|e| anyhow!("{e}"))?
            .with_intra_threads(threads)
            .map_err(|e| anyhow!("{e}"))?
            .commit_from_file(path)
            .map_err(|e| anyhow!("{e}"))?;
        let input_name = session
            .inputs()
            .first()
            .map(|i| i.name().to_string())
            .ok_or_else(|| anyhow!("model {key} has no inputs"))?;

        let entry = Arc::new(LoadedSession {
            session: Arc::new(Mutex::new(session)),
            input_name,
        });
        // keep only the requested model resident — sessions are ~1 GB each and
        // a job uses exactly one
        loaded.retain(|k, _| k == key);
        loaded.insert(key.to_string(), entry.clone());
        Ok(entry)
    }

    /// drop a cached session, e.g. its backing model file was removed; any job
    /// holding the Arc keeps its copy alive
    pub fn evict(&self, key: &str) {
        self.loaded.lock().unwrap().remove(key);
    }
}
