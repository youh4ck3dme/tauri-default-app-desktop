//! Shared Websupport REST authentication and HTTP helpers.
//!
//! Used by both API v1 (`Date` header) and v2 (`X-Date` header).
//! Credentials come from the OS keychain via
//! [`crate::commands::secrets::get_secret`].

use hmac::{Hmac, Mac};
use serde::Serialize;
use serde_json::Value;
use sha1::Sha1;
use std::time::{SystemTime, UNIX_EPOCH};

use crate::commands::secrets::get_secret;

/// Base URL for all Websupport REST API versions.
pub const BASE_URL: &str = "https://rest.websupport.sk";

const IDENTIFIER_KEY: &str = "websupport_identifier";
const SECRET_KEY: &str = "websupport_secret";

type HmacSha1 = Hmac<Sha1>;

/// Which date header the API version expects.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DateHeader {
    /// API v2 — `X-Date: YYYYMMDDTHHMMSSZ`
    XDate,
    /// API v1 — `Date: YYYYMMDDTHHMMSSZ`
    Date,
}

impl DateHeader {
    pub fn header_name(self) -> &'static str {
        match self {
            DateHeader::XDate => "X-Date",
            DateHeader::Date => "Date",
        }
    }
}

// ---------------------------------------------------------------------------
// Pure signing helpers (unit-tested)
// ---------------------------------------------------------------------------

/// Builds the canonical request string used for HMAC signing.
///
/// Format: `{METHOD} {path_with_query} {unix_timestamp}`
pub fn build_canonical_request(method: &str, path_with_query: &str, unix_timestamp: u64) -> String {
    format!("{method} {path_with_query} {unix_timestamp}")
}

/// Hex-encoded HMAC-SHA1 of `canonical` keyed with `secret`.
pub fn sign_canonical_request(secret: &str, canonical: &str) -> String {
    let mut mac =
        HmacSha1::new_from_slice(secret.as_bytes()).expect("HMAC-SHA1 accepts any key length");
    mac.update(canonical.as_bytes());
    hex::encode(mac.finalize().into_bytes())
}

/// Formats a unix timestamp as ISO8601 basic GMT: `YYYYMMDDTHHMMSSZ` (pure std).
///
/// Uses the civil-from-days algorithm (Howard Hinnant).
pub fn format_date_gmt(unix_timestamp: u64) -> String {
    let z = (unix_timestamp / 86_400) as i64 + 719_468;
    let era = if z >= 0 { z } else { z - 146_096 } / 146_097;
    let doe = (z - era * 146_097) as u64;
    let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146_096) / 365;
    let y = yoe as i64 + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let y = if m <= 2 { y + 1 } else { y };

    let secs_of_day = unix_timestamp % 86_400;
    let hour = secs_of_day / 3600;
    let minute = (secs_of_day % 3600) / 60;
    let second = secs_of_day % 60;

    format!("{y:04}{m:02}{d:02}T{hour:02}{minute:02}{second:02}Z")
}

/// Builds signed auth values for a request (pure, no I/O).
///
/// Returns `(signature_hex, date_header_value)`.
pub fn sign_request(
    secret: &str,
    method: &str,
    path_with_query: &str,
    unix_timestamp: u64,
) -> (String, String) {
    let canonical = build_canonical_request(method, path_with_query, unix_timestamp);
    let signature = sign_canonical_request(secret, &canonical);
    let date_value = format_date_gmt(unix_timestamp);
    (signature, date_value)
}

// ---------------------------------------------------------------------------
// Credentials + HTTP
// ---------------------------------------------------------------------------

pub struct Credentials {
    pub identifier: String,
    pub secret: String,
}

pub async fn load_credentials() -> Result<Credentials, String> {
    let identifier = get_secret(IDENTIFIER_KEY.to_string())
        .await?
        .filter(|s| !s.is_empty())
        .ok_or_else(|| {
            "Websupport identifier is not configured. Set it in Preferences → Integrations."
                .to_string()
        })?;

    let secret = get_secret(SECRET_KEY.to_string())
        .await?
        .filter(|s| !s.is_empty())
        .ok_or_else(|| {
            "Websupport secret is not configured. Set it in Preferences → Integrations.".to_string()
        })?;

    Ok(Credentials { identifier, secret })
}

pub fn now_unix() -> Result<u64, String> {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .map_err(|e| format!("System clock error: {e}"))
}

fn http_client() -> Result<reqwest::Client, String> {
    reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(30))
        .build()
        .map_err(|e| format!("Failed to create HTTP client: {e}"))
}

