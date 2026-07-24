//! Mistral AI chat orchestration with Websupport tool calling.
//!
//! # Safety model
//!
//! - **Read-only tools** (`list_*`, `get_dns_zone`, `test_connection`) execute
//!   immediately; results are fed back to the model (max 5 loop iterations).
//! - **Mutating tools** (`create_*`, `update_*`, `delete_*`) are **never**
//!   executed from a tool call. They are returned as [`PendingAction`] values
//!   that require an explicit [`mistral_confirm_action`] call from the UI.
//! - **Unknown tool names** are rejected outright (not treated as read-only
//!   or mutating).

use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use specta::Type;
use uuid::Uuid;

use crate::commands::secrets::get_secret;
use crate::commands::websupport_dns::{
    websupport_create_dns_record, websupport_delete_dns_record, websupport_get_dns_zone,
    websupport_list_dns_records, websupport_test_connection, websupport_update_dns_record,
    CreateDnsRecordInput,
};
use crate::commands::websupport_v1::{
    websupport_create_mailbox, websupport_delete_mailbox, websupport_list_domains,
    websupport_list_hostings, websupport_list_mailboxes, websupport_update_mailbox,
    CreateMailboxInput, UpdateMailboxInput,
};

const MISTRAL_API_URL: &str = "https://api.mistral.ai/v1/chat/completions";
const MISTRAL_MODEL: &str = "mistral-large-latest";
const MISTRAL_API_KEY_SECRET: &str = "mistral_api_key";
const MAX_TOOL_LOOP_ITERATIONS: u32 = 5;

const ALL_TOOL_NAMES: &[&str] = &[
    "list_dns_records",
    "get_dns_zone",
    "create_dns_record",
    "update_dns_record",
    "delete_dns_record",
    "test_connection",
    "list_hostings",
    "list_domains",
    "list_mailboxes",
    "create_mailbox",
    "update_mailbox",
    "delete_mailbox",
];

const MUTATING_TOOL_NAMES: &[&str] = &[
    "create_dns_record",
    "update_dns_record",
    "delete_dns_record",
    "create_mailbox",
    "update_mailbox",
    "delete_mailbox",
];

const READ_ONLY_TOOL_NAMES: &[&str] = &[
    "list_dns_records",
    "get_dns_zone",
    "test_connection",
    "list_hostings",
    "list_domains",
    "list_mailboxes",
];

// ---------------------------------------------------------------------------
// Public specta types
// ---------------------------------------------------------------------------

/// A single chat message exchanged with the UI / model.
#[derive(Debug, Clone, Serialize, Deserialize, Type)]
pub struct ChatMessage {
    pub role: String,
    pub content: String,
}

/// A mutating tool call waiting for explicit user confirmation.
#[derive(Debug, Clone, Serialize, Deserialize, Type)]
pub struct PendingAction {
    /// Stable id (uuid v4) for this pending action.
    pub id: String,
    pub tool_name: String,
    /// Human-readable summary (never includes passwords).
    pub description: String,
    /// Original tool arguments as JSON.
    pub args: Value,
}

/// Result of one user→assistant turn (possibly with pending mutations).
#[derive(Debug, Clone, Serialize, Deserialize, Type)]
pub struct MistralTurnResult {
    pub reply: String,
    pub pending_actions: Vec<PendingAction>,
}

// ---------------------------------------------------------------------------
// Tool classification (pure, unit-tested)
// ---------------------------------------------------------------------------

/// Classification of a tool name for the safety gate.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ToolKind {
    ReadOnly,
    Mutating,
}

/// Classifies a tool name.
///
/// Returns `Err` for unknown names so callers **reject** them rather than
/// treating them as either safe-to-run or confirmable mutations.
pub fn classify_tool(name: &str) -> Result<ToolKind, String> {
    if MUTATING_TOOL_NAMES.contains(&name) {
        Ok(ToolKind::Mutating)
    } else if READ_ONLY_TOOL_NAMES.contains(&name) {
        Ok(ToolKind::ReadOnly)
    } else {
        Err(format!(
            "Unknown tool '{name}'. Allowed tools: {}",
            ALL_TOOL_NAMES.join(", ")
        ))
    }
}

/// Returns `Ok(true)` for mutating tools, `Ok(false)` for read-only tools,
/// and `Err` for unknown tool names (fail-closed).
pub fn is_mutating_tool(name: &str) -> Result<bool, String> {
    match classify_tool(name)? {
        ToolKind::Mutating => Ok(true),
        ToolKind::ReadOnly => Ok(false),
    }
}

