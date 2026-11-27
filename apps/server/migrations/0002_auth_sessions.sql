CREATE TABLE auth_sessions (
    id TEXT PRIMARY KEY NOT NULL CHECK (length(id) > 0),
    account_id TEXT NOT NULL REFERENCES auth_accounts (id),
    generation TEXT NOT NULL CHECK (length(generation) > 0),
    provider_verified_at_ms INTEGER NOT NULL CHECK (provider_verified_at_ms >= 0),
    issued_at_ms INTEGER NOT NULL CHECK (issued_at_ms >= provider_verified_at_ms),
    idle_expires_at_ms INTEGER NOT NULL CHECK (idle_expires_at_ms > issued_at_ms),
    absolute_expires_at_ms INTEGER NOT NULL CHECK (absolute_expires_at_ms > issued_at_ms),
    reauthenticated_at_ms INTEGER CHECK (reauthenticated_at_ms >= issued_at_ms),
    revoked_at_ms INTEGER CHECK (revoked_at_ms >= issued_at_ms)
) STRICT;

CREATE INDEX auth_sessions_by_account ON auth_sessions (account_id);
CREATE INDEX auth_sessions_by_absolute_expiry ON auth_sessions (absolute_expires_at_ms);

CREATE TABLE auth_access_credentials (
    digest BLOB PRIMARY KEY NOT NULL CHECK (length(digest) = 32),
    session_id TEXT NOT NULL REFERENCES auth_sessions (id) ON DELETE CASCADE,
    issued_at_ms INTEGER NOT NULL CHECK (issued_at_ms >= 0),
    expires_at_ms INTEGER NOT NULL CHECK (expires_at_ms > issued_at_ms)
) STRICT;

CREATE INDEX auth_access_credentials_by_session ON auth_access_credentials (session_id);

CREATE TABLE auth_refresh_credentials (
    digest BLOB PRIMARY KEY NOT NULL CHECK (length(digest) = 32),
    session_id TEXT NOT NULL REFERENCES auth_sessions (id) ON DELETE CASCADE,
    consumed_at_ms INTEGER CHECK (consumed_at_ms >= 0),
    rotation_marker BLOB CHECK (length(rotation_marker) = 32),
    CHECK ((consumed_at_ms IS NULL) = (rotation_marker IS NULL))
) STRICT;

CREATE INDEX auth_refresh_credentials_by_session ON auth_refresh_credentials (session_id);
CREATE UNIQUE INDEX auth_one_current_refresh_per_session
    ON auth_refresh_credentials (session_id) WHERE consumed_at_ms IS NULL;
