ALTER TABLE auth_attempts ADD COLUMN browser_cookie_digest BLOB
    CHECK (length(browser_cookie_digest) = 32);
ALTER TABLE auth_attempts ADD COLUMN oauth_state_digest BLOB
    CHECK (length(oauth_state_digest) = 32);
ALTER TABLE auth_attempts ADD COLUMN encrypted_pkce_verifier BLOB
    CHECK (length(encrypted_pkce_verifier) > 0);
ALTER TABLE auth_attempts ADD COLUMN processing_started_at_ms INTEGER
    CHECK (processing_started_at_ms >= created_at_ms
        AND processing_started_at_ms < callback_expires_at_ms);
ALTER TABLE auth_attempts ADD COLUMN processing_marker BLOB
    CHECK (length(processing_marker) = 32)
    CHECK ((processing_marker IS NULL) = (processing_started_at_ms IS NULL));

CREATE UNIQUE INDEX auth_attempts_by_oauth_state
    ON auth_attempts (oauth_state_digest) WHERE oauth_state_digest IS NOT NULL;