// ---------------------------------------------------------------------------
// Tool definitions (OpenAI / Mistral function-calling schema)
// ---------------------------------------------------------------------------

fn tool_definitions() -> Value {
    json!([
        {
            "type": "function",
            "function": {
                "name": "list_dns_records",
                "description": "List all DNS records for a domain (Websupport DNS zone).",
                "parameters": {
                    "type": "object",
                    "properties": {
                        "domain": { "type": "string", "description": "Domain name, e.g. example.com" }
                    },
                    "required": ["domain"]
                }
            }
        },
        {
            "type": "function",
            "function": {
                "name": "get_dns_zone",
                "description": "Get DNS zone metadata for a domain (name, last check, DNSSEC status).",
                "parameters": {
                    "type": "object",
                    "properties": {
                        "domain": { "type": "string" }
                    },
                    "required": ["domain"]
                }
            }
        },
        {
            "type": "function",
            "function": {
                "name": "create_dns_record",
                "description": "Create a DNS record. REQUIRES user confirmation before execution.",
                "parameters": {
                    "type": "object",
                    "properties": {
                        "domain": { "type": "string" },
                        "type": { "type": "string", "description": "Record type: A, AAAA, CNAME, MX, TXT, ..." },
                        "name": { "type": "string", "description": "Record name / host" },
                        "content": { "type": "string", "description": "Record value (IP, target, text, ...)" },
                        "ttl": { "type": "integer" },
                        "priority": { "type": "integer" },
                        "port": { "type": "integer" },
                        "weight": { "type": "integer" }
                    },
                    "required": ["domain", "type", "name", "ttl"]
                }
            }
        },
        {
            "type": "function",
            "function": {
                "name": "update_dns_record",
                "description": "Update an existing DNS record by id. REQUIRES user confirmation.",
                "parameters": {
                    "type": "object",
                    "properties": {
                        "domain": { "type": "string" },
                        "record_id": { "type": "string" },
                        "type": { "type": "string" },
                        "name": { "type": "string" },
                        "content": { "type": "string" },
                        "ttl": { "type": "integer" },
                        "priority": { "type": "integer" },
                        "port": { "type": "integer" },
                        "weight": { "type": "integer" }
                    },
                    "required": ["domain", "record_id", "type", "name", "ttl"]
                }
            }
        },
        {
            "type": "function",
            "function": {
                "name": "delete_dns_record",
                "description": "Delete a DNS record by id. REQUIRES user confirmation.",
                "parameters": {
                    "type": "object",
                    "properties": {
                        "domain": { "type": "string" },
                        "record_id": { "type": "string" }
                    },
                    "required": ["domain", "record_id"]
                }
            }
        },
        {
            "type": "function",
            "function": {
                "name": "test_connection",
                "description": "Test Websupport API credentials (read-only).",
                "parameters": { "type": "object", "properties": {} }
            }
        },
        {
            "type": "function",
            "function": {
                "name": "list_hostings",
                "description": "List Websupport hosting products for the authenticated user.",
                "parameters": { "type": "object", "properties": {} }
            }
        },
        {
            "type": "function",
            "function": {
                "name": "list_domains",
                "description": "List domains (vhosts) under a hosting.",
                "parameters": {
                    "type": "object",
                    "properties": {
                        "hosting_id": { "type": "string" }
                    },
                    "required": ["hosting_id"]
                }
            }
        },
        {
            "type": "function",
            "function": {
                "name": "list_mailboxes",
                "description": "List mailboxes under a hosting.",
                "parameters": {
                    "type": "object",
                    "properties": {
                        "hosting_id": { "type": "string" }
                    },
                    "required": ["hosting_id"]
                }
            }
        },
        {
            "type": "function",
            "function": {
                "name": "create_mailbox",
                "description": "Create a mailbox. REQUIRES user confirmation. Do not echo passwords.",
                "parameters": {
                    "type": "object",
                    "properties": {
                        "hosting_id": { "type": "string" },
                        "domain_id": { "type": "string" },
                        "email": { "type": "string" },
                        "password": { "type": "string" },
                        "ipCheck": { "type": "boolean" },
                        "ips": { "type": "array", "items": { "type": "string" } },
                        "countryCheck": { "type": "boolean" },
                        "countries": { "type": "array", "items": { "type": "string" } },
                        "imapDisabled": { "type": "boolean" },
                        "pop3Disabled": { "type": "boolean" },
                        "note": { "type": "string" }
                    },
                    "required": ["hosting_id", "domain_id", "email", "password"]
                }
            }
        },
        {
            "type": "function",
            "function": {
                "name": "update_mailbox",
                "description": "Update a mailbox. REQUIRES user confirmation. Do not echo passwords.",
                "parameters": {
                    "type": "object",
                    "properties": {
                        "hosting_id": { "type": "string" },
                        "domain_id": { "type": "string" },
                        "mailbox_id": { "type": "string" },
                        "password": { "type": "string" },
                        "ipCheck": { "type": "boolean" },
                        "ips": { "type": "array", "items": { "type": "string" } },
                        "countryCheck": { "type": "boolean" },
                        "countries": { "type": "array", "items": { "type": "string" } },
                        "imapDisabled": { "type": "boolean" },
                        "pop3Disabled": { "type": "boolean" },
                        "note": { "type": "string" }
                    },
                    "required": ["hosting_id", "domain_id", "mailbox_id"]
                }
            }
        },
        {
            "type": "function",
            "function": {
                "name": "delete_mailbox",
                "description": "Delete a mailbox. REQUIRES user confirmation.",
                "parameters": {
                    "type": "object",
                    "properties": {
                        "hosting_id": { "type": "string" },
                        "domain_id": { "type": "string" },
                        "mailbox_id": { "type": "string" }
                    },
                    "required": ["hosting_id", "domain_id", "mailbox_id"]
                }
            }
        }
    ])
}

