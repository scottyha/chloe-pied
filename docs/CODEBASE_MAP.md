# Chloe-pied Codebase Map

> Last mapped: 2026-05-07T22:00:00Z
> Total files: 125 Rust source files
> Total lines: ~18,750 lines
> Chunks: 1 (monolithic analysis)

---

## System Overview

Chloe-pied is a terminal-based task management TUI (Ratatui + Crossterm) that manages a Kanban/Focus board of tasks, each of which can be sent to an AI agent (`pi`) for execution inside a PTY pane. It integrates with git worktrees for isolated development branches per task.

```
User Input → crossterm events → dispatch.rs routing → state mutation → Ratatui render
                              ↗ AppEvent channel (PTY output, hook events, classification results)
```

**Three key runtime loops** (all in `event_loop.rs`):
1. **Keyboard input** — crossterm `EventStream`, routed by `dispatch.rs` to active tab
2. **Background events** — `AppEvent` channel (PTY output, hook notifications, classification results, roadmap generation)
3. **Tick interval** (100ms) — spinner animation, worktree polling, auto-transition of completed tasks

**High-level data flow for a task:**
1. User types a brief description → AI classifies it (title, description, type) → stored in Planning column
2. User moves task to In Progress → git worktree created → `pi` agent spawned in PTY pane with task prompt
3. Agent runs in PTY — activity events detected and logged in real-time
4. Agent completes → auto-moved to Review → manual review popup or second agent reviews
5. Review passed → merge worktree → move to Done

---

## Annotated Directory Structure

