//! Device-local storage for the key used by the encrypted usage database.

use std::sync::Arc;
#[cfg(test)]
use std::sync::Mutex;

pub const USAGE_HISTORY_KEY_ID: &str = "usage-history-db-v1";

#[cfg(target_os = "windows")]
const WINDOWS_CREDENTIAL_PERSISTENCE: windows::Win32::Security::Credentials::CRED_PERSIST =
    windows::Win32::Security::Credentials::CRED_PERSIST_LOCAL_MACHINE;

pub trait KeyStore: Send + Sync {
    fn load(&self, id: &str) -> Result<Option<Vec<u8>>, String>;
    fn store(&self, id: &str, key: &[u8]) -> Result<(), String>;
}

#[cfg(any(target_os = "macos", target_os = "windows"))]
#[cfg_attr(test, allow(dead_code))]
#[derive(Debug, Default)]
pub struct OsKeyStore;

#[cfg(target_os = "macos")]
impl KeyStore for OsKeyStore {
    fn load(&self, id: &str) -> Result<Option<Vec<u8>>, String> {
        let entry = keyring::Entry::new("com.time-wise.local", id)
            .map_err(|error| format!("failed to open OS credential entry: {error}"))?;
        match entry.get_secret() {
            Ok(value) => Ok(Some(value)),
            Err(keyring::Error::NoEntry) => Ok(None),
            Err(error) => Err(format!("failed to read OS credential: {error}")),
        }
    }

    fn store(&self, id: &str, key: &[u8]) -> Result<(), String> {
        let entry = keyring::Entry::new("com.time-wise.local", id)
            .map_err(|error| format!("failed to open OS credential entry: {error}"))?;
        entry
            .set_secret(key)
            .map_err(|error| format!("failed to store OS credential: {error}"))
    }
}

#[cfg(target_os = "windows")]
impl KeyStore for OsKeyStore {
    fn load(&self, id: &str) -> Result<Option<Vec<u8>>, String> {
        use std::ptr;
        use windows::core::{HRESULT, PCWSTR};
        use windows::Win32::Foundation::ERROR_NOT_FOUND;
        use windows::Win32::Security::Credentials::{
            CredFree, CredReadW, CREDENTIALW, CRED_TYPE_GENERIC,
        };

        let target = windows_credential_target(id);
        let mut credential = ptr::null_mut::<CREDENTIALW>();
        let read = unsafe {
            CredReadW(
                PCWSTR(target.as_ptr()),
                CRED_TYPE_GENERIC,
                None,
                &mut credential,
            )
        };
        if let Err(error) = read {
            if error.code() == HRESULT::from_win32(ERROR_NOT_FOUND.0) {
                return Ok(None);
            }
            return Err(format!("failed to read OS credential: {error}"));
        }
        if credential.is_null() {
            return Err("OS credential API returned an empty credential".to_string());
        }

        let value = unsafe {
            let credential_ref = &*credential;
            let value = if credential_ref.CredentialBlobSize == 0 {
                Vec::new()
            } else if credential_ref.CredentialBlob.is_null() {
                CredFree(credential.cast());
                return Err("OS credential contains an invalid database key".to_string());
            } else {
                std::slice::from_raw_parts(
                    credential_ref.CredentialBlob,
                    credential_ref.CredentialBlobSize as usize,
                )
                .to_vec()
            };
            CredFree(credential.cast());
            value
        };
        Ok(Some(value))
    }

    fn store(&self, id: &str, key: &[u8]) -> Result<(), String> {
        use windows::core::PWSTR;
        use windows::Win32::Security::Credentials::{CredWriteW, CREDENTIALW, CRED_TYPE_GENERIC};

        let mut target = windows_credential_target(id);
        let mut secret = key.to_vec();
        let secret_len = secret
            .len()
            .try_into()
            .map_err(|_| "database key is too long for OS credential storage")?;
        let credential = CREDENTIALW {
            Type: CRED_TYPE_GENERIC,
            TargetName: PWSTR(target.as_mut_ptr()),
            CredentialBlobSize: secret_len,
            CredentialBlob: secret.as_mut_ptr(),
            Persist: WINDOWS_CREDENTIAL_PERSISTENCE,
            ..Default::default()
        };
        let result = unsafe { CredWriteW(&credential, 0) };
        secret.fill(0);
        result.map_err(|error| format!("failed to store OS credential: {error}"))
    }
}

#[cfg(target_os = "windows")]
fn windows_credential_target(id: &str) -> Vec<u16> {
    format!("com.time-wise.local.{id}")
        .encode_utf16()
        .chain(std::iter::once(0))
        .collect()
}

#[cfg(not(any(target_os = "macos", target_os = "windows")))]
#[derive(Debug, Default)]
pub struct OsKeyStore;

#[cfg(not(any(target_os = "macos", target_os = "windows")))]
impl KeyStore for OsKeyStore {
    fn load(&self, _id: &str) -> Result<Option<Vec<u8>>, String> {
        Err("OS credential storage is unavailable on this platform".to_string())
    }

    fn store(&self, _id: &str, _key: &[u8]) -> Result<(), String> {
        Err("OS credential storage is unavailable on this platform".to_string())
    }
}

#[cfg(not(test))]
pub fn default_key_store() -> Arc<dyn KeyStore> {
    Arc::new(OsKeyStore)
}

#[cfg(test)]
pub fn default_key_store() -> Arc<dyn KeyStore> {
    Arc::new(MemoryKeyStore::new(None))
}

pub fn load_or_create(store: &dyn KeyStore, id: &str) -> Result<Vec<u8>, String> {
    if let Some(key) = store.load(id)? {
        if key.len() != 32 {
            return Err("OS credential contains an invalid database key".to_string());
        }
        return Ok(key);
    }

    let mut key = vec![0; 32];
    getrandom::fill(&mut key)
        .map_err(|error| format!("failed to generate database key: {error}"))?;
    store.store(id, &key)?;
    Ok(key)
}

#[cfg(test)]
pub struct MemoryKeyStore {
    key: Mutex<Option<Vec<u8>>>,
    fail: bool,
}

#[cfg(test)]
impl MemoryKeyStore {
    pub fn new(key: Option<Vec<u8>>) -> Self {
        Self {
            key: Mutex::new(key),
            fail: false,
        }
    }
    #[allow(dead_code)]
    pub fn failing() -> Self {
        Self {
            key: Mutex::new(None),
            fail: true,
        }
    }
}

#[cfg(test)]
impl KeyStore for MemoryKeyStore {
    fn load(&self, _id: &str) -> Result<Option<Vec<u8>>, String> {
        if self.fail {
            return Err("credential store failure".to_string());
        }
        Ok(self.key.lock().unwrap().clone())
    }
    fn store(&self, _id: &str, key: &[u8]) -> Result<(), String> {
        if self.fail {
            return Err("credential store failure".to_string());
        }
        *self.key.lock().unwrap() = Some(key.to_vec());
        Ok(())
    }
}

#[cfg(all(test, target_os = "windows"))]
mod windows_tests {
    use super::*;
    use windows::Win32::Security::Credentials::CRED_PERSIST_LOCAL_MACHINE;

    #[test]
    fn credentials_are_limited_to_the_local_machine() {
        assert_eq!(WINDOWS_CREDENTIAL_PERSISTENCE, CRED_PERSIST_LOCAL_MACHINE);
    }
}
