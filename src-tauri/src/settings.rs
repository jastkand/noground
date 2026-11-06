use std::{fs, path::Path};

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct Settings {
    pub model: String,
    pub output_dir: Option<String>,
    pub overwrite: bool,
    pub format: OutputFormat,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum OutputFormat {
    Png,
    Jpeg,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            model: "bria-rmbg".into(),
            output_dir: None,
            overwrite: false,
            format: OutputFormat::Png,
        }
    }
}

impl Settings {
    pub fn load(path: &Path) -> Self {
        match fs::read_to_string(path) {
            Ok(text) => match serde_json::from_str(&text) {
                Ok(settings) => settings,
                Err(e) => {
                    eprintln!("settings file unreadable, using defaults: {e}");
                    Self::default()
                }
            },
            Err(_) => Self::default(),
        }
    }

    pub fn save(&self, path: &Path) -> std::io::Result<()> {
        if let Some(dir) = path.parent() {
            fs::create_dir_all(dir)?;
        }
        let json = serde_json::to_string_pretty(self).expect("settings serialize");
        fs::write(path, json)
    }
}
