//! OS keychain-backed secret storage.
//!
//! Stores integration credentials in the platform keychain via the `keyring` crate
//! (macOS Keychain / Windows Credential Manager / Linux Secret Service).
//! Secrets are never written to preferences JSON or other app data files.

use keyring::Entry;

/// Fixed keyring service name for all integration secrets.
const SERVICE_NAME: &str = "com.tauristarter.integrations";

/// Allowed secret key names. Anything else is rejected to prevent arbitrary keychain writes.
const ALLOWED_SECRET_KEYS: &[&str] = &[
    "websupport_identifier",
    "websupport_secret",
    "websupport_dyndns_identifier",
    "websupport_dyndns_secret",
    "mistral_api_key",
];

/// Validates that `key` is one of the known secret identifiers.
///
/// This is pure logic and does not touch the OS keychain — unit-tested in isolation.
pub fn validate_secret_key(key: &str) -> Result<(), String> {
    if ALLOWED_SECRET_KEYS.contains(&key) {
        Ok(())
    } else {
        Err(format!(
            "Invalid secret key '{key}'. Allowed keys: {}",
            ALLOWED_SECRET_KEYS.join(", ")
        ))
    }
}

/// Creates a keyring entry for the given validated secret key.
fn entry_for_key(key: &str) -> Result<Entry, String> {
    Entry::new(SERVICE_NAME, key).map_err(|e| {
        log::error!("Failed to open keyring entry for '{key}': {e}");
        format!("Keyring error: {e}")
    })
}

/// Saves a secret value to the OS keychain.
///
/// The `key` must be one of the allowed secret identifiers.
#[tauri::command]
#[specta::specta]
pub async fn save_secret(key: String, value: String) -> Result<(), String> {
    validate_secret_key(&key)?;

    log::debug!("Saving secret to keychain: {key}");
    let entry = entry_for_key(&key)?;
    entry.set_password(&value).map_err(|e| {
        log::error!("Failed to save secret '{key}': {e}");
        format!("Failed to save secret: {e}")
    })?;

    log::info!("Successfully saved secret '{key}' to keychain");
    Ok(())
}

/// Loads a secret from the OS keychain.
///
/// Returns `Ok(None)` when the secret has not been set.
/// The `key` must be one of the allowed secret identifiers.
#[tauri::command]
#[specta::specta]
pub async fn get_secret(key: String) -> Result<Option<String>, String> {
    validate_secret_key(&key)?;

    log::debug!("Loading secret from keychain: {key}");
    let entry = entry_for_key(&key)?;

    match entry.get_password() {
        Ok(value) => {
            log::info!("Successfully loaded secret '{key}' from keychain");
            Ok(Some(value))
        }
        Err(keyring::Error::NoEntry) => {
            log::debug!("Secret '{key}' not found in keychain");
            Ok(None)
        }
        Err(e) => {
            log::error!("Failed to get secret '{key}': {e}");
            Err(format!("Failed to get secret: {e}"))
        }
    }
}

/// Deletes a secret from the OS keychain.
///
/// Missing entries are treated as success (idempotent delete).
/// The `key` must be one of the allowed secret identifiers.
#[tauri::command]
#[specta::specta]
pub async fn delete_secret(key: String) -> Result<(), String> {
    validate_secret_key(&key)?;

    log::debug!("Deleting secret from keychain: {key}");
    let entry = entry_for_key(&key)?;

    match entry.delete_credential() {
        Ok(()) => {
            log::info!("Successfully deleted secret '{key}' from keychain");
            Ok(())
        }
        Err(keyring::Error::NoEntry) => {
            log::debug!("Secret '{key}' already absent from keychain");
            Ok(())
        }
        Err(e) => {
            log::error!("Failed to delete secret '{key}': {e}");
            Err(format!("Failed to delete secret: {e}"))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn validate_secret_key_accepts_all_allowed_keys() {
        for key in ALLOWED_SECRET_KEYS {
            assert!(
                validate_secret_key(key).is_ok(),
                "expected allowed key '{key}' to be accepted"
            );
        }
    }

    #[test]
    fn validate_secret_key_rejects_unknown_key() {
        let err =
            validate_secret_key("not_a_real_secret").expect_err("unknown key must be rejected");
        assert!(
            err.contains("Invalid secret key"),
            "error should mention invalid key, got: {err}"
        );
        assert!(
            err.contains("websupport_identifier"),
            "error should list allowed keys, got: {err}"
        );
    }

    #[test]
    fn validate_secret_key_rejects_empty_key() {
        assert!(validate_secret_key("").is_err());
    }

    #[test]
    fn validate_secret_key_rejects_partial_and_case_variants() {
        assert!(validate_secret_key("websupport").is_err());
        assert!(validate_secret_key("Mistral_Api_Key").is_err());
        assert!(validate_secret_key("websupport_identifier ").is_err());
        assert!(validate_secret_key(" websupport_identifier").is_err());
    }

    #[test]
    fn allowed_secret_keys_has_exactly_five_entries() {
        assert_eq!(ALLOWED_SECRET_KEYS.len(), 5);
        assert!(ALLOWED_SECRET_KEYS.contains(&"websupport_identifier"));
        assert!(ALLOWED_SECRET_KEYS.contains(&"websupport_secret"));
        assert!(ALLOWED_SECRET_KEYS.contains(&"websupport_dyndns_identifier"));
        assert!(ALLOWED_SECRET_KEYS.contains(&"websupport_dyndns_secret"));
        assert!(ALLOWED_SECRET_KEYS.contains(&"mistral_api_key"));
    }

    #[test]
    fn service_name_is_stable() {
        assert_eq!(SERVICE_NAME, "com.tauristarter.integrations");
    }
}
