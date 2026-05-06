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

fn generate_files(
    _task_id: Uuid,
    _working_directory: &Path,
    _permission_config: &PermissionConfig,
) -> Vec<GeneratedFile> {
    // Pi doesn't use a settings file like Claude Code's .claude/settings.local.json.
    // Configuration is managed through pi's own extension and settings system.
    vec![]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_spec_values() {
        assert_eq!(SPEC.command, "pi");
    }

    #[test]
    fn test_build_command_with_prompt() {
        let command = SPEC.build_command("Fix the bug");
        assert_eq!(command.program, "pi");
        assert_eq!(command.arguments, vec!["Fix the bug"]);
    }

    #[test]
    fn test_build_command_empty_prompt() {
        let command = SPEC.build_command("");
        assert_eq!(command.program, "pi");
        assert!(command.arguments.is_empty());
    }

    #[test]
    fn test_build_oneshot_command() {
        let command = SPEC.build_oneshot_command("Fix the bug");
        assert_eq!(command.program, "pi");
        assert_eq!(command.arguments, vec!["-p", "Fix the bug"]);
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