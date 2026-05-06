mod pi;

use crate::types::{AgentProvider, PermissionConfig, ProviderConfig};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use uuid::Uuid;

pub struct ProviderSpec {
    pub command: &'static str,
    pub prompt_style: PromptStyle,
    pub oneshot_style: OneShotPromptStyle,
    pub generate_files: fn(Uuid, &Path, &PermissionConfig) -> Vec<GeneratedFile>,
}

#[derive(Debug, Clone, Copy)]
pub enum PromptStyle {
    Direct,
    #[allow(dead_code)]
    Flag(&'static str),
}

#[derive(Debug, Clone, Copy)]
pub enum OneShotPromptStyle {
    #[allow(dead_code)]
    Direct,
    Flag(&'static str),
    #[allow(dead_code)]
    Subcommand(&'static str),
}

#[derive(Debug, Clone)]
pub struct GeneratedFile {
    pub path: PathBuf,
    pub content: String,
}

pub struct ProviderCommand {
    pub program: String,
    pub arguments: Vec<String>,
    pub environment: HashMap<String, String>,
}

impl ProviderSpec {
    #[must_use]
    pub fn build_command_with_config(
        &self,
        prompt: &str,
        config: Option<&ProviderConfig>,
    ) -> ProviderCommand {
        let mut arguments = Vec::new();

        // Prepend config interactive arguments
        if let Some(cfg) = config {
            arguments.extend(cfg.arguments.iter().cloned());
        }

        if !prompt.is_empty() {
            match self.prompt_style {
                PromptStyle::Direct => arguments.push(prompt.to_string()),
                PromptStyle::Flag(flag) => {
                    arguments.push(flag.to_string());
                    arguments.push(prompt.to_string());
                }
            }
        }

        let program = config
            .map(|cfg| cfg.command.to_string_lossy().to_string())
            .unwrap_or_else(|| self.command.to_string());

        let environment = config
            .map(|cfg| cfg.environment.clone())
            .unwrap_or_default();

        ProviderCommand {
            program,
            arguments,
            environment,
        }
    }

    #[must_use]
    pub fn build_oneshot_command_with_config(
        &self,
        prompt: &str,
        config: Option<&ProviderConfig>,
    ) -> ProviderCommand {
        let mut arguments = Vec::new();

        // Prepend config oneshot arguments
        if let Some(cfg) = config {
            arguments.extend(cfg.oneshot_arguments.iter().cloned());
        }

        match self.oneshot_style {
            OneShotPromptStyle::Direct => {
                arguments.push(prompt.to_string());
            }
            OneShotPromptStyle::Flag(flag) => {
                arguments.push(flag.to_string());
                arguments.push(prompt.to_string());
            }
            OneShotPromptStyle::Subcommand(subcommand) => {
                arguments.push(subcommand.to_string());
                arguments.push(prompt.to_string());
            }
        }

        let program = config
            .map(|cfg| cfg.command.to_string_lossy().to_string())
            .unwrap_or_else(|| self.command.to_string());

        let environment = config
            .map(|cfg| cfg.environment.clone())
            .unwrap_or_default();

        ProviderCommand {
            program,
            arguments,
            environment,
        }
    }

    #[must_use]
    pub fn build_files(
        &self,
        task_id: Uuid,
        working_directory: &Path,
        permission_config: &PermissionConfig,
    ) -> Vec<GeneratedFile> {
        (self.generate_files)(task_id, working_directory, permission_config)
    }
}

#[must_use]
pub fn get_spec(provider: AgentProvider) -> &'static ProviderSpec {
    match provider {
        AgentProvider::Pi => &pi::SPEC,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_build_command_with_config_direct_prompt() {
        let spec = get_spec(AgentProvider::Pi);
        let command = spec.build_command_with_config("Fix the bug", None);

        assert_eq!(command.program, "pi");
        assert_eq!(command.arguments, vec!["Fix the bug"]);
    }

    #[test]
    fn test_build_command_with_config_empty_prompt() {
        let spec = get_spec(AgentProvider::Pi);
        let command = spec.build_command_with_config("", None);

        assert_eq!(command.program, "pi");
        assert!(command.arguments.is_empty());
    }
}