// ---------------------------------------------------------------------------
// Description builders for pending mutations (never log passwords)
// ---------------------------------------------------------------------------

/// Builds a human-readable description for a mutating tool call.
///
/// Passwords are never included in the description.
pub fn describe_mutating_action(tool_name: &str, args: &Value) -> String {
    match tool_name {
        "create_dns_record" => {
            let domain = args_str(args, "domain");
            let rtype = args_str(args, "type");
            let name = args_str(args, "name");
            let content = args_str(args, "content");
            let ttl = args_i64(args, "ttl").unwrap_or(0);
            format!("Create {rtype} record {name} on {domain} → {content} (TTL {ttl})")
        }
        "update_dns_record" => {
            let domain = args_str(args, "domain");
            let record_id = args_str(args, "record_id");
            let rtype = args_str(args, "type");
            let name = args_str(args, "name");
            let content = args_str(args, "content");
            format!("Update DNS record #{record_id} on {domain}: {rtype} {name} → {content}")
        }
        "delete_dns_record" => {
            let domain = args_str(args, "domain");
            let record_id = args_str(args, "record_id");
            format!("Delete DNS record #{record_id} on {domain}")
        }
        "create_mailbox" => {
            let email = args_str(args, "email");
            let hosting_id = args_str(args, "hosting_id");
            let domain_id = args_str(args, "domain_id");
            format!("Create mailbox {email} (hosting {hosting_id}, domain {domain_id})")
        }
        "update_mailbox" => {
            let mailbox_id = args_str(args, "mailbox_id");
            let hosting_id = args_str(args, "hosting_id");
            let domain_id = args_str(args, "domain_id");
            let password_set = args
                .get("password")
                .and_then(|v| v.as_str())
                .is_some_and(|p| !p.is_empty());
            if password_set {
                format!(
                    "Update mailbox #{mailbox_id} (hosting {hosting_id}, domain {domain_id}, password change requested)"
                )
            } else {
                format!("Update mailbox #{mailbox_id} (hosting {hosting_id}, domain {domain_id})")
            }
        }
        "delete_mailbox" => {
            let mailbox_id = args_str(args, "mailbox_id");
            let hosting_id = args_str(args, "hosting_id");
            let domain_id = args_str(args, "domain_id");
            format!("Delete mailbox #{mailbox_id} (hosting {hosting_id}, domain {domain_id})")
        }
        other => format!("Pending action: {other}"),
    }
}

fn args_str(args: &Value, key: &str) -> String {
    args.get(key)
        .and_then(|v| v.as_str())
        .unwrap_or("?")
        .to_string()
}

fn args_i64(args: &Value, key: &str) -> Option<i64> {
    args.get(key).and_then(|v| v.as_i64())
}

// ---------------------------------------------------------------------------
// Tool execution
// ---------------------------------------------------------------------------

