#![allow(dead_code)]
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct AppIdentity(String);

impl AppIdentity {
    pub fn new(key: impl Into<String>) -> Result<Self, AppIdentityError> {
        let key = key.into();
        if key.trim().is_empty() {
            return Err(AppIdentityError::EmptyStableKey);
        }
        Ok(Self(key))
    }
    #[must_use]
    pub fn stable_key(&self) -> &str {
        &self.0
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AppIdentityError {
    EmptyStableKey,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AppDisplayMetadata {
    pub display_name: String,
    pub executable: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum UsageSubject {
    Identified(AppIdentity),
    Unclassified,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MeasurementDisposition {
    Track(UsageSubject),
    ExcludeTimeWise,
}

pub struct MeasurementPolicy {
    own_identity: AppIdentity,
}

impl MeasurementPolicy {
    #[must_use]
    pub fn new(own_identity: AppIdentity) -> Self {
        Self { own_identity }
    }
    #[must_use]
    pub fn classify(&self, identity: Option<AppIdentity>) -> MeasurementDisposition {
        match identity {
            Some(value) if value == self.own_identity => MeasurementDisposition::ExcludeTimeWise,
            Some(value) => MeasurementDisposition::Track(UsageSubject::Identified(value)),
            None => MeasurementDisposition::Track(UsageSubject::Unclassified),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn id(key: &str) -> AppIdentity {
        AppIdentity::new(key).unwrap()
    }

    #[test]
    fn identity_is_separate_from_metadata() {
        let identity = id("product:editor");
        let metadata = AppDisplayMetadata {
            display_name: "Editor".into(),
            executable: Some("editor.exe".into()),
        };
        assert_eq!(identity.stable_key(), "product:editor");
        assert_eq!(metadata.display_name, "Editor");
    }

    #[test]
    fn unclassified_and_self_exclusion_are_explicit() {
        let own = id("product:time-wise");
        let policy = MeasurementPolicy::new(own.clone());
        assert_eq!(
            policy.classify(Some(own)),
            MeasurementDisposition::ExcludeTimeWise
        );
        assert_eq!(
            policy.classify(None),
            MeasurementDisposition::Track(UsageSubject::Unclassified)
        );
    }
}
