<p align="center">
  <strong>Chloe-pied</strong><br>
  <em>Pi-native task management for your terminal.</em>
</p>

<p align="center">
  <a href="https://github.com/KevinEdry/chloe">
    <img src="https://img.shields.io/badge/upstream-Chloe-blue.svg" alt="Upstream: Chloe">
  </a>
  <a href="https://github.com/scottyha/chloe-pied/blob/main/LICENSE">
    <img src="https://img.shields.io/badge/license-MIT-blue.svg" alt="License: MIT">
  </a>
  <img src="https://img.shields.io/badge/rust-stable-orange.svg" alt="Rust">
  <img src="https://img.shields.io/badge/unsafe-forbidden-success.svg" alt="Unsafe Forbidden">
</p>

---

## What is this?

**Chloe-pied** is a fork of [Chloe](https://github.com/KevinEdry/chloe) adapted for the [Pi coding agent](https://github.com/mariozechner/pi-coding-agent).

Chloe is a terminal-native task manager that embeds AI coding sessions alongside a kanban board and roadmap. Chloe-pied replaces the Claude Code integration with Pi, so your tasks spin up `pi` sessions instead.

If you use Pi as your coding agent and want task management + worktree isolation + embedded terminals in one TUI, this is for you.

## Differences from upstream Chloe

| | Chloe (upstream) | Chloe-pied |
|---|---|---|
| **Agent** | Claude Code, Gemini CLI, Amp, OpenCode | Pi |
| **AI classifier** | Calls Claude Code CLI | Calls Pi CLI (`pi -p`) |
| **Roadmap gen** | Calls Claude Code CLI | Calls Pi CLI (`pi -p`) |
| **Hook system** | Unix socket + `chloe notify` for lifecycle events | Uses `chloe-pied notify` (same socket pattern, compatible) |
| **Config dir** | `.chloe/` | `.chloe-pied/` |
| **Branch prefix** | `chloe/` | `pied/` |
| **Agent settings** | Generates `.claude/settings.local.json` per worktree | Generates `.chloe-pied/settings.local.json` per worktree (minimal hook config) |

## Features (inherited from Chloe)

- **Terminal-native TUI** — Kanban board, focus view, instances, roadmap, all in your terminal
- **Multi-instance panes** — Split terminal panes per task, each running its own agent
- **Git worktree support** — Each task gets its own worktree (or Jujutsu workspace)
- **AI task classification** — Describe a task, get it categorized automatically
- **AI roadmap generation** — Point at a project, get a prioritized feature roadmap
- **Activity tracking** — See what your agent did while you weren't watching
- **~15MB memory footprint** — No Electron, no Node.js, just a Rust binary

## Installation

### From source (requires Rust nightly/2024 edition)

```bash
git clone https://github.com/scottyha/chloe-pied.git
cd chloe-pied
cargo build --release
cp target/release/chloe-pied /usr/local/bin/
```

### Quick start

```bash
cd your-project
chloe-pied init
chloe-pied
```

## Configuration

Settings are stored in `.chloe-pied/settings.json` within each project. The default provider is Pi.

```json
{
  "default_shell": "/bin/bash",
  "auto_save_interval_seconds": 30,
  "ide_command": "VSCode",
  "terminal_command": "AppleTerminal",
  "vcs_command": "Git",
  "default_provider": "Pi",
  "skip_provider_selection": true,
  "provider_registry": {
    "configs": {
      "Pi": {
        "command": "pi",
        "arguments": [],
        "environment": {},
        "working_directory_argument": null,
        "supports_worktree": true
      }
    }
  }
}
```

## Roadmap

Chloe-pied has a few goals beyond the upstream project:

- [ ] **Pi RPC mode integration** — Instead of embedding Pi as a raw PTY process, use `pi --mode rpc` for structured lifecycle events (`session_start`, `agent_end`, `session_shutdown`). Better status detection than process monitoring alone.
- [ ] **Sync layer** — Push/pull `.chloe-pied/state.json` to a shared location (PostgreSQL, git remote, or network drive). External task creation via the sync layer.
- [ ] **Multi-user tasks** — Allow clients/collaborators to inject tasks into a shared project board.

## Attribution

Chloe-pied is a fork of [Chloe](https://github.com/KevinEdry/chloe) by Kevin Edry, licensed under MIT. The upstream project is the foundation — this fork adds Pi integration and changes the config namespace to avoid conflicts.

## License

MIT — see [LICENSE](LICENSE).