```
src/
├── main.rs                     # Entry point: terminal init, App::load_or_default(), event loop
├── lib.rs                      # Module declarations, safety test
├── cli.rs                      # CLI args (clap): init, notify subcommands for hook integration
├── app.rs                      # ★ App struct: central state coordinator (6 tabs) + cached historical activity events
├── activity/                   # ★ Shared activity types used by instances + task details
│   ├── mod.rs                  # Module exports
│   └── types.rs                # ActivityEvent, ActivityEventType, ActivitySummary, ActivitySummaryMode
├── events/
│   ├── mod.rs                  # EventHandler trait, EventResult, AppAction enum
│   ├── app.rs                  # AppEvent enum (PtyOutput, PtyExit, Classification, Hook, Roadmap)
│   ├── dispatch.rs             # ★ Routes keyboard events + app events to active tab handlers
│   ├── event_loop.rs           # ★ Async event loop: crossterm stream + AppEvent channel + tick
│   └── hook.rs                 # Unix socket listener for git hook events (start/end/permission)
├── types/
│   ├── mod.rs                  # Re-exports
│   ├── provider.rs             # AgentProvider (Pi), ProviderConfig, ProviderRegistry, DetectedProvider
│   ├── permissions.rs          # PermissionConfig, PermissionPreset (Restrictive/Balanced/Permissive)
│   ├── review_mode.rs          # ReviewMode (Human/Agentic)
│   └── errors.rs               # AppError, Result<App, AppError>
├── providers/
│   ├── mod.rs                  # ProviderSpec trait: build_command(), build_files(), build_oneshot_command()
│   └── pi.rs                   # Pi provider implementation
├── helpers/
│   └── text.rs                 # Text truncation utility
├── persistence/                # ★ State serialization layer
│   ├── mod.rs                  # Module exports
│   ├── activity_log.rs         # ★ append/load/prune activity.jsonl event log
│   ├── paths.rs                # Config directory & file path resolution
│   └── storage.rs              # ★ save/load for state.json + settings.json (JSON via serde)
├── views/
│   ├── mod.rs                  # Main render() dispatcher + re-exports
│   ├── tab_bar.rs              # Tab bar widget
│   ├── layout.rs               # Main layout: panes, tabs, footer split
│   ├── footer.rs               # Status bar content (StatusBarContent struct)
│   │
│   ├── tasks/                  # ★ PRIMARY FEATURE: Task management
│   │   ├── mod.rs              # Module exports
│   │   ├── state.rs            # ★ TasksState, Task, Column, TaskType, TasksMode (big enum)
│   │   ├── dispatch.rs         # ★ Routes TasksAction → App methods (create, delete, open_in_ide, etc.)
│   │   ├── ai_classifier.rs    # Classification: spawns thread calling `pi -p` with JSON prompt
│   │   ├── events/
│   │   │   ├── mod.rs          # TasksAction enum + handle_key_event() master dispatcher
│   │   │   ├── text_input.rs   # AddingTask / EditingTask mode key handlers
│   │   │   ├── kanban_navigation.rs  # All kanban keyboard navigation
│   │   │   ├── focus_navigation.rs   # Focus view keyboard navigation
│   │   │   ├── terminal.rs     # TerminalFocused/Scroll mode in task view
│   │   │   ├── provider_selection.rs # Provider selection dialog navigation
│   │   │   ├── dialogs.rs      # Confirm delete, confirm move-back dialogs
│   │   │   └── worktree_selection.rs # Worktree selection dialog navigation
│   │   ├── operations/
│   │   │   ├── mod.rs          # TaskReference struct, query re-exports
│   │   │   ├── crud.rs         # add_task_to_planning(), delete_task_by_id(), update_task_title()
│   │   │   ├── movement.rs     # ★ Column-to-column movement: move_task_next(), move_task_previous(),
│   │   │   │                     move_to_review(), move_to_done(), move_to_in_progress()
│   │   │   ├── classification.rs # start_classification(), handle_classification_completed()
│   │   │   ├── navigation.rs   # Focus view: select_next/prev, clamp_selection
│   │   │   ├── queries.rs      # get_active_tasks(), get_done_tasks(), count helpers
│   │   │   └── worktree.rs     # ★ begin_add_task(), select_prompt(), create_worktree_for_new_task()
│   │   ├── dialogs/
│   │   │   ├── mod.rs          # Shared dialog helpers
│   │   │   ├── add_task.rs     # AddTaskDialog render
│   │   │   ├── exit_confirmation.rs
│   │   │   ├── merge_confirmation.rs
│   │   │   ├── provider_selection.rs
│   │   │   ├── worktree_selection.rs
│   │   │   └── review/         # Review popup: file_list, diff, output panels + events
│   │   └── views/
│   │       ├── mod.rs          # View dispatcher (Kanban vs Focus)
│   │       ├── kanban/         # Kanban: columns.rs, helpers.rs, view.rs
│   │       └── focus/          # Focus: task_list, done_tasks, details_panel, terminal_panel, view
│   │
│   ├── instances/              # ★ PTY instance management + activity tracking
│   │   ├── mod.rs              # Module exports
│   │   ├── state.rs            # ★ InstanceState, InstancePane, PaneNode (binary tree), AgentState
│   │   │                        #   Re-exports shared activity types from src/activity/types.rs
│   │   ├── operations.rs       # ★ create_pane(), create_pane_for_task(), pane navigation
│   │   │                        #   build_task_prompt(), build_notification_command()
│   │   │                        #   TaskPaneConfig struct
│   │   ├── pty.rs              # PtySession (alacritty_terminal), SpawnOptions, read/write thread
│   │   ├── layout.rs           # Pane layout: tree-based area calculation, split direction
│   │   ├── activity.rs         # ★ Activity event detection: command, file change, error, completion
│   │   ├── view.rs             # ★ Pane rendering, activity summary overlay
│   │   ├── action.rs           # TerminalAction enum
│   │   └── events.rs           # Instance keyboard events (navigation, focus, scroll, activity)
│   │
│   ├── settings/               # Settings UI
│   │   ├── mod.rs              # Module exports
│   │   ├── state.rs            # Settings struct, SettingsState, SettingItem enum, SettingsMode
│   │   ├── events.rs           # Settings keyboard events
│   │   ├── action.rs           # Settings action dropdown/toggle rendering
│   │   └── view.rs             # Settings view layout
│   │
│   ├── worktree/               # Git worktree management
│   │   ├── mod.rs              # Module exports, WorktreeInfo struct
│   │   ├── state.rs            # WorktreeTabState
│   │   ├── view.rs             # Worktree tab rendering
│   │   ├── operations.rs       # git worktree add/delete/list, find_repository_root, status
│   │   ├── action.rs           # WorktreeAction enum
│   │   ├── tab_events.rs       # Worktree tab key event handling
│   │   └── tab_state.rs        # WorktreeTabState detail
│   │
│   ├── roadmap/                # Roadmap view (prioritized items with status)
│   │   ├── mod.rs              # Module exports, RoadmapAction enum
│   │   ├── state.rs            # RoadmapState, RoadmapItem, RoadmapPriority, RoadmapStatus
│   │   ├── events.rs           # Roadmap keyboard events
│   │   ├── operations.rs       # Item lifecycle: add/edit/delete/convert
│   │   ├── generator.rs        # AI-powered roadmap generation
│   │   ├── action.rs           # Roadmap action rendering
│   │   └── view/               # Details panel, dialogs, items list
│   │
│   └── pull_requests/          # PR view
│       ├── mod.rs              # Module exports, PullRequestAction enum
│       ├── state.rs            # PullRequestsState
│       ├── events.rs           # PR keyboard events
│       ├── operations.rs       # Refresh PRs, open in browser
│       └── view.rs             # PR list rendering
│
└── widgets/                    # Reusable UI widgets
    ├── mod.rs                  # Module exports
    ├── spinner.rs              # Loading spinner
    ├── activity_summary.rs     # ★ ActivitySummaryWidget: renders popup with since/full-history modes
    ├── activity_digest.rs      # ★ Compact inline activity digest for task details panel
    ├── agent_indicator.rs      # Agent state dot (Idle/Running/Done/NeedsPermissions)
    ├── dialogs/                # Confirm, Error, Input (with scroll) dialogs
    │   ├── mod.rs, confirm.rs, error.rs, input.rs, style.rs
    └── terminal/               # Terminal screen rendering (alacritty_terminal integration)
        ├── mod.rs, traits.rs, view.rs, cursor.rs, colors.rs, alacritty_impl.rs
```

