mod pi;

use crate::types::{AgentProvider, PermissionConfig};
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
    pub fn build_command(&self, prompt: &str) -> ProviderCommand {
        let mut arguments = Vec::new();

        if !prompt.is_empty() {
            match self.prompt_style {
                PromptStyle::Direct => arguments.push(prompt.to_string()),
                PromptStyle::Flag(flag) => {
                    arguments.push(flag.to_string());
                    arguments.push(prompt.to_string());
                }
            }
        }

        ProviderCommand {
            program: self.command.to_string(),
            arguments,
            environment: HashMap::new(),
        }
    }

    #[must_use]
    pub fn build_oneshot_command(&self, prompt: &str) -> ProviderCommand {
        let mut arguments = Vec::new();

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

        ProviderCommand {
            program: self.command.to_string(),
            arguments,
            environment: HashMap::new(),
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
    fn test_build_command_direct_prompt() {
        let spec = get_spec(AgentProvider::Pi);
        let command = spec.build_command("Fix the bug");

        assert_eq!(command.program, "pi");
        assert_eq!(command.arguments, vec!["Fix the bug"]);
    }

    #[test]
    fn test_build_command_empty_prompt() {
        let spec = get_spec(AgentProvider::Pi);
        let command = spec.build_command("");

        assert_eq!(command.program, "pi");
        assert!(command.arguments.is_empty());
    }
}