//! Websupport.sk REST API v2 — DNS management.
//!
//! Credentials and HMAC signing live in [`crate::websupport_auth`]
//! (v2 uses the `X-Date` header). See https://rest.websupport.sk/v2/docs/intro

use serde::{Deserialize, Serialize};
use specta::Type;

use crate::websupport_auth::{
    error_from_response, send_request, validate_path_segment, DateHeader,
};

const DEFAULT_ROWS_PER_PAGE: u32 = 100;

// ---------------------------------------------------------------------------
// Public types (tauri-specta)
// ---------------------------------------------------------------------------

/// A DNS record returned by the Websupport API.
///
/// Numeric fields use `i32` (fits DNS TTL/priority/port/weight and TS `number`).
#[derive(Debug, Clone, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct DnsRecord {
    /// Record id (null for some synthetic entries).
    pub id: Option<i32>,
    pub name: String,
    pub content: String,
    pub ttl: i32,
    pub priority: Option<i32>,
    pub port: Option<i32>,
    pub weight: Option<i32>,
    /// Record type: A, AAAA, ANAME, CAA, CNAME, DNSSEC, MX, NS, SRV, TXT, …
    #[serde(rename = "type")]
    pub record_type: String,
}

/// Input for creating or updating a DNS record.
#[derive(Debug, Clone, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct CreateDnsRecordInput {
    /// Record type: A, AAAA, ANAME, CAA, CNAME, DNSSEC, MX, NS, SRV, TXT, CERT, LOC, SSHFP, TLSA, DS.
    #[serde(rename = "type")]
    pub record_type: String,
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub content: Option<String>,
    pub ttl: i32,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub priority: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub port: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub weight: Option<i32>,
}

/// DNS zone metadata for a domain.
#[derive(Debug, Clone, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct DnsZone {
    pub name: String,
    pub last_check: Option<String>,
    pub dnssec_signing: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct VerifyResponse {
    verified: bool,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct DnsRecordPage {
    current_page: u32,
    total_pages: u32,
    data: Vec<DnsRecord>,
}

// ---------------------------------------------------------------------------
// Tauri commands
// ---------------------------------------------------------------------------

/// Tests Websupport API credentials via `GET /v2/check`.
///
/// Returns `true` when the API reports `{ "verified": true }`.
#[tauri::command]
#[specta::specta]
pub async fn websupport_test_connection() -> Result<bool, String> {
    log::info!("Testing Websupport API connection");
    let response = send_request(
        reqwest::Method::GET,
        "/v2/check",
        None::<&()>,
        DateHeader::XDate,
    )
    .await?;

    if !response.status().is_success() {
        return Err(error_from_response(response).await);
    }

    let body: VerifyResponse = response
        .json()
        .await
        .map_err(|e| format!("Failed to parse /v2/check response: {e}"))?;

    log::info!("Websupport connection verified={}", body.verified);
    Ok(body.verified)
}

/// Fetches DNS zone metadata for `domain` via `GET /v2/service/{domain}/dns/zone`.
#[tauri::command]
#[specta::specta]
pub async fn websupport_get_dns_zone(domain: String) -> Result<DnsZone, String> {
    validate_path_segment(&domain, "Domain")?;
    log::info!("Fetching Websupport DNS zone for {domain}");

    let path = format!("/v2/service/{domain}/dns/zone");
    let response =
        send_request(reqwest::Method::GET, &path, None::<&()>, DateHeader::XDate).await?;

    if !response.status().is_success() {
        return Err(error_from_response(response).await);
    }

    response
        .json::<DnsZone>()
        .await
        .map_err(|e| format!("Failed to parse DNS zone response: {e}"))
}

/// Lists all DNS records for `domain`, paging through the collection.
#[tauri::command]
#[specta::specta]
pub async fn websupport_list_dns_records(domain: String) -> Result<Vec<DnsRecord>, String> {
    validate_path_segment(&domain, "Domain")?;
    log::info!("Listing Websupport DNS records for {domain}");

    let mut page: u32 = 1;
    let mut all_records: Vec<DnsRecord> = Vec::new();

    loop {
        let path = format!(
            "/v2/service/{domain}/dns/record?page={page}&rowsPerPage={DEFAULT_ROWS_PER_PAGE}"
        );
        let response =
            send_request(reqwest::Method::GET, &path, None::<&()>, DateHeader::XDate).await?;

        if !response.status().is_success() {
            return Err(error_from_response(response).await);
        }

        let page_body: DnsRecordPage = response
            .json()
            .await
            .map_err(|e| format!("Failed to parse DNS records page: {e}"))?;

        all_records.extend(page_body.data);

        if page_body.current_page >= page_body.total_pages || page_body.total_pages == 0 {
            break;
        }
        page += 1;
    }

    log::info!("Loaded {} DNS record(s) for {domain}", all_records.len());
    Ok(all_records)
}

/// Creates a DNS record via `POST /v2/service/{domain}/dns/record` (204 on success).
#[tauri::command]
#[specta::specta]
pub async fn websupport_create_dns_record(
    domain: String,
    record: CreateDnsRecordInput,
) -> Result<(), String> {
    validate_path_segment(&domain, "Domain")?;
    log::info!(
        "Creating Websupport DNS record type={} name={} on {domain}",
        record.record_type,
        record.name
    );

    let path = format!("/v2/service/{domain}/dns/record");
    let response = send_request(
        reqwest::Method::POST,
        &path,
        Some(&record),
        DateHeader::XDate,
    )
    .await?;

    let status = response.status();
    if status.as_u16() != 204 && !status.is_success() {
        return Err(error_from_response(response).await);
    }

    log::info!("Created DNS record on {domain}");
    Ok(())
}

/// Updates a DNS record via `PUT /v2/service/{domain}/dns/record/{recordId}` (204 on success).
#[tauri::command]
#[specta::specta]
pub async fn websupport_update_dns_record(
    domain: String,
    record_id: String,
    record: CreateDnsRecordInput,
) -> Result<(), String> {
    validate_path_segment(&domain, "Domain")?;
    validate_path_segment(&record_id, "Record id")?;

    log::info!("Updating Websupport DNS record {record_id} on {domain}");

    let path = format!("/v2/service/{domain}/dns/record/{record_id}");
    let response = send_request(
        reqwest::Method::PUT,
        &path,
        Some(&record),
        DateHeader::XDate,
    )
    .await?;

    let status = response.status();
    if status.as_u16() != 204 && !status.is_success() {
        return Err(error_from_response(response).await);
    }

    log::info!("Updated DNS record {record_id} on {domain}");
    Ok(())
}

/// Deletes a DNS record via `DELETE /v2/service/{domain}/dns/record/{recordId}` (204 on success).
#[tauri::command]
#[specta::specta]
pub async fn websupport_delete_dns_record(domain: String, record_id: String) -> Result<(), String> {
    validate_path_segment(&domain, "Domain")?;
    validate_path_segment(&record_id, "Record id")?;

    log::info!("Deleting Websupport DNS record {record_id} on {domain}");

    let path = format!("/v2/service/{domain}/dns/record/{record_id}");
    let response = send_request(
        reqwest::Method::DELETE,
        &path,
        None::<&()>,
        DateHeader::XDate,
    )
    .await?;

    let status = response.status();
    if status.as_u16() != 204 && !status.is_success() {
        return Err(error_from_response(response).await);
    }

    log::info!("Deleted DNS record {record_id} on {domain}");
    Ok(())
}