---

## Module Deep Dives

### 1. app.rs — State Coordinator

**`App` struct** (6 tab states, fully serializable to JSON):
```rust
pub struct App {
    pub active_tab: Tab,              // Tasks, Instances, Roadmap, Worktree, PullRequests, Settings
    pub tasks: TasksState,
    pub instances: InstanceState,
    pub roadmap: RoadmapState,
    pub worktree: WorktreeTabState,
    pub pull_requests: PullRequestsState,
    pub settings: SettingsState,       // #[serde(skip)] — loaded separately
    pub showing_exit_confirmation: bool, // #[serde(skip)]
    pub cached_historical_events: HashMap<Uuid, Vec<ActivityEvent>>, // #[serde(skip)]
    event_sender: Option<mpsc::UnboundedSender<AppEvent>>, // #[serde(skip)]
}
```

Key orchestration methods:
- **`load_or_default()`** — loads state.json, settings.json, prunes/loads activity.jsonl into cached_historical_events, restores session
- **`save()`** — serializes full App to state.json
- **`sync_task_instances()`** — creates PTY panes for tasks in In Progress that lack instances
- **`jump_to_task_instance()`** — switches to Instances tab, focuses the task's pane
- **`auto_transition_completed_tasks()`** — moves done-agent tasks to Review column; spawns agentic review
- **`spawn_review_agent()`** — creates a second PTY pane with the code-review skill
- **`process_hook_event()`** — updates agent state from git hook notifications (start/end/permission)
- **`commit_task_changes()`**, **`merge_task_branch()`** — lifecycle actions
- **`refresh_tasks_from_disk()`**, **`refresh_roadmap_from_disk()`** — reload cross-process changes

### 2. Persistence Layer — What's Saved and Where

**Two JSON files plus one append-only JSONL activity log, loaded independently:**

| File | Path | Contains | Schema |
|---|---|---|---|
| `state.json` | `.chloe-pied/state.json` | Full app state | `App` struct (tasks, instances, roadmap, worktree tab, pull requests) |
| `settings.json` | `.chloe-pied/settings.json` | User preferences + local overrides | `Settings` struct with override merging |
| `activity.jsonl` | `.chloe-pied/activity.jsonl` | Persisted activity events | Appended JSON per `ActivityEvent` |