async fn execute_read_only_tool(name: &str, args: &Value) -> Result<String, String> {
    match name {
        "list_dns_records" => {
            let domain = require_str(args, "domain")?;
            let records = websupport_list_dns_records(domain).await?;
            Ok(serde_json::to_string_pretty(&records).unwrap_or_else(|_| "[]".into()))
        }
        "get_dns_zone" => {
            let domain = require_str(args, "domain")?;
            let zone = websupport_get_dns_zone(domain).await?;
            Ok(serde_json::to_string_pretty(&zone).unwrap_or_else(|_| "{}".into()))
        }
        "test_connection" => {
            let ok = websupport_test_connection().await?;
            Ok(format!(r#"{{"verified":{ok}}}"#))
        }
        "list_hostings" => {
            let items = websupport_list_hostings().await?;
            Ok(serde_json::to_string_pretty(&items).unwrap_or_else(|_| "[]".into()))
        }
        "list_domains" => {
            let hosting_id = require_str(args, "hosting_id")?;
            let items = websupport_list_domains(hosting_id).await?;
            Ok(serde_json::to_string_pretty(&items).unwrap_or_else(|_| "[]".into()))
        }
        "list_mailboxes" => {
            let hosting_id = require_str(args, "hosting_id")?;
            let items = websupport_list_mailboxes(hosting_id).await?;
            Ok(serde_json::to_string_pretty(&items).unwrap_or_else(|_| "[]".into()))
        }
        other => Err(format!("Tool '{other}' is not a read-only tool")),
    }
}

async fn execute_mutating_tool(name: &str, args: &Value) -> Result<String, String> {
    // Safety: only called from mistral_confirm_action after classify_tool == Mutating
    match name {
        "create_dns_record" => {
            let domain = require_str(args, "domain")?;
            let record = parse_dns_record_input(args)?;
            websupport_create_dns_record(domain, record).await?;
            Ok("DNS record created successfully".into())
        }
        "update_dns_record" => {
            let domain = require_str(args, "domain")?;
            let record_id = require_str(args, "record_id")?;
            let record = parse_dns_record_input(args)?;
            websupport_update_dns_record(domain, record_id, record).await?;
            Ok("DNS record updated successfully".into())
        }
        "delete_dns_record" => {
            let domain = require_str(args, "domain")?;
            let record_id = require_str(args, "record_id")?;
            websupport_delete_dns_record(domain, record_id).await?;
            Ok("DNS record deleted successfully".into())
        }
        "create_mailbox" => {
            let hosting_id = require_str(args, "hosting_id")?;
            let domain_id = require_str(args, "domain_id")?;
            let input = parse_create_mailbox_input(args)?;
            // Log only email — never password
            log::info!("Confirming create_mailbox for email={}", input.email);
            websupport_create_mailbox(hosting_id, domain_id, input).await?;
            Ok("Mailbox created successfully".into())
        }
        "update_mailbox" => {
            let hosting_id = require_str(args, "hosting_id")?;
            let domain_id = require_str(args, "domain_id")?;
            let mailbox_id = require_str(args, "mailbox_id")?;
            let input = parse_update_mailbox_input(args)?;
            log::info!("Confirming update_mailbox id={mailbox_id}");
            websupport_update_mailbox(hosting_id, domain_id, mailbox_id, input).await?;
            Ok("Mailbox updated successfully".into())
        }
        "delete_mailbox" => {
            let hosting_id = require_str(args, "hosting_id")?;
            let domain_id = require_str(args, "domain_id")?;
            let mailbox_id = require_str(args, "mailbox_id")?;
            websupport_delete_mailbox(hosting_id, domain_id, mailbox_id).await?;
            Ok("Mailbox deleted successfully".into())
        }
        other => Err(format!("Tool '{other}' is not a mutating tool")),
    }
}

fn require_str(args: &Value, key: &str) -> Result<String, String> {
    args.get(key)
        .and_then(|v| v.as_str())
        .map(|s| s.to_string())
        .filter(|s| !s.is_empty())
        .ok_or_else(|| format!("Missing or empty required argument '{key}'"))
}

fn parse_dns_record_input(args: &Value) -> Result<CreateDnsRecordInput, String> {
    Ok(CreateDnsRecordInput {
        record_type: require_str(args, "type")?,
        name: require_str(args, "name")?,
        content: args
            .get("content")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string()),
        ttl: args
            .get("ttl")
            .and_then(|v| v.as_i64())
            .ok_or_else(|| "Missing required argument 'ttl'".to_string())? as i32,
        priority: args
            .get("priority")
            .and_then(|v| v.as_i64())
            .map(|n| n as i32),
        port: args.get("port").and_then(|v| v.as_i64()).map(|n| n as i32),
        weight: args
            .get("weight")
            .and_then(|v| v.as_i64())
            .map(|n| n as i32),
    })
}

fn parse_create_mailbox_input(args: &Value) -> Result<CreateMailboxInput, String> {
    Ok(CreateMailboxInput {
        email: require_str(args, "email")?,
        password: require_str(args, "password")?,
        ip_check: args.get("ipCheck").and_then(|v| v.as_bool()),
        ips: args
            .get("ips")
            .and_then(|v| serde_json::from_value(v.clone()).ok()),
        country_check: args.get("countryCheck").and_then(|v| v.as_bool()),
        countries: args
            .get("countries")
            .and_then(|v| serde_json::from_value(v.clone()).ok()),
        imap_disabled: args.get("imapDisabled").and_then(|v| v.as_bool()),
        pop3_disabled: args.get("pop3Disabled").and_then(|v| v.as_bool()),
        note: args
            .get("note")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string()),
    })
}

