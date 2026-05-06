use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
pub enum AgentProvider {
    #[default]
    Pi,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DetectedProvider {
    pub provider: AgentProvider,
    pub path: PathBuf,
}

impl AgentProvider {
    #[must_use]
    pub const fn display_name(self) -> &'static str {
        match self {
            Self::Pi => "Pi",
        }
    }

    #[must_use]
    pub const fn command_name(self) -> &'static str {
        match self {
            Self::Pi => "pi",
        }
    }

    #[must_use]
    pub const fn all() -> &'static [Self] {
        &[Self::Pi]
    }

    #[must_use]
    pub fn detect(self) -> Option<DetectedProvider> {
        let output = std::process::Command::new("which")
            .arg(self.command_name())
            .output()
            .ok()?;

        if !output.status.success() {
            return None;
        }

        let path_string = String::from_utf8_lossy(&output.stdout);
        let path = PathBuf::from(path_string.trim());

        if path.exists() {
            Some(DetectedProvider {
                provider: self,
                path,
            })
        } else {
            None
        }
    }

    #[must_use]
    pub fn detect_all_available() -> Vec<DetectedProvider> {
        Self::all()
            .iter()
            .filter_map(|provider| provider.detect())
            .collect()
    }

    #[must_use]
    pub fn default_config(self) -> ProviderConfig {
        match self {
            Self::Pi => ProviderConfig {
                command: "pi".into(),
                arguments: vec![],
                oneshot_arguments: vec![],
                environment: HashMap::new(),
                working_directory_argument: None,
                supports_worktree: true,
            },
        }
    }
}

impl std::fmt::Display for AgentProvider {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(formatter, "{}", self.display_name())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProviderConfig {
    pub command: PathBuf,
    /// Extra arguments prepended to the interactive command (e.g., ["--no-orchestrator"])
    #[serde(default)]
    pub arguments: Vec<String>,
    /// Extra arguments prepended to oneshot commands (e.g., ["--no-orchestrator", "--no-skills"])
    #[serde(default)]
    pub oneshot_arguments: Vec<String>,
    pub environment: HashMap<String, String>,
    pub working_directory_argument: Option<String>,
    pub supports_worktree: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ProviderRegistry {
    pub configs: HashMap<AgentProvider, ProviderConfig>,
}

impl ProviderRegistry {
    #[must_use]
    pub fn new() -> Self {
        let mut configs = HashMap::new();
        for provider in AgentProvider::all() {
            configs.insert(*provider, provider.default_config());
        }
        Self { configs }
    }
}