**Serialization rules:**
- `#[serde(skip)]` on: `pty_session`, `event_sender`, `last_render_area`, `pane_areas`, scroll state, error messages, pending actions, spinner frames
- `#[serde(default)]` on: `activity_events`, `agent_state`, `provider`, `kind`, `instance_id`, `review_instance_id`, `is_paused`, `worktree_info` — ensures backward compat
- **`activity_events: VecDeque<ActivityEvent>` IS serialized in `state.json`** for active panes, AND appended to `activity.jsonl` for persistence, cold start, and closed-pane fallback.
- **`App::cached_historical_events` is runtime-only** and loaded from `activity.jsonl` at startup so task details can render historical activity without rereading the log on every frame.
- PTY sessions are **never** serialized — live terminal state is lost on restart

**Settings override chain:**
1. Global defaults (hardcoded in `Settings::default()`)
2. Global settings file (`~/.config/chloe-pied/settings.json`)
3. Local settings file (`.chloe-pied/settings.json`) — merges only specified fields via `SettingsOverrides`

### 3. Activity Tracking (Existing — Key Feature for Digest View)

**Already implemented — what exists:**

Shared activity types live in `src/activity/types.rs` and are re-exported by `views/instances/state.rs` for instance code compatibility. They are used by both instance panes and task details.

**`ActivityEventType`** (in `activity/types.rs`):
```rust
pub enum ActivityEventType {
    CommandExecuted,
    FileChanged,
    TaskCompleted,
    ErrorOccurred,
    ProviderNotification,
}
```

**`ActivityEvent`** (in `activity/types.rs`):
```rust
pub struct ActivityEvent {
    pub pane_id: Uuid,
    pub timestamp: DateTime<Utc>,
    pub event_type: ActivityEventType,
    pub description: String,
    pub metadata: Option<String>,
}
```

**`ActivitySummary`** (in `activity/types.rs`):
```rust
pub struct ActivitySummary {
    pub since: DateTime<Utc>,              // last_viewed_at or first event for full history
    pub elapsed_seconds: i64,
    pub commands_executed: Vec<String>,
    pub files_changed: Vec<String>,
    pub errors: Vec<String>,
    pub notifications: Vec<String>,
    pub tasks_completed: usize,
}
```

**Detection pipeline** (in `instances/activity.rs`):
- `detect_and_log_activity(pane, output)` — called from `InstanceState::process_pty_output()` on every PTY output chunk
- Regex patterns for: shell commands (`$ cmd`), file changes (Created/Writing/Wrote), errors (Error:/Exception:), task completion keywords, provider notifications (pi:/Agent:/Assistant:)
- Adds `ActivityEvent` to `pane.activity_events` (VecDeque, max 500, retention 7 days)
- `prune_old_activity_events()` called on every add + on load

**ActivitySummary generation** (in `instances/state.rs` + `activity/types.rs`):
- `generate_activity_summary(mode)` — filters by `ActivitySummaryMode::SinceLastViewed` or `ActivitySummaryMode::FullHistory`
- `ActivitySummary::from_events()` categorizes events into summary fields
- Has `format_as_text()` and `format_as_summary_line()` formatters

**ActivitySummary UI** (`widgets/activity_summary.rs` and `widgets/activity_digest.rs`):
- `ActivitySummaryWidget` — renders a centered popup with header (elapsed time) + categorized lists
- Supports toggling between `SinceLastViewed` (default) and `FullHistory` modes via `f`
- Triggered by pressing `A` in Instances tab → sets `InstanceMode::ActivitySummary`
- Scrollable via j/k/Ctrl+d/u/g
- `activity_digest.rs` formats a compact inline activity digest for the Task Details panel.

**What works for activity:**
- ✅ Real-time activity event detection from PTY output
- ✅ Dual-tracked events: in-memory `VecDeque` fast path + append-only `activity.jsonl` persistence path
- ✅ Persistence to both `state.json` (active panes) and `activity.jsonl` (cold start / closed-pane fallback)
- ✅ Historical aggregation: Tasks show digest of their activity history from active pane memory or `App::cached_historical_events` after pane closes
- ✅ Pruning on load
- ✅ Per-pane activity summary popup with full-history toggle
- ✅ Scrollable digest view with categories

### 4. Task State (`views/tasks/state.rs`)