fn parse_update_mailbox_input(args: &Value) -> Result<UpdateMailboxInput, String> {
    Ok(UpdateMailboxInput {
        password: args
            .get("password")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string()),
        ip_check: args.get("ipCheck").and_then(|v| v.as_bool()),
        ips: args
            .get("ips")
            .and_then(|v| serde_json::from_value(v.clone()).ok()),
        country_check: args.get("countryCheck").and_then(|v| v.as_bool()),
        countries: args
            .get("countries")
            .and_then(|v| serde_json::from_value(v.clone()).ok()),
        imap_disabled: args.get("imapDisabled").and_then(|v| v.as_bool()),
        pop3_disabled: args.get("pop3Disabled").and_then(|v| v.as_bool()),
        note: args
            .get("note")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string()),
    })
}

// ---------------------------------------------------------------------------
// Mistral HTTP (injectable for tests)
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Deserialize)]
struct ModelToolCall {
    id: String,
    function: ModelFunctionCall,
}

#[derive(Debug, Clone, Deserialize)]
struct ModelFunctionCall {
    name: String,
    /// JSON object encoded as a string (OpenAI/Mistral convention).
    arguments: String,
}

#[derive(Debug, Clone, Deserialize)]
struct ModelMessage {
    #[allow(dead_code)]
    role: String,
    #[serde(default)]
    content: Option<String>,
    #[serde(default)]
    tool_calls: Option<Vec<ModelToolCall>>,
}

#[derive(Debug, Clone, Deserialize)]
struct ModelChoice {
    message: ModelMessage,
}

#[derive(Debug, Clone, Deserialize)]
pub(crate) struct ModelResponse {
    choices: Vec<ModelChoice>,
}

/// Messages sent to / received from the Mistral chat completions API.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub(crate) enum ApiMessage {
    Simple {
        role: String,
        content: String,
    },
    AssistantToolCalls {
        role: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        content: Option<String>,
        tool_calls: Vec<Value>,
    },
    ToolResult {
        role: String,
        name: String,
        content: String,
        tool_call_id: String,
    },
}

async fn load_mistral_api_key() -> Result<String, String> {
    get_secret(MISTRAL_API_KEY_SECRET.to_string())
        .await?
        .filter(|s| !s.is_empty())
        .ok_or_else(|| {
            "Mistral API key is not configured. Set it in Preferences → Integrations.".to_string()
        })
}

/// Real HTTP call to Mistral chat completions.
async fn call_mistral_http(
    api_key: &str,
    messages: &[ApiMessage],
) -> Result<ModelResponse, String> {
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(60))
        .build()
        .map_err(|e| format!("Failed to create HTTP client: {e}"))?;

    let body = json!({
        "model": MISTRAL_MODEL,
        "messages": messages,
        "tools": tool_definitions(),
        "tool_choice": "auto",
    });

    let response = client
        .post(MISTRAL_API_URL)
        .bearer_auth(api_key)
        .header("Content-Type", "application/json")
        .header("Accept", "application/json")
        .json(&body)
        .send()
        .await
        .map_err(|e| format!("Mistral network error: {e}"))?;

    let status = response.status();
    if !status.is_success() {
        let text = response.text().await.unwrap_or_default();
        return Err(format!("Mistral API error: HTTP {status} — {text}"));
    }

    response
        .json::<ModelResponse>()
        .await
        .map_err(|e| format!("Failed to parse Mistral response: {e}"))
}

