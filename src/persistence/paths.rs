use std::env;
use std::path::PathBuf;

const CONFIG_DIRECTORY_NAME: &str = "chloe-pied";
const PROJECT_CONFIG_DIRECTORY_NAME: &str = ".chloe-pied";

#[must_use]
pub fn get_config_dir() -> PathBuf {
    env::current_dir()
        .unwrap_or_else(|_| PathBuf::from("."))
        .join(PROJECT_CONFIG_DIRECTORY_NAME)
}

#[must_use]
pub fn get_global_config_dir() -> PathBuf {
    dirs::config_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join(CONFIG_DIRECTORY_NAME)
}

#[must_use]
pub fn get_state_path() -> PathBuf {
    get_config_dir().join("state.json")
}

#[must_use]
pub fn get_activity_log_path() -> PathBuf {
    get_config_dir().join("activity.jsonl")
}

#[must_use]
pub fn get_settings_path() -> PathBuf {
    get_config_dir().join("settings.json")
}

#[must_use]
pub fn get_global_settings_path() -> PathBuf {
    get_global_config_dir().join("settings.json")
}
