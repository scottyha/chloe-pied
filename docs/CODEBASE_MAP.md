# Chloe Codebase Map

> Last mapped: 2026-05-07T11:16:16Z
> Total files: 121 Rust source files
> Total lines: ~16,639 lines
> Chunks: 1 (monolithic analysis)

---

## System Overview

Chloe is a terminal-based task management TUI (Ratatui + Crossterm) that manages a Kanban/Focus board of tasks, each of which can be sent to an AI agent (`pi`) for execution inside a PTY pane. It integrates with git worktrees for isolated development branches per task.

```
User Input → crossterm events → app.rs dispatch → state mutation → Ratatui render
```

**High-level data flow for a task:**
1. User types a brief task description → AI classifies it (title, description, type) → stored in Planning column
2. User moves task to In Progress → git worktree created → `pi` agent spawned in PTY pane with task prompt
3. Agent completes → auto-moved to Review column → manual or agentic review
4. Review passed → merge worktree → move to Done

---

## Annotated Directory Structure

```
src/
├── main.rs                     # Entry point: event loop, terminal init, PTY poll
├── lib.rs                      # Module declarations
├── cli.rs                      # CLI args (clap): --json, --load, hooks, notify subcommands
├── app.rs                      # App struct: central state coordinator (tasks, instances, settings, tabs)
│                                # Key methods: sync_task_instances(), spawn_review_agent(), commit/merge
│                                # Contains build_review_prompt() — review prompt template
├── events/
│   ├── mod.rs                  # Event types: PtyOutput, PtyExit, ClassificationCompleted, HookReceived
│   ├── app.rs                  # AppEvent enum definition
│   ├── dispatch.rs             # Routes keyboard events to active tab
│   ├── event_loop.rs           # Async event loop: PTY polling, hook listening, classification results
│   └── hook.rs                 # Git hook integration (session_start/end/permission_request events)
├── types/
│   ├── mod.rs                  # Re-exports
│   ├── provider.rs             # AgentProvider (Pi), ProviderConfig, ProviderRegistry, DetectedProvider
│   ├── permissions.rs          # PermissionConfig, PermissionPreset (Restrictive/Balanced/Permissive)
│   ├── review_mode.rs          # ReviewMode (Human/Agentic)
│   └── errors.rs               # AppError, Result
├── providers/
│   ├── mod.rs                  # ProviderSpec trait: build_command(), build_oneshot_command()
│   │                            # PromptStyle (Direct/Flag), OneShotPromptStyle
│   └── pi.rs                   # Pi provider spec: command="pi", PromptStyle::Direct, OneShotPromptStyle::Flag("-p")
├── persistence/
│   ├── mod.rs                  # Module exports
│   ├── paths.rs                # get_config_dir(), get_state_path(), get_settings_path()
│   ├── storage.rs              # save_state(), load_state(), save_settings(), load_settings()
│                                # state.json → full App serialization (tasks, instances, roadmap)
│                                # settings.json → Settings struct only
├── views/
│   ├── mod.rs                  # Tab-bar rendering
│   ├── tab_bar.rs              # Tab bar widget
│   ├── layout.rs               # Main layout: panes, tabs, status bar
│   ├── footer.rs               # Keybinding hints footer
│   │
│   ├── tasks/                  # ★ PRIMARY FOCUS: Task management
│   │   ├── mod.rs              # Module exports
│   │   ├── state.rs            # TasksState, Task, Column, TaskType, TasksMode (big enum)
│   │   │                        # Task: id, title, description, kind, provider, instance_id, worktree_info
│   │   ├── ai_classifier.rs    # ★ Classification: spawns thread to call `pi -p` with JSON prompt
│   │   ├── dispatch.rs         # Routes TasksAction → App methods (create, delete, open_in_ide, etc.)
│   │   ├── events/
│   │   │   ├── mod.rs          # TasksAction enum + handle_key_event() dispatcher
│   │   │   ├── text_input.rs   # AddingTask, EditingTask mode key handlers
│   │   │   ├── kanban_navigation.rs  # 'a'=add, 'e'=edit, 'd'=delete, Enter=move next, etc.
│   │   │   ├── focus_navigation.rs   # Focus view key handlers (same actions)
│   │   │   ├── terminal.rs     # TerminalFocused/Scroll mode key handlers
│   │   │   ├── provider_selection.rs # Provider selection dialog
│   │   │   └── worktree_selection.rs # Worktree selection dialog
│   │   ├── operations/
│   │   │   ├── mod.rs          # Query helpers: get_active_tasks(), get_done_tasks()
│   │   │   ├── crud.rs         # add_task_to_planning(), delete_task_by_id(), update_task_title()
│   │   │   ├── movement.rs     # move_task_next(), move_task_previous(), move_to_in_progress, etc.
│   │   │   ├── classification.rs # start_classification(), handle_classification_completed()
│   │   │   ├── navigation.rs   # Focus view: select_next/prev, clamp_selection
│   │   │   ├── queries.rs      # Task lookup helpers (find_task_by_id, find_task_index_by_id)
│   │   │   └── worktree.rs     # ★ begin_add_task(), select_prompt(), create_worktree_for_new_task()
│   │   ├── dialogs/
│   │   │   ├── mod.rs          # Shared dialog helpers (centered_rect, popup background)
│   │   │   ├── add_task.rs     # ★ AddTaskDialog render: prompt + input area + tip block
│   │   │   ├── exit_confirmation.rs
│   │   │   ├── merge_confirmation.rs
│   │   │   ├── provider_selection.rs
│   │   │   ├── worktree_selection.rs
│   │   │   └── review/         # Review popup: file list, diff, output
│   │   └── views/
│   │       ├── mod.rs          # View dispatcher (Kanban vs Focus)
│   │       ├── kanban/         # Kanban board: columns, tasks, view
│   │       └── focus/          # Focus view: task_list, done_tasks, details_panel, terminal_panel
│   │
│   ├── instances/              # PTY instance management
│   │   ├── mod.rs              # Module exports
│   │   ├── state.rs            # InstanceState, InstancePane, PaneNode (tree), AgentState
│   │   ├── operations.rs       # ★ create_pane_for_task(): builds task prompt, spawns PTY with agent
│   │   │                        # Contains build_task_prompt() — ★ THIS IS THE KEY PROMPT BUILDER
│   │   ├── pty.rs              # PTY session (alacritty_terminal), spawn, read, write
│   │   ├── layout.rs           # Pane layout algorithms
│   │   ├── activity.rs         # Activity event detection
│   │   ├── view.rs             # Instance rendering
│   │   ├── action.rs           # Instance action render
│   │   └── events.rs           # Instance keyboard events
│   │
│   ├── settings/               # Settings UI
│   │   ├── mod.rs              # Module exports
│   │   ├── state.rs            # ★ Settings struct, SettingsState, SettingItem enum, SettingsMode
│   │   ├── events.rs           # Settings keyboard events
│   │   ├── action.rs           # Settings action render (dropdowns, toggles)
│   │   └── view.rs             # Settings view layout
│   │
│   ├── worktree/               # Git worktree management
│   ├── roadmap/                # Roadmap view (priority-prioritized items)
│   └── pull_requests/          # PR view
│
└── widgets/                    # Reusable UI widgets
    ├── mod.rs
    ├── spinner.rs              # Loading spinner
    ├── activity_summary.rs
    ├── agent_indicator.rs      # Agent state (Running/Done/NeedsPermissions)
    ├── dialogs/                # Confirm, Error, Input dialogs
    └── terminal/               # Terminal widget (alacritty_terminal rendering)
```

