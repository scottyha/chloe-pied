# Pi RPC Mode JSONL Format

Investigation command:

```bash
echo '{"type":"prompt","message":"list the files in the current directory"}' \
  | timeout 30 pi --mode rpc --no-session \
  2>/tmp/pi-rpc-stderr.log \
  > /tmp/pi-rpc-output.jsonl
```

Additional probe: a Python subprocess kept stdin open after writing the prompt so Pi could finish the turn and emit `agent_end`.

## Transport findings

- Stdout is JSONL: one complete JSON object per line.
- No non-JSON stdout was observed.
- Stderr was empty in the probes.
- If stdin is closed immediately by `echo ... | pi --mode rpc --no-session`, Pi can exit before the assistant turn completes. In the observed run it emitted `response`, `agent_start`, `turn_start`, and the user `message_*` events, then exited with no `agent_end`.
- If stdin remains open, Pi emits the full lifecycle through `agent_end`.
- After `agent_end`, Pi stays running and waits for another RPC command on stdin. It does not exit automatically while stdin remains open.

## Stdin prompt command

Prompt command format:

```json
{"type":"prompt","message":"say hello and stop"}
```

Pi immediately acknowledges it on stdout:

```json
{"type":"response","command":"prompt","success":true}
```

## Top-level event types observed

### `extension_ui_request`

Emitted by extensions before/during agent execution. These are JSON events on stdout and should be ignored or handled separately by Chloe.

Schemas observed:

```json
{"type":"extension_ui_request","id":"uuid","method":"setStatus","statusKey":"agent-forge"}
{"type":"extension_ui_request","id":"uuid","method":"setWidget","widgetKey":"agent-forge"}
{"type":"extension_ui_request","id":"uuid","method":"notify","message":"Signal Profile: loaded (20218 chars)","notifyType":"info"}
```

### `response`

```json
{"type":"response","command":"prompt","success":true}
```

### `agent_start`

```json
{"type":"agent_start"}
```

### `turn_start`

```json
{"type":"turn_start"}
```

### `message_start`

For user messages:

```json
{
  "type": "message_start",
  "message": {
    "role": "user",
    "content": [{"type":"text","text":"say hello and stop"}],
    "timestamp": 1778178028949
  }
}
```

For assistant messages:

```json
{
  "type": "message_start",
  "message": {
    "role": "assistant",
    "content": [],
    "api": "openai-completions",
    "provider": "ollama",
    "model": "glm-5.1:cloud",
    "usage": {"input":0,"output":0,"cacheRead":0,"cacheWrite":0,"totalTokens":0,"cost":{"input":0,"output":0,"cacheRead":0,"cacheWrite":0,"total":0}},
    "stopReason": "stop",
    "timestamp": 1778178029099
  }
}
```

### `message_update`

General shape:

```json
{
  "type": "message_update",
  "assistantMessageEvent": {
    "type": "text_delta",
    "contentIndex": 0,
    "delta": "Hello",
    "partial": {"role":"assistant","content":[{"type":"text","text":"Hello"}]}
  },
  "message": {"role":"assistant","content":[{"type":"text","text":"Hello"}]}
}
```

Inner `assistantMessageEvent.type` values observed:

- `thinking_start`
- `thinking_delta`
- `thinking_end`
- `text_start`
- `text_delta`
- `text_end`
- `toolcall_start`
- `toolcall_delta`
- `toolcall_end`

Text stream examples:

```json
{"type":"message_update","assistantMessageEvent":{"type":"text_delta","contentIndex":1,"delta":"Hello","partial":{"role":"assistant","content":[{"type":"thinking","thinking":"Simple request - just say hello and stop.","thinkingSignature":"reasoning"},{"type":"text","text":"Hello! 👋"}]}},"message":{"role":"assistant","content":[{"type":"thinking","thinking":"Simple request - just say hello and stop.","thinkingSignature":"reasoning"},{"type":"text","text":"Hello! 👋"}]}}
```

Tool-call stream examples:

