//! Device-local storage for the key used by the encrypted usage database.

use std::sync::Arc;
#[cfg(test)]
use std::sync::Mutex;

pub const USAGE_HISTORY_KEY_ID: &str = "usage-history-db-v1";

pub trait KeyStore: Send + Sync {
    fn load(&self, id: &str) -> Result<Option<Vec<u8>>, String>;
    fn store(&self, id: &str, key: &[u8]) -> Result<(), String>;
}

#[cfg(any(target_os = "macos", target_os = "windows"))]
#[cfg_attr(test, allow(dead_code))]
#[derive(Debug, Default)]
pub struct OsKeyStore;

#[cfg(any(target_os = "macos", target_os = "windows"))]
impl KeyStore for OsKeyStore {
    fn load(&self, id: &str) -> Result<Option<Vec<u8>>, String> {
        let entry = keyring::Entry::new("com.time-wise.local", id)
            .map_err(|error| format!("failed to open OS credential entry: {error}"))?;
        match entry.get_password() {
            Ok(value) => decode_key(&value).map(Some),
            Err(keyring::Error::NoEntry) => Ok(None),
            Err(error) => Err(format!("failed to read OS credential: {error}")),
        }
    }

    fn store(&self, id: &str, key: &[u8]) -> Result<(), String> {
        let entry = keyring::Entry::new("com.time-wise.local", id)
            .map_err(|error| format!("failed to open OS credential entry: {error}"))?;
        entry
            .set_password(&encode_key(key))
            .map_err(|error| format!("failed to store OS credential: {error}"))
    }
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

#[allow(dead_code)]
fn encode_key(key: &[u8]) -> String {
    key.iter().map(|byte| format!("{byte:02x}")).collect()
}

#[allow(dead_code)]
fn decode_key(value: &str) -> Result<Vec<u8>, String> {
    if value.len() != 64 || !value.is_ascii() {
        return Err("OS credential contains an invalid database key".to_string());
    }
    (0..value.len())
        .step_by(2)
        .map(|index| {
            u8::from_str_radix(&value[index..index + 2], 16)
                .map_err(|_| "OS credential contains an invalid database key".to_string())
        })
        .collect()
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