---

## Module Guide

### 1. Task Creation Flow (Add Task)

**Entry points:**
- Kanban view: `key 'a'` → `kanban_navigation.rs` → `state.begin_add_task()`
- Focus view: `key 'a'` → `focus_navigation.rs` → `state.begin_add_task()`

**`begin_add_task()`** (in `operations/worktree.rs`):
1. Calls `select_prompt()` — randomly selects one of 20 greeting prompts (e.g. "What should we build today?")
2. Sets `state.mode = TasksMode::AddingTask { input, prompt }`

**UI rendering** (in `dialogs/add_task.rs`):
- Shows the prompt string at top (bold white)
- Shows user input with cursor below
- Shows a "How It Works" tip block explaining the AI expansion

**User types + Enter** (in `events/text_input.rs`):
- Characters accumulate in `input` field
- Enter → fires `TasksAction::CreateTask { title: input }`
- Esc → cancels, returns to Normal mode

**Dispatch** (in `dispatch.rs` → `TasksAction::CreateTask`):
- Calls `state.start_classification(title, provider, config, event_sender)`

**Classification** (in `operations/classification.rs` + `ai_classifier.rs`):
1. Creates a `Task::new_classifying(raw_input)` — placeholder with `is_classifying: true`
2. Inserts into Planning column (index 0)
3. Spawns a thread calling `classify_with_provider()`
4. `classify_with_provider()` builds a **classification prompt** hardcoded in `ai_classifier.rs`:
   ```
   Classify this task description and respond with ONLY valid JSON...
   User input: "{raw_input}"
   Output format: { title, description, task_type }
   ```
