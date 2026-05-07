use clap::{Parser, Subcommand};
use std::fs::{self, OpenOptions};
use std::io::{Read, Write};
use std::path::Path;
use uuid::Uuid;

#[derive(Parser)]
#[command(name = "chloe-pied")]
#[command(version)]
#[command(about = "Chloe-pied - Task management with Pi integration")]
pub struct Cli {
    #[command(subcommand)]
    pub command: Option<Commands>,
}

#[derive(Subcommand)]
pub enum Commands {
    /// Initialize Chloe-pied in the current directory
    Init,

    /// Handle lifecycle hook events (internal use)
    Notify {
        /// Event type: start, end, permission
        event_type: String,

        /// Worktree ID associated with this event
        #[arg(long)]
        worktree_id: Uuid,
    },
}

pub fn handle_notify_command(event_type: String, worktree_id: Uuid) -> Result<(), String> {
    let mut hook_data = String::new();
    std::io::stdin()
        .read_to_string(&mut hook_data)
        .map_err(|error| format!("Failed to read hook data from stdin: {error}"))?;

    let timestamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_err(|error| format!("System time error: {error}"))?
        .as_nanos();

    let hook_data_value = serde_json::from_str::<serde_json::Value>(&hook_data)
        .unwrap_or(serde_json::Value::String(hook_data));

    let event = crate::events::HookEvent {
        event: event_type,
        worktree_id,
        timestamp,
        hook_data: hook_data_value,
    };

    // Silently ignore errors - Chloe TUI may not be running
    let _ = crate::events::send_event(&event);

    Ok(())
}

pub fn handle_init_command() -> Result<(), String> {
    let chloe_directory = Path::new(".chloe-pied");
    let gitignore_path = Path::new(".gitignore");
    let gitignore_entry = ".chloe-pied/";

    fs::create_dir_all(chloe_directory)
        .map_err(|error| format!("Failed to create .chloe-pied directory: {error}"))?;

    println!("Created .chloe-pied/ directory");

    let should_add_to_gitignore = if gitignore_path.exists() {
        let contents = fs::read_to_string(gitignore_path)
            .map_err(|error| format!("Failed to read .gitignore: {error}"))?;

        !contents.lines().any(|line| line.trim() == gitignore_entry)
    } else {
        true
    };

    if should_add_to_gitignore {
        let mut file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(gitignore_path)
            .map_err(|error| format!("Failed to open .gitignore: {error}"))?;

        let needs_newline = if gitignore_path.exists() {
            let contents = fs::read_to_string(gitignore_path).unwrap_or_default();
            !contents.is_empty() && !contents.ends_with('\n')
        } else {
            false
        };

        let entry_to_write = if needs_newline {
            format!("\n{gitignore_entry}\n")
        } else {
            format!("{gitignore_entry}\n")
        };

        file.write_all(entry_to_write.as_bytes())
            .map_err(|error| format!("Failed to write to .gitignore: {error}"))?;

        println!("Added .chloe-pied/ to .gitignore");
    } else {
        println!(".chloe-pied/ already in .gitignore");
    }

    let current_directory = std::env::current_dir()
        .map_err(|error| format!("Failed to get current directory: {error}"))?;

    if !crate::views::worktree::operations::is_git_repo(&current_directory) {
        println!();
        println!("No git repository found in this directory.");
        println!("Would you like to initialize one? [Y/n]: ");

        let mut input = String::new();
        std::io::stdin()
            .read_line(&mut input)
            .map_err(|error| format!("Failed to read input: {error}"))?;

        let should_initialize = input.trim().is_empty() || input.trim().eq_ignore_ascii_case("y");

        if should_initialize {
            crate::views::worktree::operations::init_git_repo(&current_directory)
                .map_err(|error| format!("Failed to initialize git repository: {error}"))?;
            println!("✓ Initialized empty git repository.");

            if crate::views::worktree::operations::is_gh_available() {
                println!();
                println!("Create a private GitHub repository for this project? [y/N]: ");

                let mut github_input = String::new();
                std::io::stdin()
                    .read_line(&mut github_input)
                    .map_err(|error| format!("Failed to read input: {error}"))?;

                if github_input.trim().eq_ignore_ascii_case("y") {
                    let directory_name = current_directory
                        .file_name()
                        .and_then(|name| name.to_str())
                        .unwrap_or("my-project");

                    match crate::views::worktree::operations::create_github_repo(
                        &current_directory,
                        directory_name,
                    ) {
                        Ok(remote_url) => {
                            println!("✓ Created private GitHub repository: {remote_url}");
                        }
                        Err(error) => {
                            println!("⚠ Failed to create GitHub repository: {error}");
                            println!(
                                "  You can create one manually later with: gh repo create {directory_name} --private --source=. --remote=origin --push"
                            );
                        }
                    }
                }
            } else {
                println!();
                println!(
                    "  Tip: Install the GitHub CLI (gh) and run the following to create a remote repository:"
                );
                println!(
                    "    gh repo create <repo-name> --private --source=. --remote=origin --push"
                );
            }
        }
    }

    println!("Chloe-pied initialized successfully!");

    Ok(())
}
