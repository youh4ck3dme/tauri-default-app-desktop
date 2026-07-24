//! Websupport.sk REST API v1 — hostings, domains (vhosts), and mailboxes.
//!
//! Credentials and HMAC signing live in [`crate::websupport_auth`]
//! (v1 uses the `Date` header instead of `X-Date`).
//!
//! User id is resolved once per process via `GET /v1/user/self` and cached.

use serde::{Deserialize, Serialize};
use specta::Type;
use std::sync::Mutex;

use crate::websupport_auth::{
    error_from_response, send_request, validate_path_segment, DateHeader,
};

/// Process-lifetime cache of the numeric user id from `/v1/user/self`.
static CACHED_USER_ID: Mutex<Option<i32>> = Mutex::new(None);

// ---------------------------------------------------------------------------
// Public types (tauri-specta)
// ---------------------------------------------------------------------------

/// A hosting product under the authenticated user.
#[derive(Debug, Clone, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct Hosting {
    pub id: i32,
    pub name: String,
    #[serde(default)]
    pub uuid: Option<String>,
}

/// A virtual host (domain) under a hosting.
#[derive(Debug, Clone, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct VHost {
    pub id: i32,
    /// Domain name (API may expose as `name` or `domain`).
    #[serde(alias = "domain")]
    pub name: String,
}

/// A mailbox under a hosting.
#[derive(Debug, Clone, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct Mailbox {
    pub id: i32,
    /// Full email address or local part (API-dependent).
    #[serde(default, alias = "login")]
    pub email: Option<String>,
    #[serde(default)]
    pub domain: Option<String>,
    #[serde(default)]
    pub note: Option<String>,
    #[serde(default)]
    pub ip_check: Option<bool>,
    #[serde(default)]
    pub country_check: Option<bool>,
    #[serde(default)]
    pub imap_disabled: Option<bool>,
    #[serde(default)]
    pub pop3_disabled: Option<bool>,
}

/// Input for creating a mailbox. Password is never logged.
///
/// `Debug` is implemented by hand to redact `password` — this is the
/// safety net for any future `{:?}` of the whole struct (panic messages,
/// a stray debug log, etc.), on top of the explicit field-level logging
/// already used in `websupport_create_mailbox`.
#[derive(Clone, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct CreateMailboxInput {
    pub email: String,
    pub password: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ip_check: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ips: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub country_check: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub countries: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub imap_disabled: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pop3_disabled: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub note: Option<String>,
}

impl std::fmt::Debug for CreateMailboxInput {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("CreateMailboxInput")
            .field("email", &self.email)
            .field("password", &"[REDACTED]")
            .field("ip_check", &self.ip_check)
            .field("ips", &self.ips)
            .field("country_check", &self.country_check)
            .field("countries", &self.countries)
            .field("imap_disabled", &self.imap_disabled)
            .field("pop3_disabled", &self.pop3_disabled)
            .field("note", &self.note)
            .finish()
    }
}

/// Input for updating a mailbox (POST, not PUT). Password is never logged.
///
/// `Debug` is implemented by hand to redact `password` (see
/// `CreateMailboxInput`'s impl for the rationale).
#[derive(Clone, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct UpdateMailboxInput {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub password: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ip_check: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ips: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub country_check: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub countries: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub imap_disabled: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pop3_disabled: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub note: Option<String>,
}

impl std::fmt::Debug for UpdateMailboxInput {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let password_redacted = self.password.as_ref().map(|_| "[REDACTED]");
        f.debug_struct("UpdateMailboxInput")
            .field("password", &password_redacted)
            .field("ip_check", &self.ip_check)
            .field("ips", &self.ips)
            .field("country_check", &self.country_check)
            .field("countries", &self.countries)
            .field("imap_disabled", &self.imap_disabled)
            .field("pop3_disabled", &self.pop3_disabled)
            .field("note", &self.note)
            .finish()
    }
}