5. Calls `pi -p "<prompt>"` (oneshot mode via `OneShotPromptStyle::Flag("-p")`)
6. Parses JSON response → sends `AppEvent::ClassificationCompleted { task_id, result }`
7. On completion → `handle_classification_completed()` updates task with title, description, type

**Classification prompt is NOT configurable** — it's hardcoded in `ai_classifier.rs` at line ~38.

---

### 2. Task Prompt Construction (for Agent Execution)

**Where the prompt is built:** `views/instances/operations.rs` — `build_task_prompt()` (line ~133)

```rust
fn build_task_prompt(
    title: &str,
    description: &str,
    vcs_command: &VcsCommand,
    omit_no_commit_instruction: bool,
) -> String {
    let base_prompt = if description.is_empty() {
        title.to_string()
    } else {
        format!("Work on this task:\n\nTitle: {title}\n\nDescription: {description}")
    };

    if omit_no_commit_instruction {
        return base_prompt;
    }

    let vcs_command_name = vcs_command.command_name();
    format!(
        "{base_prompt}\n\nIMPORTANT: Do not commit these changes with '{vcs_command_name} commit' until I explicitly ask you to."
    )
}
```

**This is the key prompt for your change.** Currently **hardcoded** — no template, no settings integration.

**Call chain:** `sync_task_instances()` (in `app.rs`) → `create_pane_for_task()` (in `operations.rs`):
1. Builds prompt via `build_task_prompt(title, description, vcs_command, ...)`
2. Builds command via `spec.build_command_with_config(&prompt, config)` → results in `pi "<prompt>"`
3. Wraps in shell command: `pi "<prompt>"; chloe-pied notify end --worktree-id <id> 2>/dev/null; exec $SHELL`
4. Spawns PTY with this command running in the worktree directory

**There is also a review prompt** in `app.rs` → `build_review_prompt()`:
```
You are a code reviewer for task: "{title}"
Description: {description}
1. Run `git diff`...
2. Review the diff...
3. If good: stage, commit, type REVIEW_COMPLETE
4. If not: describe fixes, type REVIEW_REQUEST_CHANGES
```

---

### 3. settings.json Structure

**Defined by** `Settings` struct in `views/settings/state.rs` (line ~82):

```rust
pub struct Settings {
    pub default_shell: String,
    pub auto_save_interval_seconds: u64,
    pub ide_command: IdeCommand,              // Cursor | VSCode | WebStorm | Custom(String)
    pub terminal_command: TerminalCommand,    // AppleTerminal | ITerm2 | Custom(String)
    pub vcs_command: VcsCommand,              // Git | Jujutsu
    pub default_provider: AgentProvider,      // Pi
    pub skip_provider_selection: bool,
    pub provider_registry: ProviderRegistry,  // HashMap<AgentProvider, ProviderConfig>
    pub permission_configs: HashMap<AgentProvider, PermissionConfig>,
    pub review_mode: ReviewMode,              // Human | Agentic
}
```

**Stored at:** `.chloe-pied/settings.json`
**Loaded via:** `persistence/storage::load_settings()` → called in `app.rs::load_or_default()`
**Saved via:** `persistence/storage::save_settings()` → called from `dispatch.rs` when settings change

