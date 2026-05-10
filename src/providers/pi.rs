use super::{GeneratedFile, OneShotPromptStyle, PromptStyle, ProviderSpec};
use crate::types::PermissionConfig;
use std::path::Path;
use uuid::Uuid;

pub static SPEC: ProviderSpec = ProviderSpec {
    command: "pi",
    prompt_style: PromptStyle::Direct,
    oneshot_style: OneShotPromptStyle::Flag("-p"),
    generate_files,
};

const fn generate_files(
    _task_id: Uuid,
    _working_directory: &Path,
    _permission_config: &PermissionConfig,
) -> Vec<GeneratedFile> {
    Vec::new()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::ProviderConfig;

    #[test]
    fn test_spec_values() {
        assert_eq!(SPEC.command, "pi");
    }

    #[test]
    fn test_build_command_with_prompt() {
        let command = SPEC.build_command_with_config("Fix the bug", None);
        assert_eq!(command.program, "pi");
        assert_eq!(command.arguments, vec!["Fix the bug"]);
    }

    #[test]
    fn test_build_command_empty_prompt() {
        let command = SPEC.build_command_with_config("", None);
        assert_eq!(command.program, "pi");
        assert!(command.arguments.is_empty());
    }

    #[test]
    fn test_build_command_with_config_overrides() {
        let config = ProviderConfig {
            command: "pi".into(),
            arguments: vec!["--no-orchestrator".to_string()],
            oneshot_arguments: vec![],
            environment: std::collections::HashMap::new(),
            working_directory_argument: None,
            supports_worktree: true,
            rpc_mode: false,
        };
        let command = SPEC.build_command_with_config("Fix the bug", Some(&config));
        assert_eq!(command.program, "pi");
        assert_eq!(command.arguments, vec!["--no-orchestrator", "Fix the bug"]);
    }

    #[test]
    fn test_build_oneshot_command() {
        let command = SPEC.build_oneshot_command_with_config("Fix the bug", None);
        assert_eq!(command.program, "pi");
        assert_eq!(command.arguments, vec!["-p", "Fix the bug"]);
    }

    #[test]
    fn test_build_oneshot_command_with_config_overrides() {
        let config = ProviderConfig {
            command: "pi".into(),
            arguments: vec![],
            oneshot_arguments: vec!["--no-orchestrator".to_string(), "--no-skills".to_string()],
            environment: std::collections::HashMap::new(),
            working_directory_argument: None,
            supports_worktree: true,
            rpc_mode: false,
        };
        let command = SPEC.build_oneshot_command_with_config("Fix the bug", Some(&config));
        assert_eq!(command.program, "pi");
        assert_eq!(
            command.arguments,
            vec!["--no-orchestrator", "--no-skills", "-p", "Fix the bug"]
        );
    }

    #[test]
    fn test_generate_files_returns_empty() {
        let task_id = uuid::Uuid::new_v4();
        let working_dir = Path::new("/tmp/test");
        let permission_config = PermissionConfig::default();
        let files = generate_files(task_id, working_dir, &permission_config);
        assert!(files.is_empty());
    }
}
