use std::{
    collections::HashMap,
    path::Path,
    sync::{Arc, Mutex},
};

use anyhow::{anyhow, Result};

use crate::models;

pub struct LoadedSession {
    pub session: Arc<Mutex<ort::session::Session>>,
    pub input_name: String,
}

impl LoadedSession {
    /// Run the model on a preprocessed [1, 3, s, s] tensor and return the
    /// first output plane (the raw mask). The one inference path shared by
    /// the app pipeline and the CLI. ort::Error is not Send + Sync —
    /// stringify inside the closure.
    pub fn infer(&self, input: Vec<f32>, size: u32) -> Result<Vec<f32>, String> {
        let s = size as usize;
        let plane_len = s * s;
        let mut session = self.session.lock().unwrap();
        (|| {
            let input_tensor = ort::value::Tensor::from_array(([1usize, 3, s, s], input))
                .map_err(|e| e.to_string())?;
            let outputs = session
                .run(ort::inputs![self.input_name.as_str() => input_tensor])
                .map_err(|e| e.to_string())?;
            let (_, data) = outputs[0]
                .try_extract_tensor::<f32>()
                .map_err(|e| e.to_string())?;
            Ok(data[..plane_len].to_vec())
        })()
    }
}

#[derive(Default)]
pub struct SessionCache {
    loaded: Mutex<HashMap<String, Arc<LoadedSession>>>,
}

impl SessionCache {
    /// `verify` is Some for files in the world-writable shared cache: their
    /// pinned checksum is checked right before the session is built (cache
    /// misses only, ~1 s per GB), since nothing on the filesystem pins that
    /// content. Files borrowed from ~/.rembg / ~/.u2net aren't managed here
    /// and may legitimately be older rembg-era downloads — load unverified.
    pub fn get_or_load(
        &self,
        key: &str,
        path: &Path,
        verify: Option<&models::ModelSpec>,
    ) -> Result<Arc<LoadedSession>> {
        let mut loaded = self.loaded.lock().unwrap();
        if let Some(existing) = loaded.get(key) {
            return Ok(existing.clone());
        }
        if let Some(spec) = verify {
            models::verify_file_checksum(path, spec)?;
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
