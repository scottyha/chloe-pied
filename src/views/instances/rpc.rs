use serde::{Deserialize, Serialize};
use uuid::Uuid;

const TYPE_FIELD: &str = "type";
const TOOL_NAME_FIELD: &str = "toolName";
const TOOL_ARGUMENTS_FIELD: &str = "args";
const METHOD_FIELD: &str = "method";
const CONFIRM_METHOD: &str = "confirm";
const SELECT_METHOD: &str = "select";
const TITLE_FIELD: &str = "title";
const MESSAGE_FIELD: &str = "message";
const PROMPT_FIELD: &str = "prompt";
const PARAMETERS_FIELD: &str = "params";
const IS_ERROR_FIELD: &str = "isError";
const UNKNOWN_TOOL_NAME: &str = "unknown";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RpcEvent {
    pub pane_id: Uuid,
    pub rpc_type: RpcEventType,
    #[serde(flatten)]
    pub data: serde_json::Value,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum RpcEventType {
    AgentStart,
    AgentEnd,
    TurnStart,
    TurnEnd,
    MessageStart,
    MessageUpdate,
    MessageEnd,
    ToolExecutionStart,
    ToolExecutionUpdate,
    ToolExecutionEnd,
    ExtensionUiRequest,
    Response,
    QueueUpdate,
    CompactionStart,
    CompactionEnd,
    AutoRetryStart,
    AutoRetryEnd,
    ExtensionError,
    Unknown(String),
}

impl RpcEvent {
    #[must_use]
    pub fn tool_name(&self) -> Option<&str> {
        if !self.is_tool_execution_event() {
            return None;
        }

        self.data.get(TOOL_NAME_FIELD)?.as_str()
    }

    #[must_use]
    pub fn tool_arguments(&self) -> Option<&serde_json::Value> {
        if !self.is_tool_execution_event() {
            return None;
        }

        self.data.get(TOOL_ARGUMENTS_FIELD)
    }

    #[must_use]
    pub fn is_permission_request(&self) -> bool {
        if self.rpc_type != RpcEventType::ExtensionUiRequest {
            return false;
        }

        matches!(self.method(), Some(CONFIRM_METHOD | SELECT_METHOD))
    }

    #[must_use]
    pub fn permission_prompt(&self) -> Option<String> {
        if !self.is_permission_request() {
            return None;
        }

        self.prompt_field_from(&self.data).map(ToOwned::to_owned)
    }

    const fn is_tool_execution_event(&self) -> bool {
        matches!(
            self.rpc_type,
            RpcEventType::ToolExecutionStart
                | RpcEventType::ToolExecutionUpdate
                | RpcEventType::ToolExecutionEnd
        )
    }

    fn method(&self) -> Option<&str> {
        self.data.get(METHOD_FIELD)?.as_str()
    }

    fn prompt_field_from<'a>(&'a self, value: &'a serde_json::Value) -> Option<&'a str> {
        let prompt = value
            .get(TITLE_FIELD)
            .or_else(|| value.get(MESSAGE_FIELD))
            .or_else(|| value.get(PROMPT_FIELD))
            .and_then(serde_json::Value::as_str);

        if prompt.is_some() {
            return prompt;
        }

        self.parameters_prompt()
    }

    fn parameters_prompt(&self) -> Option<&str> {
        let parameters = self.data.get(PARAMETERS_FIELD)?;

        parameters
            .get(TITLE_FIELD)
            .or_else(|| parameters.get(MESSAGE_FIELD))
            .or_else(|| parameters.get(PROMPT_FIELD))
            .and_then(serde_json::Value::as_str)
    }
}

/// Try to parse a complete line from the PTY buffer as an RPC event.
#[must_use]
pub fn parse_rpc_line(line: &str) -> Option<(RpcEventType, serde_json::Value)> {
    let trimmed_line = line.trim();

    if trimmed_line.is_empty() {
        return None;
    }

    let json_value: serde_json::Value = serde_json::from_str(trimmed_line).ok()?;
    let rpc_type_text = json_value.get(TYPE_FIELD)?.as_str()?;
    let rpc_type = rpc_type_from_text(rpc_type_text);

    Some((rpc_type, json_value))
}

#[allow(clippy::needless_pass_by_value)]
pub fn handle_rpc_event(application: &mut crate::app::App, event: RpcEvent) {
    let pane_id = event.pane_id;

    let Some(pane) = application.instances.find_pane(pane_id) else {
        log::debug!("RPC event {:?} for closed pane {pane_id}", event.rpc_type);
        return;
    };

    if !pane.rpc_mode {
        log::debug!("RPC event {:?} for non-RPC pane {pane_id}", event.rpc_type);
        return;
    }

    let task_id = application.find_task_id_by_instance(pane_id);

    match event.rpc_type {
        RpcEventType::AgentStart => handle_agent_start(application, pane_id, task_id),
        RpcEventType::AgentEnd => handle_agent_end(application, pane_id),
        RpcEventType::ExtensionUiRequest if event.is_permission_request() => {
            mark_pane_agent_state(
                application,
                pane_id,
                crate::views::instances::AgentState::NeedsPermissions,
            );
        }
        RpcEventType::ToolExecutionStart => {
            record_tool_execution_start(application, pane_id, &event);
        }
        RpcEventType::ToolExecutionEnd => record_tool_execution_end(application, pane_id, &event),
        _ => {
            log::debug!(
                "RPC event {:?} for pane {pane_id} (no state change)",
                event.rpc_type
            );
        }
    }
}

fn handle_agent_start(application: &mut crate::app::App, pane_id: Uuid, task_id: Option<Uuid>) {
    mark_pane_agent_state(
        application,
        pane_id,
        crate::views::instances::AgentState::Running,
    );

    if let Some(task_id) = task_id {
        let vcs_command = &application.settings.settings.vcs_command;
        application
            .tasks
            .move_task_to_in_progress_by_id(task_id, vcs_command);
    }
}

fn handle_agent_end(application: &mut crate::app::App, pane_id: Uuid) {
    mark_pane_agent_state(
        application,
        pane_id,
        crate::views::instances::AgentState::Done,
    );
    application.auto_transition_completed_tasks();
}

fn mark_pane_agent_state(
    application: &mut crate::app::App,
    pane_id: Uuid,
    agent_state: crate::views::instances::AgentState,
) {
    let Some(pane) = application.instances.find_pane_mut(pane_id) else {
        return;
    };

    pane.agent_state = agent_state;
}

fn record_tool_execution_start(application: &mut crate::app::App, pane_id: Uuid, event: &RpcEvent) {
    let Some(pane) = application.instances.find_pane_mut(pane_id) else {
        return;
    };

    let tool_name = event.tool_name().unwrap_or(UNKNOWN_TOOL_NAME);
    pane.add_activity_event(
        crate::views::instances::state::ActivityEventType::ToolUsed,
        format!("Tool: {tool_name}"),
        event.tool_arguments().map(ToString::to_string),
    );
}

fn record_tool_execution_end(application: &mut crate::app::App, pane_id: Uuid, event: &RpcEvent) {
    let Some(pane) = application.instances.find_pane_mut(pane_id) else {
        return;
    };

    let tool_name = event.tool_name().unwrap_or(UNKNOWN_TOOL_NAME);
    let is_error = event
        .data
        .get(IS_ERROR_FIELD)
        .and_then(serde_json::Value::as_bool)
        .unwrap_or(false);
    let description = if is_error {
        format!("Tool finished with error: {tool_name}")
    } else {
        format!("Tool finished: {tool_name}")
    };

    pane.add_activity_event(
        crate::views::instances::state::ActivityEventType::ToolUsed,
        description,
        None,
    );
}

fn rpc_type_from_text(rpc_type_text: &str) -> RpcEventType {
    match rpc_type_text {
        "agent_start" => RpcEventType::AgentStart,
        "agent_end" => RpcEventType::AgentEnd,
        "turn_start" => RpcEventType::TurnStart,
        "turn_end" => RpcEventType::TurnEnd,
        "message_start" => RpcEventType::MessageStart,
        "message_update" => RpcEventType::MessageUpdate,
        "message_end" => RpcEventType::MessageEnd,
        "tool_execution_start" => RpcEventType::ToolExecutionStart,
        "tool_execution_update" => RpcEventType::ToolExecutionUpdate,
        "tool_execution_end" => RpcEventType::ToolExecutionEnd,
        "extension_ui_request" => RpcEventType::ExtensionUiRequest,
        "response" => RpcEventType::Response,
        "queue_update" => RpcEventType::QueueUpdate,
        "compaction_start" => RpcEventType::CompactionStart,
        "compaction_end" => RpcEventType::CompactionEnd,
        "auto_retry_start" => RpcEventType::AutoRetryStart,
        "auto_retry_end" => RpcEventType::AutoRetryEnd,
        "extension_error" => RpcEventType::ExtensionError,
        unknown_type => RpcEventType::Unknown(unknown_type.to_owned()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn parses_agent_start_event() {
        let (rpc_type, data) = parse_rpc_line(r#"{"type":"agent_start"}"#).unwrap();

        assert_eq!(rpc_type, RpcEventType::AgentStart);
        assert_eq!(data[TYPE_FIELD], "agent_start");
    }

    #[test]
    fn parses_agent_end_event_with_messages() {
        let line = r#"{"type":"agent_end","messages":[{"role":"assistant","content":[{"type":"text","text":"Done"}]}]}"#;
        let (rpc_type, data) = parse_rpc_line(line).unwrap();

        assert_eq!(rpc_type, RpcEventType::AgentEnd);
        assert_eq!(data["messages"][0]["role"], "assistant");
    }

    #[test]
    fn parses_tool_execution_start_event() {
        let line = r#"{"type":"tool_execution_start","toolCallId":"call_123","toolName":"bash","args":{"command":"ls"}}"#;
        let (rpc_type, data) = parse_rpc_line(line).unwrap();

        assert_eq!(rpc_type, RpcEventType::ToolExecutionStart);
        assert_eq!(data[TOOL_NAME_FIELD], "bash");
    }

    #[test]
    fn parses_extension_ui_confirm_request() {
        let line = r#"{"type":"extension_ui_request","id":"abc","method":"confirm","message":"Allow tool?"}"#;
        let (rpc_type, data) = parse_rpc_line(line).unwrap();

        assert_eq!(rpc_type, RpcEventType::ExtensionUiRequest);
        assert_eq!(data[METHOD_FIELD], CONFIRM_METHOD);
    }

    #[test]
    fn empty_line_returns_none() {
        assert!(parse_rpc_line("   ").is_none());
    }

    #[test]
    fn non_json_line_returns_none() {
        assert!(parse_rpc_line("plain terminal output").is_none());
    }

    #[test]
    fn partial_json_returns_none() {
        assert!(parse_rpc_line(r#"{"type":"agent_start"#).is_none());
    }

    #[test]
    fn unknown_event_type_maps_to_unknown() {
        let (rpc_type, _) = parse_rpc_line(r#"{"type":"future_event"}"#).unwrap();

        assert_eq!(rpc_type, RpcEventType::Unknown("future_event".to_owned()));
    }

    #[test]
    fn helper_methods_extract_tool_fields() {
        let event = RpcEvent {
            pane_id: Uuid::nil(),
            rpc_type: RpcEventType::ToolExecutionStart,
            data: json!({
                "type": "tool_execution_start",
                "toolName": "bash",
                "args": {"command": "ls"}
            }),
        };

        assert_eq!(event.tool_name(), Some("bash"));
        assert_eq!(event.tool_arguments(), Some(&json!({"command": "ls"})));
    }

    #[test]
    fn helper_methods_detect_permission_request() {
        let event = RpcEvent {
            pane_id: Uuid::nil(),
            rpc_type: RpcEventType::ExtensionUiRequest,
            data: json!({
                "type": "extension_ui_request",
                "method": "confirm",
                "title": "Allow command?"
            }),
        };

        assert!(event.is_permission_request());
        assert_eq!(event.permission_prompt(), Some("Allow command?".to_owned()));
    }

    #[test]
    fn helper_methods_extract_nested_permission_prompt() {
        let event = RpcEvent {
            pane_id: Uuid::nil(),
            rpc_type: RpcEventType::ExtensionUiRequest,
            data: json!({
                "type": "extension_ui_request",
                "method": "select",
                "params": {"message": "Choose an option"}
            }),
        };

        assert!(event.is_permission_request());
        assert_eq!(
            event.permission_prompt(),
            Some("Choose an option".to_owned())
        );
    }
}
