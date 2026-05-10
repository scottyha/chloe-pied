# Pi RPC Mode Feature

## What RPC mode is

RPC mode starts Pi with `--mode rpc` and sends the task prompt as a JSONL command over stdin:

```json
{"type":"prompt","message":"..."}
```

It exists so Chloe can observe structured Pi lifecycle events instead of inferring agent state from a shell process exit. Pi emits JSONL events such as `agent_start`, `tool_execution_start`, and `agent_end`; Chloe parses those events and updates pane/task state directly.

## Difference from the shell wrapper approach

The older path builds a full shell command containing the prompt as a CLI argument, wraps it in `bash -c`, and installs an `EXIT` trap that runs `chloe-pied notify end --worktree-id ...`. That works for completion notification, but it only tells Chloe that the shell process exited.

RPC mode keeps Pi as the foreground process and avoids the Chloe notification wrapper. Provider config arguments are still applied, then Chloe appends `--mode rpc`. The prompt is not placed in argv; it is sent through stdin as escaped JSON.

Example command shape:

```bash
pi --no-orchestrator --mode rpc
```

## ProviderConfig setting

`ProviderConfig` has an `rpc_mode` boolean:

```json
{
  "command": "pi",
  "arguments": ["--no-orchestrator"],
  "oneshot_arguments": [],
  "environment": {},
  "working_directory_argument": null,
  "supports_worktree": true,
  "rpc_mode": true
}
```

When `rpc_mode` is true for a provider, task panes use RPC startup. When it is false, Chloe uses the legacy shell wrapper path.

## Lifecycle event mapping

Chloe maps Pi RPC events into pane and task state:

| Pi RPC event | Chloe behavior |
|---|---|
| `agent_start` | Marks the pane `Running`; moves the linked task to In Progress. |
| `agent_end` | Marks the pane `Done`; runs task auto-transition logic. |
| `extension_ui_request` with `confirm` or `select` | Marks the pane `NeedsPermissions`. |
| `tool_execution_start` | Adds a `ToolUsed` activity event with the tool name and arguments. |
| `tool_execution_end` | Adds a `ToolUsed` activity event showing tool completion or error. |
| Other known events | Parsed and logged with no state change. |
| Unknown events | Preserved as `Unknown(...)` and logged with no state change. |

## Backward compatibility

`rpc_mode` uses `#[serde(default)]`, so old provider config JSON without the field still deserializes successfully with `rpc_mode: false`.

Existing configs therefore keep legacy behavior until the field is added or the default provider config is regenerated. New Pi default configs enable RPC mode.

Serialized `ProviderConfig` values include the `rpc_mode` field, making the setting explicit after save.

## Known limitations

- The terminal pane still displays Pi's raw JSONL output in RPC mode.
- Chloe consumes lifecycle events for state updates, but it does not yet render a dedicated structured transcript view.
- There is no interactive continuation after `agent_end`; the pane is treated as completed.
- Permission requests are detected at the pane state level, but full in-TUI permission response handling is not implemented here.