**`Task` struct:**
```rust
pub struct Task {
    pub id: Uuid,
    pub title: String,
    pub description: String,
    pub created_at: DateTime<Utc>,
    pub kind: TaskType,              // Feature | Bug | Chore | Task
    pub provider: Option<AgentProvider>,
    pub instance_id: Option<Uuid>,   // Link to InstancePane
    pub review_instance_id: Option<Uuid>,
    pub is_paused: bool,
    pub worktree_info: Option<WorktreeInfo>,
    pub is_classifying: bool,        // #[serde(skip)]
}
```

**Column structure:** 4 columns (Planning, In Progress, Review, Done), each a `Vec<Task>`.

**`TasksState.mode`** — controls keyboard handling with variants for all dialogs:
```rust
TasksMode::Normal | TerminalFocused | TerminalScroll |
AddingTask { input, prompt } | SelectWorktree { ... } | SelectProvider { ... } |
EditingTask { ... } | ConfirmDelete | ConfirmMoveBack |
ReviewPopup { ... } | ReviewRequestChanges { ... } | MergeConfirmation { ... }
```

### 5. Instance Module (`views/instances/`)

**Pane layout** — binary tree of `PaneNode`:
```rust
pub enum PaneNode {
    Leaf(Box<InstancePane>),
    Split {
        direction: SplitDirection,  // Horizontal | Vertical
        ratio: f32,
        first: Box<PaneNode>,
        second: Box<PaneNode>,
    },
}
```

**`InstancePane`:**
```rust
pub struct InstancePane {
    pub id: Uuid,
    pub name: Option<String>,              // Display name
    pub working_directory: PathBuf,
    pub provider: AgentProvider,
    pub rows: u16, pub columns: u16,
    pub pty_session: Option<PtySession>,   // #[serde(skip)]
    pub pty_spawn_error: Option<String>,   // #[serde(skip)]
    pub agent_state: AgentState,           // Idle | Running | NeedsPermissions | Done
    pub scroll_offset: usize,              // #[serde(skip)]
    pub last_viewed_at: Option<DateTime<Utc>>,
    pub activity_events: VecDeque<ActivityEvent>,  // ★ SERIALIZED
}
```

**`InstanceState` modes and popup state:**
```rust
pub enum InstanceMode {
    Normal,           // Pane navigation (hjkl), Enter to focus
    Focused,          // Keystrokes go to PTY
    Scroll,           // Scroll PTY history
    ActivitySummary,  // Scroll activity digest popup
}

pub activity_summary_mode: ActivitySummaryMode,  // SinceLastViewed by default; f toggles FullHistory
```


**`PtySession`** (in `pty.rs`):
- Spawned via `alacritty_terminal::tty` in a `std::thread::spawn` reader that sends `AppEvent::PtyOutput` / `AppEvent::PtyExit`
- Writes via `session.write_input(data)` — returns `anyhow::Result<()>`
- Resize support via `session.resize(rows, cols)`
- Term accessible via `session.term()` → `Arc<Mutex<Term<EventProxy>>>`

### 6. Event System

**`AppEvent`** (internal async channel):
```rust
PtyOutput { pane_id, data }           → dispatch::handle_app_event → instances.process_pty_output()
PtyExit { pane_id }                   → instances.handle_pty_exit()
ClassificationCompleted { task_id, result } → tasks.handle_classification_completed()
RoadmapGenerationCompleted { result } → roadmap.handle_generation_completed()
HookReceived(HookEvent)               → app.process_hook_event()
```

**`AppAction`** (synchronous return from EventHandler):
```rust
Terminal(TerminalAction)   → instances write input
Roadmap(RoadmapAction)     → roadmap status
PullRequest(PullRequestAction) → PR refresh/open
Worktree(WorktreeAction)   → IDE/terminal open
Settings(SettingsAction)   → settings save
```

**`HookEvent`** (Unix socket JSON):
```rust
{ event: "start"|"end"|"permission", worktree_id: Uuid, timestamp: u128 }
```

---

## Data Flow: Full Task Lifecycle