// ---------------------------------------------------------------------------
// Orchestration loop (injectable completion for tests)
// ---------------------------------------------------------------------------

/// Runs the tool-calling loop with an injectable completion function.
///
/// Used by [`mistral_send_message`] (real HTTP) and unit tests (mock).
pub async fn orchestrate_turn<F, Fut>(
    conversation: Vec<ChatMessage>,
    mut complete: F,
) -> Result<MistralTurnResult, String>
where
    F: FnMut(Vec<ApiMessage>) -> Fut,
    Fut: std::future::Future<Output = Result<ModelResponse, String>>,
{
    let mut api_messages: Vec<ApiMessage> = conversation
        .into_iter()
        .map(|m| ApiMessage::Simple {
            role: m.role,
            content: m.content,
        })
        .collect();

    // System preamble once at the start if the first message isn't system.
    if !matches!(
        api_messages.first(),
        Some(ApiMessage::Simple { role, .. }) if role == "system"
    ) {
        api_messages.insert(
            0,
            ApiMessage::Simple {
                role: "system".into(),
                content: SYSTEM_PROMPT.into(),
            },
        );
    }

    let mut pending_actions: Vec<PendingAction> = Vec::new();
    let mut last_text = String::new();

    for iteration in 0..MAX_TOOL_LOOP_ITERATIONS {
        log::debug!("Mistral orchestration iteration {iteration}");
        let response = complete(api_messages.clone()).await?;
        let choice = response
            .choices
            .first()
            .ok_or_else(|| "Mistral returned no choices".to_string())?;
        let message = &choice.message;

        if let Some(text) = message.content.as_ref().filter(|s| !s.is_empty()) {
            last_text = text.clone();
        }

        let tool_calls = message.tool_calls.clone().unwrap_or_default();
        if tool_calls.is_empty() {
            return Ok(MistralTurnResult {
                reply: last_text,
                pending_actions,
            });
        }

        // Build assistant message with tool_calls for the conversation history.
        let tool_calls_json: Vec<Value> = tool_calls
            .iter()
            .map(|tc| {
                json!({
                    "id": tc.id,
                    "type": "function",
                    "function": {
                        "name": tc.function.name,
                        "arguments": tc.function.arguments,
                    }
                })
            })
            .collect();

        api_messages.push(ApiMessage::AssistantToolCalls {
            role: "assistant".into(),
            content: message.content.clone(),
            tool_calls: tool_calls_json,
        });

        let mut executed_any_read_only = false;

        for tc in &tool_calls {
            let name = tc.function.name.as_str();
            let args: Value = serde_json::from_str(&tc.function.arguments).unwrap_or(json!({}));

            match classify_tool(name) {
                Err(e) => {
                    // Reject unknown tools — do not execute.
                    log::warn!("Rejected unknown tool call: {name}");
                    api_messages.push(ApiMessage::ToolResult {
                        role: "tool".into(),
                        name: name.into(),
                        content: format!(r#"{{"error":"{e}"}}"#),
                        tool_call_id: tc.id.clone(),
                    });
                    executed_any_read_only = true; // continue loop so model can recover
                }
                Ok(ToolKind::ReadOnly) => {
                    log::info!("Executing read-only tool: {name}");
                    let content = match execute_read_only_tool(name, &args).await {
                        Ok(s) => s,
                        Err(e) => format!(r#"{{"error":{}}}"#, json!(e)),
                    };
                    api_messages.push(ApiMessage::ToolResult {
                        role: "tool".into(),
                        name: name.into(),
                        content,
                        tool_call_id: tc.id.clone(),
                    });
                    executed_any_read_only = true;
                }
                Ok(ToolKind::Mutating) => {
                    // CRITICAL: never execute — queue for UI confirmation.
                    log::info!("Queuing mutating tool for confirmation: {name}");
                    let description = describe_mutating_action(name, &args);
                    pending_actions.push(PendingAction {
                        id: Uuid::new_v4().to_string(),
                        tool_name: name.to_string(),
                        description: description.clone(),
                        args,
                    });
                    // Inform the model that confirmation is required (no execution).
                    api_messages.push(ApiMessage::ToolResult {
                        role: "tool".into(),
                        name: name.into(),
                        content: format!(
                            r#"{{"status":"pending_confirmation","message":"Action queued for user confirmation: {description}. Do not claim it was executed."}}"#
                        ),
                        tool_call_id: tc.id.clone(),
                    });
                    // Count as progress so we can let the model produce a final reply.
                    executed_any_read_only = true;
                }
            }
        }

        if !executed_any_read_only {
            // Should not happen; safety break.
            break;
        }
    }

    if last_text.is_empty() {
        last_text = "I reached the maximum number of tool steps. Please confirm any pending actions or try a simpler request.".into();
    } else {
        last_text.push_str(
            "\n\n(Note: tool-calling loop reached the safety limit; some steps may be incomplete.)",
        );
    }

    Ok(MistralTurnResult {
        reply: last_text,
        pending_actions,
    })
}

const SYSTEM_PROMPT: &str = "\
You are an assistant that manages Websupport DNS records and mailboxes via tools. \
Use tools to read current state before proposing changes. \
Mutating tools (create/update/delete) will NOT execute immediately — they require the user to confirm in the UI. \
Never invent record ids or hosting ids; always list first when unsure. \
Never include passwords in your natural-language replies.";

// ---------------------------------------------------------------------------
// Tauri commands
// ---------------------------------------------------------------------------

/// Sends the conversation to Mistral with Websupport tools.
///
/// Mutating tool calls are returned as [`PendingAction`]s — they are never
/// executed in this command.
#[tauri::command]
#[specta::specta]
pub async fn mistral_send_message(
    conversation: Vec<ChatMessage>,
) -> Result<MistralTurnResult, String> {
    if conversation.is_empty() {
        return Err("Conversation must not be empty".into());
    }

    let api_key = load_mistral_api_key().await?;
    log::info!("Mistral turn starting ({} message(s))", conversation.len());

    orchestrate_turn(conversation, |messages| {
        let key = api_key.clone();
        async move { call_mistral_http(&key, &messages).await }
    })
    .await
}

/// Executes a previously returned pending mutating action after user confirmation.
///
/// Rejects tool names that are not in the mutating set (including read-only
/// and unknown names).
#[tauri::command]
#[specta::specta]
pub async fn mistral_confirm_action(action: PendingAction) -> Result<String, String> {
    // Fail-closed: only mutating tools may be confirmed.
    if !is_mutating_tool(&action.tool_name)? {
        return Err(format!(
            "Cannot confirm read-only tool '{}'. Only mutating tools require confirmation.",
            action.tool_name
        ));
    }

    log::info!(
        "Confirming pending action id={} tool={}",
        action.id,
        action.tool_name
    );
    // Description is already redacted; never log args (may contain passwords).
    log::info!("Action description: {}", action.description);

    execute_mutating_tool(&action.tool_name, &action.args).await
}

// ---------------------------------------------------------------------------
// Unit tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::{Arc, Mutex};

    #[test]
    fn is_mutating_tool_true_for_all_mutating_names() {
        for name in MUTATING_TOOL_NAMES {
            assert!(
                is_mutating_tool(name).expect("known tool"),
                "{name} should be mutating"
            );
        }
    }

    #[test]
    fn is_mutating_tool_false_for_all_read_only_names() {
        for name in READ_ONLY_TOOL_NAMES {
            assert!(
                !is_mutating_tool(name).expect("known tool"),
                "{name} should be read-only"
            );
        }
    }

    #[test]
    fn is_mutating_tool_rejects_unknown_names() {
        let err = is_mutating_tool("rm_rf_root").expect_err("unknown must err");
        assert!(err.contains("Unknown tool"), "got: {err}");
        assert!(is_mutating_tool("").is_err());
        assert!(is_mutating_tool("list_dns_record").is_err()); // singular typo
    }

    #[test]
    fn classify_tool_covers_all_defined_tools() {
        assert_eq!(
            ALL_TOOL_NAMES.len(),
            MUTATING_TOOL_NAMES.len() + READ_ONLY_TOOL_NAMES.len()
        );
        for name in ALL_TOOL_NAMES {
            assert!(classify_tool(name).is_ok(), "unclassified: {name}");
        }
    }

    #[test]
    fn describe_create_dns_record() {
        let args = json!({
            "domain": "example.com",
            "type": "A",
            "name": "api",
            "content": "1.2.3.4",
            "ttl": 300
        });
        let desc = describe_mutating_action("create_dns_record", &args);
        assert!(desc.contains("A"));
        assert!(desc.contains("api"));
        assert!(desc.contains("1.2.3.4"));
        assert!(desc.contains("300"));
    }

    #[test]
    fn describe_create_mailbox_never_includes_password() {
        let args = json!({
            "hosting_id": "1",
            "domain_id": "2",
            "email": "user@example.com",
            "password": "super-secret-password-xyz"
        });
        let desc = describe_mutating_action("create_mailbox", &args);
        assert!(desc.contains("user@example.com"));
        assert!(!desc.contains("super-secret-password-xyz"));
        assert!(!desc.to_lowercase().contains("super-secret"));
    }

    #[tokio::test]
    async fn mistral_confirm_action_rejects_read_only_tool() {
        let action = PendingAction {
            id: "test-id".into(),
            tool_name: "list_dns_records".into(),
            description: "should not run".into(),
            args: json!({ "domain": "example.com" }),
        };
        let err = mistral_confirm_action(action)
            .await
            .expect_err("read-only confirm must fail");
        assert!(
            err.contains("Cannot confirm read-only") || err.contains("read-only"),
            "got: {err}"
        );
    }

    #[tokio::test]
    async fn mistral_confirm_action_rejects_unknown_tool() {
        let action = PendingAction {
            id: "test-id".into(),
            tool_name: "drop_database".into(),
            description: "evil".into(),
            args: json!({}),
        };
        let err = mistral_confirm_action(action)
            .await
            .expect_err("unknown confirm must fail");
        assert!(err.contains("Unknown tool"), "got: {err}");
    }

    #[tokio::test]
    async fn orchestrate_turn_queues_mutating_without_executing() {
        // Mock model: first response requests create_dns_record, second returns text.
        let call_count = Arc::new(Mutex::new(0u32));
        let call_count_clone = call_count.clone();

        let result = orchestrate_turn(
            vec![ChatMessage {
                role: "user".into(),
                content: "Add an A record".into(),
            }],
            move |_messages| {
                let count = {
                    let mut c = call_count_clone.lock().unwrap();
                    *c += 1;
                    *c
                };
                async move {
                    if count == 1 {
                        Ok(ModelResponse {
                            choices: vec![ModelChoice {
                                message: ModelMessage {
                                    role: "assistant".into(),
                                    content: Some(
                                        "I'll queue creating that record for your confirmation."
                                            .into(),
                                    ),
                                    tool_calls: Some(vec![ModelToolCall {
                                        id: "call_1".into(),
                                        function: ModelFunctionCall {
                                            name: "create_dns_record".into(),
                                            arguments: json!({
                                                "domain": "example.com",
                                                "type": "A",
                                                "name": "api",
                                                "content": "1.2.3.4",
                                                "ttl": 300
                                            })
                                            .to_string(),
                                        },
                                    }]),
                                },
                            }],
                        })
                    } else {
                        Ok(ModelResponse {
                            choices: vec![ModelChoice {
                                message: ModelMessage {
                                    role: "assistant".into(),
                                    content: Some(
                                        "Please confirm the pending DNS create action.".into(),
                                    ),
                                    tool_calls: None,
                                },
                            }],
                        })
                    }
                }
            },
        )
        .await
        .expect("orchestrate should succeed");

        assert_eq!(result.pending_actions.len(), 1);
        assert_eq!(result.pending_actions[0].tool_name, "create_dns_record");
        assert!(result.pending_actions[0].description.contains("1.2.3.4"));
        assert!(result.reply.contains("confirm") || result.reply.contains("Confirm"));
        assert!(*call_count.lock().unwrap() >= 1);
    }

    #[tokio::test]
    async fn orchestrate_turn_plain_text_no_tools() {
        let result = orchestrate_turn(
            vec![ChatMessage {
                role: "user".into(),
                content: "Hello".into(),
            }],
            |_messages| async move {
                Ok(ModelResponse {
                    choices: vec![ModelChoice {
                        message: ModelMessage {
                            role: "assistant".into(),
                            content: Some("Hi! How can I help with Websupport?".into()),
                            tool_calls: None,
                        },
                    }],
                })
            },
        )
        .await
        .expect("ok");

        assert!(result.pending_actions.is_empty());
        assert!(result.reply.contains("Websupport"));
    }

    #[test]
    fn tool_definitions_include_all_tools() {
        let tools = tool_definitions();
        let arr = tools.as_array().expect("array");
        assert_eq!(arr.len(), ALL_TOOL_NAMES.len());
        let names: Vec<&str> = arr
            .iter()
            .filter_map(|t| t.pointer("/function/name")?.as_str())
            .collect();
        for expected in ALL_TOOL_NAMES {
            assert!(names.contains(expected), "missing tool def: {expected}");
        }
    }
}