**Current settings.json example** (from `.chloe-pied/settings.json`):
```json
{
  "default_shell": "/bin/bash",
  "auto_save_interval_seconds": 30,
  "ide_command": "VSCode",
  "terminal_command": "ITerm2",
  "vcs_command": "Git",
  "default_provider": "Pi",
  "skip_provider_selection": false,
  "provider_registry": { "configs": { "Pi": { ... } } },
  "permission_configs": { "Pi": { "allowed_tools": [...], "sandbox": {...} } }
}
```

**Note:** There's also a **`state.json`** (the full App state including tasks, instances, roadmap) — separate from settings.json. Settings is the user preferences subset.

---

### 4. How the Settings UI Maps to Fields

| Setting Section | Setting Item | Field in Settings | Type | Editable via |
|---|---|---|---|---|
| Shell & Terminal | Default Shell | `default_shell` | String | Text input |
| Shell & Terminal | Terminal | `terminal_command` | enum | Selection |
| Shell & Terminal | VCS Command | `vcs_command` | enum | Selection |
| Editor & IDE | IDE Command | `ide_command` | enum | Selection |
| Agent | Default Agent | `default_provider` | AgentProvider | Selection |
| Agent | Agent Permissions | `permission_configs[provider]` | PermissionConfig | Preset selection |
| Persistence | Auto-save Interval | `auto_save_interval_seconds` | u64 | Numeric input |
| Review | Review Mode | `review_mode` | ReviewMode | Selection |

---

## Data Flow: Full Task Lifecycle

```
User presses 'a'
  → begin_add_task() [worktree.rs]
    → random prompt selected from 20 hardcoded strings
  → UI shows dialog [add_task.rs]
  → User types + Enter
    → TasksAction::CreateTask { title }
  → start_classification() [classification.rs]
    → spawn_classification() [ai_classifier.rs]
      → pi -p "Classify this task... User input: ..."
      → Parse JSON → ClassificationCompleted event
    → Task now has: title, description, task_type
    → Task sits in Planning column

User presses Enter on task
  → move_task_next() [movement.rs]
    → begin_worktree_selection_for_task() [worktree.rs]
      → load_worktree_selection_options() (detect existing worktrees)
      → User picks: AutoCreate, InitLocalRepo, CreateOnGitHub, or Existing
  → move_task_to_in_progress_with_worktree() [movement.rs]
    → Creates git worktree
    → Sets pending_instance_creation

sync_task_instances() [app.rs]
  → create_pane_for_task() [instances/operations.rs]
    → build_task_prompt(title, description, vcs_command, ...)
    → spec.build_command_with_config(&prompt) → "pi \"Work on this task:...\""
    → Builds shell wrapper: pi "..."; notify end; exec $SHELL
    → Spawns PtySession with command in worktree directory

Agent runs in PTY (user can watch/interact)
Complete → agent_state = Done
  → auto_transition_completed_tasks() [app.rs]
    → move_task_to_review_by_instance()
    → If AgenticReview: spawn_review_agent() with build_review_prompt()

Review:
  → Manual: user opens review popup → commit/merge/move to done
  → Agentic: second agent reviews → either REVIEW_COMPLETE or REVIEW_REQUEST_CHANGES

Done → merge worktree → clean up → task in Done column
```

---

## Conventions & Patterns

- **Code Locality:** UI rendering, state, and events for each feature live in the same module directory (e.g., `views/tasks/dialogs/add_task.rs` renders the dialog, `views/tasks/events/text_input.rs` handles its keys)
- **Action enum pattern:** Each view returns an action enum (`TasksAction`) that the dispatch layer converts into method calls on `App`. Clean separation of UI events from side effects.
- **Mode-based state:** `TasksState.mode` is a large enum (`TasksMode::Normal | AddingTask | EditingTask | SelectWorktree | ...`) controlling which key handlers are active and what's rendered
- **No unsafe code:** `#![forbid(unsafe_code)]` in main.rs
- **Serialization hot path:** `App` + `Settings` serialize to JSON. `#[serde(skip)]` on runtime-only fields (PTY sessions, event senders, error messages)
- **Classification runs synchronously in a spawned thread**, not async — uses `std::thread::spawn`, sends result back via `mpsc::UnboundedSender`

---

## Gotchas

1. **`select_prompt()` in `worktree.rs` uses time-based random selection** — time-based seeding won't produce varied results if called rapidly. Not a bug for a single user hitting 'a' occasionally.

