use worker::{
    d1::{D1Database, D1Type},
    Error, Result,
};

/// Parameters produced by the browser bootstrap flow before consuming the launch ticket.
pub struct BrowserBinding<'a> {
    pub state_digest: &'a [u8; 32],
    pub cookie_digest: &'a [u8; 32],
    /// Environment-encrypted PKCE verifier. The storage layer only accepts ciphertext.
    pub encrypted_pkce_verifier: &'a [u8],
}

/// Both proofs and the fresh operation marker required to admit a callback.
pub struct CallbackClaim<'a> {
    pub attempt_id: &'a str,
    pub state_digest: &'a [u8; 32],
    pub cookie_digest: &'a [u8; 32],
    pub processing_marker: &'a [u8; 32],
}

/// Provider completion before the separate publication transaction.
pub enum ProviderCompletion {
    ReadyToPublish,
    Failed,
    Denied,
}

/// Primary D1 access for OAuth attempt admission and cleanup.
pub struct OAuthAttempts<'a> {
    db: &'a D1Database,
    generation: &'a str,
}

impl<'a> OAuthAttempts<'a> {
    /// Connect using the current Worker authentication generation.
    pub fn new(db: &'a D1Database, generation: &'a str) -> Self {
        Self { db, generation }
    }

    /// Consume one launch ticket and return the newly bound attempt ID.
    pub async fn bootstrap(
        &self,
        ticket_digest: &[u8; 32],
        now_ms: u64,
        binding: BrowserBinding<'_>,
    ) -> Result<Option<String>> {
        self.db
            .prepare(include_str!("bootstrap.sql"))
            .bind_refs(&[
                D1Type::Blob(ticket_digest),
                D1Type::Text(self.generation),
                timestamp(now_ms)?,
                D1Type::Blob(binding.state_digest),
                D1Type::Blob(binding.cookie_digest),
                D1Type::Blob(binding.encrypted_pkce_verifier),
            ])?
            .first(Some("id"))
            .await
    }

    /// Resolve the attempt-specific cookie name. Claim repeats all admission checks.
    pub async fn pending_attempt_id(
        &self,
        state_digest: &[u8; 32],
        now_ms: u64,
    ) -> Result<Option<String>> {
        self.db
            .prepare(include_str!("lookup.sql"))
            .bind_refs(&[
                D1Type::Blob(state_digest),
                D1Type::Text(self.generation),
                timestamp(now_ms)?,
            ])?
            .first(Some("id"))
            .await
    }

    /// Give the winning provider operation its encrypted PKCE verifier.
    pub async fn claim(&self, claim: CallbackClaim<'_>, now_ms: u64) -> Result<Option<Vec<u8>>> {
        self.db
            .prepare(include_str!("claim.sql"))
            .bind_refs(&[
                D1Type::Text(claim.attempt_id),
                D1Type::Text(self.generation),
                timestamp(now_ms)?,
                D1Type::Blob(claim.state_digest),
                D1Type::Blob(claim.cookie_digest),
                D1Type::Blob(claim.processing_marker),
            ])?
            .first(Some("encrypted_pkce_verifier"))
            .await
    }

    /// Cancel an eligible attempt and erase its provider material.
    pub async fn cancel(
        &self,
        attempt_id: &str,
        challenge_digest: &[u8; 32],
        now_ms: u64,
    ) -> Result<bool> {
        let cancelled: Option<String> = self
            .db
            .prepare(include_str!("cancel.sql"))
            .bind_refs(&[
                D1Type::Text(attempt_id),
                D1Type::Text(self.generation),
                timestamp(now_ms)?,
                D1Type::Blob(challenge_digest),
            ])?
            .first(Some("id"))
            .await?;
        Ok(cancelled.is_some())
    }

    /// Erase provider material for the winning operation, even after admission expires.
    pub async fn finish_provider(
        &self,
        attempt_id: &str,
        processing_marker: &[u8; 32],
        completion: ProviderCompletion,
    ) -> Result<bool> {
        let state = match completion {
            ProviderCompletion::ReadyToPublish => "processing",
            ProviderCompletion::Failed => "failed",
            ProviderCompletion::Denied => "cancelled",
        };
        let completed: Option<String> = self
            .db
            .prepare(include_str!("finish.sql"))
            .bind_refs(&[
                D1Type::Text(attempt_id),
                D1Type::Text(self.generation),
                D1Type::Blob(processing_marker),
                D1Type::Text(state),
            ])?
            .first(Some("id"))
            .await?;
        Ok(completed.is_some())
    }
}

fn timestamp(ms: u64) -> Result<D1Type<'static>> {
    if ms > (1_u64 << 53) - 1 {
        return Err(Error::RustError(
            "Timestamp exceeds D1 integer precision".into(),
        ));
    }
    Ok(D1Type::Real(ms as f64))
}