```
User presses 'a'
  → begin_add_task() [operations/worktree.rs]
    → random prompt selected from 20 hardcoded strings
  → UI shows dialog [dialogs/add_task.rs]
  → User types + Enter
    → TasksAction::CreateTask { title }
  → start_classification() [operations/classification.rs] + [ai_classifier.rs]
    → pi -p "Classify this task..." (oneshot mode)
    → Parses JSON → ClassificationCompleted event
    → Task now has: title, description, task_type
  → Task sits in Planning column

User presses Enter on task (in Kanban) or 'e' (in Focus)
  → move_task_next() [operations/movement.rs]
    → For In Progress: begin_worktree_selection_for_task()
      → User picks: AutoCreate, InitLocalRepo, CreateOnGitHub, or Existing worktree
    → move_task_to_in_progress_with_worktree()
      → Creates git worktree
      → Sets pending_instance_creation flag

sync_task_instances() [app.rs] (called from process_pending_actions)
  → create_pane_for_task() [instances/operations.rs]
    → build_task_prompt(title, description, vcs_command, task_prompt_template)
    → spec.build_command_with_config(&prompt) → "pi \"<prompt>\""
    → Shell wrapper: ( export KEY=VAL; pi "<prompt>"; notify end ) + exec $SHELL
    → Spawns PtySession with command in worktree directory

Agent runs in PTY (user can watch/interact, switch to to other panes)
  → PTY output → AppEvent::PtyOutput → instances.process_pty_output()
    → detect_and_log_activity() [activity.rs] — regex-parses output for commands/files/errors
  → User sees activity in real-time, can view ActivitySummary with 'A'

Agent completes → PTY exits → agent_state = Done
  → auto_transition_completed_tasks() [app.rs]
    → move_task_to_review_by_instance()
    → If AgenticReview: spawn_review_agent() with build_review_prompt()
      → Creates second PTY pane running pi --skill code-review

Review:
  → Manual: user opens review popup → commit / merge & complete / move to done
    → Review popup shows: file list (from git diff), diff content, review output
  → Agentic: second agent runs code-review skill
    → Either REVIEW_COMPLETE (commit + done) or REVIEW_REQUEST_CHANGES (back to In Progress)

Done:
  → merge worktree → clean up → task in Done column
  → Persistent pane state stays in state.json
```

---

## Activity Tracking Deep Dive (Existing Infrastructure for Digest View)

### What's Captured

| Event Type | Trigger | Example |
|---|---|---|
| `CommandExecuted` | Shell prompt regex `^$ cmd` | `Executed: cargo build` |
| `FileChanged` | "Created/Writing/Wrote/Modified" + file extension regex | `Modified: src/main.rs` |
| `TaskCompleted` | Keywords: "task complete", "done", "finished" | `Task marked as complete` |
| `ErrorOccurred` | "Error:", "Exception:", "Failed:" patterns | `Error: compilation error` |
| `ProviderNotification` | "pi:", "Agent:", "Assistant:" patterns | `Processing request...` |

### Architecture and Limitations

- **Storage duality:** Activity is stored in `InstancePane.activity_events` (serialized to `state.json`) AND appended to `activity.jsonl`.
- **Fast path:** Active pane activity reads from the pane's in-memory `VecDeque<ActivityEvent>`.
- **Persistence path:** `src/persistence/activity_log.rs` appends each event as JSONL, loads all persisted events at startup, and prunes old events.
- **Closed-pane fallback:** `App::cached_historical_events` is populated from `activity.jsonl` during `load_or_default()`. Task Focus view uses this runtime cache when no active pane exists for the task/review instance.
- **Activity event pruning happens on load.** `prune_events()` removes events older than 7 days from `activity.jsonl`.

---

## Key Files for Activity History Persistence and Digest View

