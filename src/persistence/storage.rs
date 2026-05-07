use crate::app::App;
use crate::types::{AgentProvider, PermissionConfig, ProviderRegistry, Result, ReviewMode};
use crate::views::settings::state::{IdeCommand, Settings, TerminalCommand, VcsCommand};
use serde::Deserialize;
use std::collections::HashMap;
use std::fs;

#[derive(Default, Deserialize)]
struct SettingsOverrides {
    default_shell: Option<String>,
    auto_save_interval_seconds: Option<u64>,
    ide_command: Option<IdeCommand>,
    terminal_command: Option<TerminalCommand>,
    vcs_command: Option<VcsCommand>,
    default_provider: Option<AgentProvider>,
    skip_provider_selection: Option<bool>,
    provider_registry: Option<ProviderRegistry>,
    permission_configs: Option<HashMap<AgentProvider, PermissionConfig>>,
    review_mode: Option<ReviewMode>,
    task_prompt_template: Option<String>,
}

impl SettingsOverrides {
    fn apply_to(self, settings: &mut Settings) {
        if let Some(default_shell) = self.default_shell {
            settings.default_shell = default_shell;
        }
        if let Some(auto_save_interval_seconds) = self.auto_save_interval_seconds {
            settings.auto_save_interval_seconds = auto_save_interval_seconds;
        }
        if let Some(ide_command) = self.ide_command {
            settings.ide_command = ide_command;
        }
        if let Some(terminal_command) = self.terminal_command {
            settings.terminal_command = terminal_command;
        }
        if let Some(vcs_command) = self.vcs_command {
            settings.vcs_command = vcs_command;
        }
        if let Some(default_provider) = self.default_provider {
            settings.default_provider = default_provider;
        }
        if let Some(skip_provider_selection) = self.skip_provider_selection {
            settings.skip_provider_selection = skip_provider_selection;
        }
        if let Some(provider_registry) = self.provider_registry {
            settings.provider_registry = provider_registry;
        }
        if let Some(permission_configs) = self.permission_configs {
            settings.permission_configs = permission_configs;
        }
        if let Some(review_mode) = self.review_mode {
            settings.review_mode = review_mode;
        }
        if let Some(task_prompt_template) = self.task_prompt_template {
            settings.task_prompt_template = Some(task_prompt_template);
        }
    }
}

pub fn save_state(app: &App) -> Result<()> {
    let path = super::paths::get_state_path();

    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }

    let json = serde_json::to_string_pretty(app)?;
    fs::write(path, json)?;

    Ok(())
}

pub fn load_state() -> Result<App> {
    let path = super::paths::get_state_path();

    if !path.exists() {
        return Ok(App::default());
    }

    let json = fs::read_to_string(path)?;
    let app: App = serde_json::from_str(&json)?;

    Ok(app)
}

pub fn save_settings(settings: &Settings) -> Result<()> {
    let path = super::paths::get_settings_path();

    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }

    let json = serde_json::to_string_pretty(settings)?;
    fs::write(path, json)?;

    Ok(())
}

#[allow(dead_code)]
pub fn save_global_settings(settings: &Settings) -> Result<()> {
    let path = super::paths::get_global_settings_path();

    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }

    let json = serde_json::to_string_pretty(settings)?;
    fs::write(path, json)?;

    Ok(())
}

pub fn load_settings() -> Result<Settings> {
    let mut settings = load_global_settings()?;
    let path = super::paths::get_settings_path();

    if !path.exists() {
        return Ok(settings);
    }

    let json = fs::read_to_string(path)?;
    let overrides: SettingsOverrides = serde_json::from_str(&json)?;
    overrides.apply_to(&mut settings);

    Ok(settings)
}

pub fn load_global_settings() -> Result<Settings> {
    let path = super::paths::get_global_settings_path();

    if !path.exists() {
        return Ok(Settings::default());
    }

    let json = fs::read_to_string(path)?;
    let overrides: SettingsOverrides = serde_json::from_str(&json)?;
    let mut settings = Settings::default();
    overrides.apply_to(&mut settings);

    Ok(settings)
}