2. **The add-task prompt (greeting) is NOT the same as the task execution prompt.** The greeting is cosmetic UI text; the actual agent prompt comes from `build_task_prompt()` in `instances/operations.rs`. Confusing them would miss the real customization point.

3. **Classification prompt is separate from task prompt.** When you type "fix the login bug", the AI first classifies it via `ai_classifier.rs` prompt (JSON extraction), then later the task prompt `build_task_prompt()` sends the full title+description to the working agent. Both are hardcoded independently.

4. **Two prompt files matter:**
   - `src/views/tasks/ai_classifier.rs` — classification prompt (JSON format, used once at task creation)
   - `src/views/instances/operations.rs` — task execution prompt (natural language, sent to the working agent)

5. **`omit_no_commit_instruction` flag** controls whether the "Do not commit" warning is appended. Set to `true` for review agents, `false` for normal task agents.

6. **Settings UI has no "Task Prompts" section** — only Shell, IDE, Agent, Persistence, Review sections exist. Adding a task prompt template requires adding a new `SettingItem` variant and section entry.

7. **`settings.json` and `state.json` are separate files.** Settings is the user preferences; state is the full app state. They serialize/deserialize independently.

---

## Navigation Guide — Where to Change X

| What you want to change | File(s) |
|---|---|
| **Add customizable task prompt via settings.json** | `src/views/settings/state.rs` (add field to `Settings` struct, add UI entry) |
| **Modify how the task prompt is built** | `src/views/instances/operations.rs` → `build_task_prompt()` |
| **Modify the classification prompt** | `src/views/tasks/ai_classifier.rs` → `classify_with_provider()` |
| **Modify the review prompt** | `src/app.rs` → `build_review_prompt()` |
| **Modify the add-task dialog greeting prompts** | `src/views/tasks/operations/worktree.rs` → `select_prompt()` |
| **Change settings.json path** | `src/persistence/paths.rs` |
| **Change settings serialization** | `src/persistence/storage.rs` → `save_settings()` / `load_settings()` |
| **Add a new settings section** | `src/views/settings/state.rs` → add to `SettingsSection::ALL`, add `SettingItem` variants, add handling in `start_editing()` |
| **Change task UI rendering** | `src/views/tasks/views/focus/` or `src/views/tasks/views/kanban/` |
| **Change the add-task dialog UI** | `src/views/tasks/dialogs/add_task.rs` |
| **Add a new task action** | Add variant to `TasksAction` in `events/mod.rs`, handle in `dispatch.rs` |

---

## Open Questions / Uncertain Areas

- **Template syntax for task prompts:** If adding a `task_prompt_template` to Settings, what placeholder syntax? `{title}` `{description}` is the obvious choice but needs agreement.
- **Fallback behavior:** If template is empty/missing, should it fall back to the current hardcoded prompt? (Probably yes — backward compatible.)
- **Settings UI for prompt editing:** Text input field for the template? Or a multiline editor? The current Settings UI only has single-line text inputs and enum selections.
- **Per-task vs global override:** Should the template be global (settings.json) or per-task (with a custom prompt field on the Task struct)? The current request says settings.json, but per-task override could be a future extension.

---

## Key Files for the Customizable Task Prompts Feature

| Priority | File | What to do |
|---|---|---|
| **1 (core)** | `src/views/settings/state.rs` | Add `task_prompt_template: Option<String>` field to `Settings`. Add UI entry in Settings section (new section or extend Agent section). |
| **2 (core)** | `src/views/instances/operations.rs` | Modify `build_task_prompt()` to check settings for a template. If set, render with `{title}`/`{description}` placeholders; otherwise use current hardcoded format. |
| **3 (pass-through)** | `src/app.rs` | In `sync_task_instances()`, pass `settings.settings.task_prompt_template` (or the `Settings` ref) into `create_pane_for_task()`. |
| **4 (pass-through)** | `src/views/instances/state.rs` | The `TaskPaneConfig` struct needs a field for the optional template (or accept `Settings` reference). |
| **5 (optional)** | `src/persistence/storage.rs` | No changes needed — JSON serialization handles `Option<String>` automatically. |