| Area | File | Role |
|---|---|---|
| Shared types | `src/activity/types.rs` | Defines `ActivityEvent`, `ActivityEventType`, `ActivitySummary`, and `ActivitySummaryMode` for both instance and task views. |
| Activity module export | `src/activity/mod.rs` | Exposes shared activity types module. |
| Detection | `src/views/instances/activity.rs` | Regex-heuristic detection from PTY output: commands, file changes, errors, completions, provider notifications. |
| Instance storage | `src/views/instances/state.rs` | Keeps active pane `VecDeque<ActivityEvent>`, appends events to `activity.jsonl`, generates since/full-history summaries. |
| JSONL persistence | `src/persistence/activity_log.rs` | Append-only `.chloe-pied/activity.jsonl` writer plus startup load and retention pruning. |
| Runtime cache | `src/app.rs` | `cached_historical_events` stores loaded JSONL events for closed-pane task details fallback. |
| Popup widget | `src/widgets/activity_summary.rs` | Scrollable activity summary popup; `f` toggles `SinceLastViewed`/`FullHistory`. |
| Inline digest | `src/widgets/activity_digest.rs` | Compact activity digest embedded in the task details panel. |
| Task details integration | `src/views/tasks/views/focus/details_panel.rs` | Renders inline activity digest. |
| Activity source selection | `src/views/tasks/views/focus/view.rs` | Chooses active pane events first, then `cached_historical_events` for closed panes. |

---

## Serialization Schema (state.json)

The `App` struct serializes to `state.json` with this structure:

```json
{
  "active_tab": "Tasks",
  "tasks": {
    "columns": [
      { "name": "Planning", "tasks": [ { "id": "...", "title": "...", ... } ] },
      { "name": "In Progress", "tasks": [...] },
      { "name": "Review", "tasks": [...] },
      { "name": "Done", "tasks": [...] }
    ],
    "mode": "Normal",
    "view_mode": "Focus",
    "kanban_selected_column": 0,
    "kanban_selected_task": null,
    "focus_active_index": 0,
    "focus_done_index": 0,
    "focus_panel": "ActiveTasks",
    "focus_details_scroll": 0
  },
  "instances": {
    "root": {
      "Leaf" | "Split"  { ... tree of panes ... }
    },
    "selected_pane_id": "...",
    "mode": "Normal"
  },
  "roadmap": {
    "items": [
      { "id": "...", "title": "...", "priority": "Medium", "status": "Planned", ... }
    ],
    "selected_item": null,
    "mode": "Normal"
  },
  "worktree": { ... },
  "pull_requests": { ... }
}
```

Note: `settings` is NOT in state.json — it's in a separate `settings.json` file.

---

## Dependencies

| Crate | Version | Purpose |
|---|---|---|
| ratatui | 0.30 | Terminal UI framework |
| crossterm | 0.29 | Terminal backend + event-stream feature |
| tokio | 1 | Async runtime (event loop, channels) |
| serde + serde_json | 1.0 | Serialization to state.json / settings.json |
| uuid | 1.23 | Unique IDs for tasks, panes, roadmap items |
| chrono | 0.4 | Timestamps, date formatting, duration arithmetic |
| alacritty_terminal | 0.25 | PTY session + terminal emulation |
| clap | 4.6 | CLI argument parsing |
| git2 | 0.20 | Git operations (worktrees, merge, status) |
| regex | 1.11 | Activity detection patterns |
| dirs | 6.0 | Global config directory resolution |
| similar | 2.6 | Diff computation for review |
| anyhow | 1 | Error handling |
| futures | 0.3 | Async stream utilities |

---

## Conventions & Patterns

- **Code Locality:** UI rendering, state, and events for each feature in the same module directory
- **Action enum pattern:** Each view returns an action enum (`TasksAction`, `AppAction`) that dispatch converts to `App` method calls
- **Mode-based state:** `TasksState.mode` (16 variants) controls keyboard handlers + render; `InstanceMode` (4 variants) for instance interaction
- **No unsafe code:** `#![forbid(unsafe_code)]` at crate level
- **#[serde(skip)] vs #[serde(default)]:** runtime-only fields (PTY, event senders, errors) get `skip`; optional persisted fields (activity_events, provider) get `default`
- **Synchronous classification in spawned thread:** not async — uses `std::thread::spawn`, sends result via `mpsc::UnboundedSender`
- **Settings overrides:** local `.chloe-pied/settings.json` merges onto global `~/.config/chloe-pied/settings.json` via `SettingsOverrides` — only explicitly set fields override
- **Ui ↔ Event dispatching:** `events/dispatch.rs` routes from crossterm `Event::Key` → tab-specific `EventHandler::handle_key()` → returns `EventResult` (Consumed/Ignored/Action/Quit)

---

## Gotchas

1. **Activity events are per-pane, not per-task.** If a task has two instances (task + review), activity is split across two separate pane event lists. The Focus view can look up both active and cached historical events by pane ID, but there is no project-level aggregation.

