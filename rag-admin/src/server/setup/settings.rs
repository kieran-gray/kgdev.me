use std::path::{Path, PathBuf};

use tokio::fs;

use crate::server::setup::exceptions::SetupError;
use crate::shared::SettingsDto;

pub fn data_dir() -> PathBuf {
    std::env::current_dir()
        .map(|p| p.join("rag-admin").join("data"))
        .unwrap_or_else(|_| PathBuf::from("rag-admin/data"))
}

pub fn settings_path() -> PathBuf {
    data_dir().join("settings.toml")
}

pub fn manifest_path() -> PathBuf {
    data_dir().join("manifest.json")
}

pub fn post_chunking_config_path() -> PathBuf {
    data_dir().join("post-chunking.json")
}

pub fn tokenizer_path() -> PathBuf {
    data_dir().join("tokenizer.json")
}

pub fn evaluations_dir() -> PathBuf {
    data_dir().join("evaluations")
}

pub async fn load_settings(path: &Path) -> Result<SettingsDto, SetupError> {
    if path.exists() {
        let bytes = fs::read(path)
            .await
            .map_err(|e| SetupError::Io(format!("read settings: {e}")))?;
        if !bytes.is_empty() {
            let text = std::str::from_utf8(&bytes)
                .map_err(|e| SetupError::Config(format!("settings.toml not utf-8: {e}")))?;
            let settings: SettingsDto = toml::from_str(text)
                .map_err(|e| SetupError::Config(format!("parse settings.toml: {e}")))?;
            return Ok(settings);
        }
    }
    Ok(SettingsDto::default())
}

pub async fn save_settings(path: &Path, settings: &SettingsDto) -> Result<(), SetupError> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .await
            .map_err(|e| SetupError::Io(format!("create settings dir: {e}")))?;
    }
    let s = toml::to_string_pretty(settings)
        .map_err(|e| SetupError::Internal(format!("encode settings: {e}")))?;
    fs::write(path, s)
        .await
        .map_err(|e| SetupError::Io(format!("write settings: {e}")))
}
