use crate::app::App;
use crate::views::tasks::state::{Task, TaskType};
use clap::{Args, Parser, Subcommand};
use std::fs::{self, OpenOptions};
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
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

    /// Add a task to Planning without opening the TUI
    AddTask(AddTaskArgs),

    /// Handle lifecycle hook events (internal use)
    Notify {
        /// Event type: start, end, permission
        event_type: String,

        /// Worktree ID associated with this event
        #[arg(long)]
        worktree_id: Uuid,
    },
}

#[derive(Args)]
pub struct AddTaskArgs {
    /// Task title
    #[arg(long)]
    pub title: String,

    /// Task description
    #[arg(long)]
    pub description: String,

    /// Task type: feature, bug, chore, or task
    #[arg(long = "task-type", default_value = "task")]
    pub task_type: String,

    /// Transcript path to include as task source metadata
    #[arg(long = "transcript-path")]
    pub transcript_path: Option<PathBuf>,

    /// Source label to include as task metadata
    #[arg(long)]
    pub source: Option<String>,

    /// Show the task that would be created without writing state
    #[arg(long)]
    pub dry_run: bool,
}

pub fn handle_add_task_command(arguments: AddTaskArgs) -> Result<(), String> {
    let task_type = parse_task_type(&arguments.task_type)?;
    let description = build_task_description(
        &arguments.description,
        arguments.transcript_path.as_deref(),
        arguments.source.as_deref(),
    );
    let task = Task::new(arguments.title, description, task_type);

    if arguments.dry_run {
        print_task_preview("Dry run: task not saved", &task)?;
        return Ok(());
    }

    let mut app = App::load_or_default();
    app.tasks.columns[0].tasks.push(task.clone());
    app.tasks.kanban_selected_column = 0;
    app.tasks.kanban_selected_task = Some(app.tasks.columns[0].tasks.len() - 1);
    app.save()
        .map_err(|error| format!("Failed to save task state: {error}"))?;

    print_task_preview("Created task in Planning", &task)
}

fn parse_task_type(value: &str) -> Result<TaskType, String> {
    match value.trim().to_ascii_lowercase().as_str() {
        "feature" | "feat" => Ok(TaskType::Feature),
        "bug" | "fix" => Ok(TaskType::Bug),
        "chore" => Ok(TaskType::Chore),
        "task" => Ok(TaskType::Task),
        _ => Err(format!(
            "Invalid task type '{value}'. Expected feature, bug, chore, or task."
        )),
    }
}

fn build_task_description(
    description: &str,
    transcript_path: Option<&Path>,
    source: Option<&str>,
) -> String {
    let mut task_description = description.trim().to_string();
    let source = source.map(str::trim).filter(|value| !value.is_empty());

    if transcript_path.is_none() && source.is_none() {
        return task_description;
    }

    if !task_description.is_empty() {
        task_description.push_str("\n\n");
    }

    task_description.push_str("Source metadata:");

    if let Some(source) = source {
        task_description.push_str("\n- source: ");
        task_description.push_str(source);
    }

    if let Some(path) = transcript_path {
        task_description.push_str("\n- transcript: ");
        task_description.push_str(&path.display().to_string());
    }

    task_description
}

fn print_task_preview(message: &str, task: &Task) -> Result<(), String> {
    let json = serde_json::to_string_pretty(task)
        .map_err(|error| format!("Failed to serialize task preview: {error}"))?;

    println!("{message}:");
    println!("{json}");
    Ok(())
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
    let pi_extension_directory = Path::new(".pi/extensions");
    let pi_extension_path = pi_extension_directory.join("chloe-pied.ts");
    let gitignore_path = Path::new(".gitignore");
    let gitignore_entry = ".chloe-pied/";

    fs::create_dir_all(chloe_directory)
        .map_err(|error| format!("Failed to create .chloe-pied directory: {error}"))?;

    println!("Created .chloe-pied/ directory");

    fs::create_dir_all(pi_extension_directory)
        .map_err(|error| format!("Failed to create .pi/extensions directory: {error}"))?;
    fs::write(pi_extension_path, include_str!("assets/pi_extension.ts"))
        .map_err(|error| format!("Failed to write Chloe-pied Pi extension: {error}"))?;

    println!("Installed Pi extension at .pi/extensions/chloe-pied.ts");

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