2. **Activity detection is regex-heuristic, not deterministic.** False positives/negatives possible. Commands detected from shell prompts (`$ cmd`) miss commands piped through the agent. File change detection looks for natural language patterns from agent output, not actual git diff.

3. **`last_viewed_at` controls the default summary window.** The `ActivitySummaryWidget` defaults to `SinceLastViewed`, but `f` toggles to `FullHistory`. Calling `mark_viewed()` resets the default window; older events remain available through full-history mode and persisted JSONL until retention pruning.

4. **Activity event pruning happens on add + on load.** `prune_old_activity_events()` removes events older than 7 days AND caps at 500 per pane. If a pane has a very long-running session, old events silently disappear.

5. **PTY sessions don't survive restart.** Only pane metadata and activity events persist. The actual terminal buffer, scrollback, and running process are lost.

6. **state.json and settings.json are separate files.** Don't look for settings in state.json. Don't expect state to auto-save when settings change — only `save_settings()` is called on settings edit.

7. **Event Loop runs three concurrent sources.** Crossterm keyboard events, AppEvent channel (background tasks), and a 100ms tick. The `biased` tokio::select! prioritizes keyboard input first.

8. **Activity digest spans four locations:**
   - `src/views/instances/activity.rs` — detection heuristics (how events are captured)
   - `src/activity/types.rs` — shared event and summary types
   - `src/persistence/activity_log.rs` + `App::cached_historical_events` — persisted/closed-pane fallback
   - `src/widgets/activity_digest.rs` — compact task details presentation

---

## Navigation Guide — Where to Change What

| What you want to change | File(s) |
|---|---|
| **Add new activity event type** | `src/activity/types.rs` (ActivityEventType enum), `src/views/instances/activity.rs` (detection), affected widgets |
| **Change activity detection regex** | `src/views/instances/activity.rs` |
| **Change summary time window / mode behavior** | `src/views/instances/state.rs` → `generate_activity_summary(mode)` and `src/activity/types.rs` |
| **Change JSONL activity persistence** | `src/persistence/activity_log.rs` and `src/persistence/paths.rs` |
| **Change closed-pane activity fallback** | `src/app.rs` (`cached_historical_events` load/cache), `src/views/tasks/views/focus/view.rs` |
| **Add cross-pane aggregation** | `src/app.rs` (walk `instances.collect_panes()` plus `cached_historical_events`) or `src/views/instances/state.rs` |
| **Add digest view mode** | `src/views/instances/state.rs` (InstanceMode variant), `events.rs` (key handler), `view.rs` (render) |
| **Rework ActivitySummaryWidget** | `src/widgets/activity_summary.rs` |
| **Rework inline task digest** | `src/widgets/activity_digest.rs` and `src/views/tasks/views/focus/details_panel.rs` |
| **Change state.json schema** | `src/persistence/storage.rs` (serde handles new fields automatically) |
| **Add settings for activity** | `src/views/settings/state.rs` (new SettingItem), `events.rs` (value update) |
| **Change task rendering** | `src/views/tasks/views/focus/` or `kanban/` |
| **Change instance keybindings** | `src/views/instances/events.rs` |
| **Change activity keybindings** | `src/views/instances/events.rs` → `A` opens popup, `f` toggles since/full-history while popup is active |

---

## Open Questions / Uncertain Areas

- **Activity schema evolution:** `ActivityEvent` currently has `metadata: Option<String>`. If we add richer metadata (structured JSON, typed fields), this requires a serde migration. Backward compat via `#[serde(default)]` on new fields is the pattern.
- **Digest view placement:** Current implementation uses an inline task-details digest plus an Instances popup. A full tab or richer split-panel view remains an open UX option.
- **Cross-session timeline:** Activity events persist across sessions and full-history mode exists, but there is no session overview that groups events by session start/end.
- **Performance:** 500 events per pane × 10 panes = 5000 events. Regex on every PTY output chunk scales linearly. No issues today but worth noting for heavy workloads.
- **Level of detail:** Current detection captures commands and file changes. Should it capture git commit messages, build output, test results? The heuristic approach has natural limits — agent output is unstructured text.