```json
{"type":"message_update","assistantMessageEvent":{"type":"toolcall_start","contentIndex":1,"partial":{"role":"assistant","content":[{"type":"thinking","thinking":"The user wants to see the files in the current working directory.","thinkingSignature":"reasoning"},{"type":"toolCall","id":"call_hg2m44ir","name":"bash","arguments":{"command":"ls -la /path"},"partialArgs":"{\"command\":\"ls -la /path\"}","streamIndex":0}],"stopReason":"toolUse"}},"message":{"role":"assistant","content":[{"type":"thinking","thinking":"The user wants to see the files in the current working directory.","thinkingSignature":"reasoning"},{"type":"toolCall","id":"call_hg2m44ir","name":"bash","arguments":{"command":"ls -la /path"},"partialArgs":"{\"command\":\"ls -la /path\"}","streamIndex":0}],"stopReason":"toolUse"}}
```

```json
{"type":"message_update","assistantMessageEvent":{"type":"toolcall_end","contentIndex":1,"toolCall":{"type":"toolCall","id":"call_hg2m44ir","name":"bash","arguments":{"command":"ls -la /path"}},"partial":{"role":"assistant","content":[{"type":"thinking","thinking":"The user wants to see the files in the current working directory.","thinkingSignature":"reasoning"},{"type":"toolCall","id":"call_hg2m44ir","name":"bash","arguments":{"command":"ls -la /path"}}],"stopReason":"toolUse"}},"message":{"role":"assistant","content":[{"type":"thinking","thinking":"The user wants to see the files in the current working directory.","thinkingSignature":"reasoning"},{"type":"toolCall","id":"call_hg2m44ir","name":"bash","arguments":{"command":"ls -la /path"}}],"stopReason":"toolUse"}}
```

### `message_end`

```json
{
  "type": "message_end",
  "message": {
    "role": "assistant",
    "content": [{"type":"text","text":"Hello! 👋"}],
    "usage": {"input":34921,"output":14,"cacheRead":0,"cacheWrite":0,"totalTokens":34935,"cost":{"input":0,"output":0,"cacheRead":0,"cacheWrite":0,"total":0}},
    "stopReason": "stop",
    "timestamp": 1778178029099,
    "responseId": "chatcmpl-503",
    "responseModel": "glm-5.1"
  }
}
```

### Tool execution lifecycle

When Pi executes a tool, separate top-level tool execution events are emitted.

```json
{"type":"tool_execution_start","toolCallId":"call_hg2m44ir","toolName":"bash","args":{"command":"ls -la /path"}}
```

```json
{"type":"tool_execution_update","toolCallId":"call_hg2m44ir","toolName":"bash","args":{"command":"ls -la /path"},"partialResult":{"content":[]}}
```

```json
{"type":"tool_execution_end","toolCallId":"call_hg2m44ir","toolName":"bash","result":{"content":[{"type":"text","text":"...tool stdout..."}]},"isError":false}
```

### `turn_end`

```json
{
  "type": "turn_end",
  "message": {"role":"assistant","content":[{"type":"text","text":"Hello! 👋"}],"stopReason":"stop"},
  "toolResults": []
}
```

With tools, `toolResults` contains the final tool result objects.

### `agent_end`

```json
{
  "type": "agent_end",
  "messages": [
    {"role":"user","content":[{"type":"text","text":"say hello and stop"}],"timestamp":1778178028949},
    {"role":"assistant","content":[{"type":"thinking","thinking":"Simple request - just say hello and stop.","thinkingSignature":"reasoning"},{"type":"text","text":"Hello! 👋"}],"stopReason":"stop"}
  ]
}
```

## Implementation notes for Chloe

- Treat stdout as strict JSONL, but be tolerant of unrelated event types like `extension_ui_request`.
- Keep Pi stdin open until `agent_end` if Chloe wants a complete turn.
- Use `message_update.assistantMessageEvent.type == "text_delta"` for streaming display.
- Use `message_end`, `turn_end`, or `agent_end` for final accumulated assistant content.
- `agent_end` is a lifecycle event, not a process-exit signal.