// ---------------------------------------------------------------------------
// Internal response envelopes
// ---------------------------------------------------------------------------

#[derive(Debug, Deserialize)]
struct UserSelf {
    id: i32,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct ItemsEnvelope<T> {
    items: Vec<T>,
}

/// Vhost list may be a bare array or an `{ items: [...] }` envelope.
#[derive(Debug, Deserialize)]
#[serde(untagged)]
enum VHostListResponse {
    Envelope(ItemsEnvelope<VHost>),
    Bare(Vec<VHost>),
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

async fn resolve_user_id() -> Result<i32, String> {
    {
        let guard = CACHED_USER_ID
            .lock()
            .map_err(|_| "User id cache lock poisoned".to_string())?;
        if let Some(id) = *guard {
            return Ok(id);
        }
    }

    log::info!("Resolving Websupport user id via /v1/user/self");
    let response = send_request(
        reqwest::Method::GET,
        "/v1/user/self",
        None::<&()>,
        DateHeader::Date,
    )
    .await?;

    if !response.status().is_success() {
        return Err(error_from_response(response).await);
    }

    let user: UserSelf = response
        .json()
        .await
        .map_err(|e| format!("Failed to parse /v1/user/self: {e}"))?;

    let mut guard = CACHED_USER_ID
        .lock()
        .map_err(|_| "User id cache lock poisoned".to_string())?;
    *guard = Some(user.id);
    log::info!("Cached Websupport user id={}", user.id);
    Ok(user.id)
}

/// Clears the session user-id cache (test / logout hook).
#[cfg(test)]
fn clear_user_id_cache() {
    if let Ok(mut guard) = CACHED_USER_ID.lock() {
        *guard = None;
    }
}

// ---------------------------------------------------------------------------
// Tauri commands
// ---------------------------------------------------------------------------

/// Lists hostings for the authenticated user (`GET /v1/user/{id}/hosting`).
#[tauri::command]
#[specta::specta]
pub async fn websupport_list_hostings() -> Result<Vec<Hosting>, String> {
    let user_id = resolve_user_id().await?;
    log::info!("Listing Websupport hostings for user {user_id}");

    let path = format!("/v1/user/{user_id}/hosting");
    let response = send_request(reqwest::Method::GET, &path, None::<&()>, DateHeader::Date).await?;

    if !response.status().is_success() {
        return Err(error_from_response(response).await);
    }

    let envelope: ItemsEnvelope<Hosting> = response
        .json()
        .await
        .map_err(|e| format!("Failed to parse hostings response: {e}"))?;

    log::info!("Loaded {} hosting(s)", envelope.items.len());
    Ok(envelope.items)
}

/// Lists domains (vhosts) under a hosting
/// (`GET /v1/user/{id}/hosting/{hostingId}/vhost`).
#[tauri::command]
#[specta::specta]
pub async fn websupport_list_domains(hosting_id: String) -> Result<Vec<VHost>, String> {
    validate_path_segment(&hosting_id, "Hosting id")?;
    let user_id = resolve_user_id().await?;
    log::info!("Listing Websupport domains for hosting {hosting_id}");

    let path = format!("/v1/user/{user_id}/hosting/{hosting_id}/vhost");
    let response = send_request(reqwest::Method::GET, &path, None::<&()>, DateHeader::Date).await?;

    if !response.status().is_success() {
        return Err(error_from_response(response).await);
    }

    let body: VHostListResponse = response
        .json()
        .await
        .map_err(|e| format!("Failed to parse vhosts response: {e}"))?;

    let items = match body {
        VHostListResponse::Envelope(e) => e.items,
        VHostListResponse::Bare(v) => v,
    };

    log::info!("Loaded {} domain(s) for hosting {hosting_id}", items.len());
    Ok(items)
}

/// Lists mailboxes under a hosting
/// (`GET /v1/user/{id}/hosting/{hostingId}/mailbox`).
#[tauri::command]
#[specta::specta]
pub async fn websupport_list_mailboxes(hosting_id: String) -> Result<Vec<Mailbox>, String> {
    validate_path_segment(&hosting_id, "Hosting id")?;
    let user_id = resolve_user_id().await?;
    log::info!("Listing Websupport mailboxes for hosting {hosting_id}");

    let path = format!("/v1/user/{user_id}/hosting/{hosting_id}/mailbox");
    let response = send_request(reqwest::Method::GET, &path, None::<&()>, DateHeader::Date).await?;

    if !response.status().is_success() {
        return Err(error_from_response(response).await);
    }

    let envelope: ItemsEnvelope<Mailbox> = response
        .json()
        .await
        .map_err(|e| format!("Failed to parse mailboxes response: {e}"))?;

    log::info!(
        "Loaded {} mailbox(es) for hosting {hosting_id}",
        envelope.items.len()
    );
    Ok(envelope.items)
}

/// Creates a mailbox
/// (`POST /v1/user/{id}/hosting/{hostingId}/domain/{domainId}/mailbox`).
///
/// Password is never written to logs.
#[tauri::command]
#[specta::specta]
pub async fn websupport_create_mailbox(
    hosting_id: String,
    domain_id: String,
    input: CreateMailboxInput,
) -> Result<(), String> {
    validate_path_segment(&hosting_id, "Hosting id")?;
    validate_path_segment(&domain_id, "Domain id")?;
    if input.email.trim().is_empty() {
        return Err("Email must not be empty".to_string());
    }
    if input.password.is_empty() {
        return Err("Password must not be empty".to_string());
    }

    let user_id = resolve_user_id().await?;
    // Redacted log — never include password
    log::info!(
        "Creating Websupport mailbox email={} on hosting={hosting_id} domain={domain_id}",
        input.email
    );

    let path = format!("/v1/user/{user_id}/hosting/{hosting_id}/domain/{domain_id}/mailbox");
    let response =
        send_request(reqwest::Method::POST, &path, Some(&input), DateHeader::Date).await?;

    let status = response.status();
    if status.as_u16() != 204 && !status.is_success() {
        return Err(error_from_response(response).await);
    }

    log::info!("Created mailbox {} on hosting {hosting_id}", input.email);
    Ok(())
}

/// Updates a mailbox via POST (not PUT)
/// (`POST /v1/user/{id}/hosting/{hostingId}/domain/{domainId}/mailbox/{mailboxId}`).
///
/// Password is never written to logs.
#[tauri::command]
#[specta::specta]
pub async fn websupport_update_mailbox(
    hosting_id: String,
    domain_id: String,
    mailbox_id: String,
    input: UpdateMailboxInput,
) -> Result<(), String> {
    validate_path_segment(&hosting_id, "Hosting id")?;
    validate_path_segment(&domain_id, "Domain id")?;
    validate_path_segment(&mailbox_id, "Mailbox id")?;

    let user_id = resolve_user_id().await?;
    let password_set = input.password.as_ref().is_some_and(|p| !p.is_empty());
    // Redacted log — never include password
    log::info!(
        "Updating Websupport mailbox id={mailbox_id} hosting={hosting_id} domain={domain_id} password_set={password_set}"
    );

    let path =
        format!("/v1/user/{user_id}/hosting/{hosting_id}/domain/{domain_id}/mailbox/{mailbox_id}");
    let response =
        send_request(reqwest::Method::POST, &path, Some(&input), DateHeader::Date).await?;

    let status = response.status();
    if status.as_u16() != 204 && !status.is_success() {
        return Err(error_from_response(response).await);
    }

    log::info!("Updated mailbox {mailbox_id} on hosting {hosting_id}");
    Ok(())
}

/// Deletes a mailbox
/// (`DELETE /v1/user/{id}/hosting/{hostingId}/domain/{domainId}/mailbox/{mailboxId}`).
#[tauri::command]
#[specta::specta]
pub async fn websupport_delete_mailbox(
    hosting_id: String,
    domain_id: String,
    mailbox_id: String,
) -> Result<(), String> {
    validate_path_segment(&hosting_id, "Hosting id")?;
    validate_path_segment(&domain_id, "Domain id")?;
    validate_path_segment(&mailbox_id, "Mailbox id")?;

    let user_id = resolve_user_id().await?;
    log::info!(
        "Deleting Websupport mailbox id={mailbox_id} hosting={hosting_id} domain={domain_id}"
    );

    let path =
        format!("/v1/user/{user_id}/hosting/{hosting_id}/domain/{domain_id}/mailbox/{mailbox_id}");
    let response = send_request(
        reqwest::Method::DELETE,
        &path,
        None::<&()>,
        DateHeader::Date,
    )
    .await?;

    let status = response.status();
    if status.as_u16() != 204 && !status.is_success() {
        return Err(error_from_response(response).await);
    }

    log::info!("Deleted mailbox {mailbox_id} on hosting {hosting_id}");
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn create_mailbox_input_serializes_without_none_fields() {
        let input = CreateMailboxInput {
            email: "user@example.com".into(),
            password: "s3cret".into(),
            ip_check: None,
            ips: None,
            country_check: None,
            countries: None,
            imap_disabled: None,
            pop3_disabled: None,
            note: None,
        };
        let json = serde_json::to_value(&input).expect("serialize");
        assert_eq!(json["email"], "user@example.com");
        assert_eq!(json["password"], "s3cret");
        assert!(json.get("ipCheck").is_none());
        assert!(json.get("note").is_none());
    }

    #[test]
    fn update_mailbox_input_omits_unset_password() {
        let input = UpdateMailboxInput {
            password: None,
            ip_check: Some(true),
            ips: None,
            country_check: None,
            countries: None,
            imap_disabled: Some(false),
            pop3_disabled: None,
            note: Some("note".into()),
        };
        let json = serde_json::to_value(&input).expect("serialize");
        assert!(json.get("password").is_none());
        assert_eq!(json["ipCheck"], true);
        assert_eq!(json["imapDisabled"], false);
        assert_eq!(json["note"], "note");
    }

    #[test]
    fn user_id_cache_starts_empty() {
        clear_user_id_cache();
        let guard = CACHED_USER_ID.lock().unwrap();
        assert!(guard.is_none());
    }

    #[test]
    fn create_mailbox_input_debug_redacts_password() {
        let input = CreateMailboxInput {
            email: "user@example.com".into(),
            password: "super-secret-value".into(),
            ip_check: None,
            ips: None,
            country_check: None,
            countries: None,
            imap_disabled: None,
            pop3_disabled: None,
            note: None,
        };
        let debug_output = format!("{input:?}");
        assert!(!debug_output.contains("super-secret-value"));
        assert!(debug_output.contains("[REDACTED]"));
        assert!(debug_output.contains("user@example.com"));
    }

    #[test]
    fn update_mailbox_input_debug_redacts_password_when_set() {
        let input = UpdateMailboxInput {
            password: Some("another-secret".into()),
            ip_check: None,
            ips: None,
            country_check: None,
            countries: None,
            imap_disabled: None,
            pop3_disabled: None,
            note: None,
        };
        let debug_output = format!("{input:?}");
        assert!(!debug_output.contains("another-secret"));
        assert!(debug_output.contains("[REDACTED]"));
    }

    #[test]
    fn update_mailbox_input_debug_shows_none_when_password_unset() {
        let input = UpdateMailboxInput {
            password: None,
            ip_check: None,
            ips: None,
            country_check: None,
            countries: None,
            imap_disabled: None,
            pop3_disabled: None,
            note: None,
        };
        let debug_output = format!("{input:?}");
        assert!(!debug_output.contains("[REDACTED]"));
    }
}