/// Sends an authenticated Websupport request.
///
/// `date_header` selects `X-Date` (v2) or `Date` (v1).
pub async fn send_request(
    method: reqwest::Method,
    path_with_query: &str,
    body: Option<&impl Serialize>,
    date_header: DateHeader,
) -> Result<reqwest::Response, String> {
    let creds = load_credentials().await?;
    let timestamp = now_unix()?;
    let (signature, date_value) =
        sign_request(&creds.secret, method.as_str(), path_with_query, timestamp);

    let url = format!("{BASE_URL}{path_with_query}");
    let client = http_client()?;

    let mut request = client
        .request(method, &url)
        .header("Content-Type", "application/json")
        .header("Accept", "application/json")
        .header(date_header.header_name(), date_value)
        .basic_auth(&creds.identifier, Some(&signature));

    if let Some(body) = body {
        request = request.json(body);
    }

    request
        .send()
        .await
        .map_err(|e| format!("Network error: {e}"))
}

pub async fn error_from_response(response: reqwest::Response) -> String {
    let status = response.status();
    let body = response.text().await.unwrap_or_default();
    if body.is_empty() {
        format!("Websupport API error: HTTP {status}")
    } else if let Ok(value) = serde_json::from_str::<Value>(&body) {
        if let Some(title) = value.get("title").and_then(|v| v.as_str()) {
            format!("Websupport API error: HTTP {status} — {title}")
        } else if let Some(message) = value.get("message").and_then(|v| v.as_str()) {
            format!("Websupport API error: HTTP {status} — {message}")
        } else if let Some(message) = value.get("error").and_then(|v| v.as_str()) {
            format!("Websupport API error: HTTP {status} — {message}")
        } else {
            format!("Websupport API error: HTTP {status} — {body}")
        }
    } else {
        format!("Websupport API error: HTTP {status} — {body}")
    }
}

/// Validates a path segment id (hosting, domain, mailbox, record, domain name).
pub fn validate_path_segment(value: &str, label: &str) -> Result<(), String> {
    let value = value.trim();
    if value.is_empty() {
        return Err(format!("{label} must not be empty"));
    }
    if value.contains('/') || value.contains('?') || value.contains('#') || value.contains(' ') {
        return Err(format!("{label} contains invalid characters"));
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// Unit tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    /// RFC 2202 test case 2 — independent known vector for HMAC-SHA1.
    #[test]
    fn sign_canonical_request_matches_rfc2202_vector() {
        let signature = sign_canonical_request("Jefe", "what do ya want for nothing?");
        assert_eq!(signature, "effcdf6ae5eb2fa2d27416d5f184df9c259a7c79");
    }

    /// Websupport-shaped canonical string with a dummy secret (not a real credential).
    #[test]
    fn sign_canonical_request_websupport_shaped_vector() {
        let canonical = build_canonical_request("GET", "/v2/check", 1_548_240_417);
        assert_eq!(canonical, "GET /v2/check 1548240417");
        let signature = sign_canonical_request("test-secret", &canonical);
        assert_eq!(signature, "c2a007be93d6ffbe0c610a926c9bda2de55c00a6");
    }

    #[test]
    fn build_canonical_request_includes_query_string() {
        let canonical = build_canonical_request("GET", "/v1/user/1/hosting?page=1", 1_548_240_417);
        assert_eq!(canonical, "GET /v1/user/1/hosting?page=1 1548240417");
    }

    #[test]
    fn sign_request_returns_signature_and_gmt_date() {
        let (signature, date) = sign_request("test-secret", "GET", "/v1/user/self", 1_548_240_417);
        assert_eq!(
            signature,
            sign_canonical_request("test-secret", "GET /v1/user/self 1548240417",)
        );
        assert_eq!(date, "20190123T104657Z");
    }

    #[test]
    fn date_header_variants_differ_by_name_only() {
        assert_eq!(DateHeader::XDate.header_name(), "X-Date");
        assert_eq!(DateHeader::Date.header_name(), "Date");
        // Same timestamp produces the same date *value* for both API versions.
        let ts = 1_784_894_400_u64; // 2026-07-24T12:00:00Z
        let value = format_date_gmt(ts);
        assert_eq!(value, "20260724T120000Z");
        // Header names are what differ between v1 and v2:
        assert_ne!(
            DateHeader::XDate.header_name(),
            DateHeader::Date.header_name()
        );
    }

    #[test]
    fn format_date_gmt_epoch() {
        assert_eq!(format_date_gmt(0), "19700101T000000Z");
    }

    #[test]
    fn sign_is_deterministic() {
        let a = sign_canonical_request("secret", "GET /v1/user/self 100");
        let b = sign_canonical_request("secret", "GET /v1/user/self 100");
        assert_eq!(a, b);
    }

    #[test]
    fn validate_path_segment_rejects_injection() {
        assert!(validate_path_segment("", "id").is_err());
        assert!(validate_path_segment("a/b", "id").is_err());
        assert!(validate_path_segment("ok-id", "id").is_ok());
        assert!(validate_path_segment("example.com", "domain").is_ok());
    }
}
