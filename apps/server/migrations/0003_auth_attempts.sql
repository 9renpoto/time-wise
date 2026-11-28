CREATE TABLE auth_attempts (
    creation_order INTEGER PRIMARY KEY AUTOINCREMENT,
    id TEXT NOT NULL UNIQUE CHECK (length(id) > 0),
    generation TEXT NOT NULL CHECK (length(generation) > 0),
    client_profile_id TEXT NOT NULL CHECK (length(client_profile_id) > 0),
    purpose TEXT NOT NULL CHECK (purpose IN ('login', 'reauthenticate')),
    target_session_id TEXT CHECK (length(target_session_id) > 0),
    state TEXT NOT NULL DEFAULT 'created' CHECK (state IN (
        'created', 'pending', 'processing', 'verified', 'ready', 'consumed',
        'failed', 'cancelled', 'expired'
    )),
    comparison_code TEXT NOT NULL CHECK (
        length(comparison_code) = 10
        AND comparison_code NOT GLOB '*[^0123456789ABCDEFGHJKMNPQRSTVWXYZ]*'
        AND instr(comparison_code, char(0)) = 0
    ),
    claim_challenge_digest BLOB NOT NULL CHECK (length(claim_challenge_digest) = 32),
    launch_ticket_digest BLOB UNIQUE CHECK (length(launch_ticket_digest) = 32),
    created_at_ms INTEGER NOT NULL CHECK (created_at_ms >= 0),
    callback_expires_at_ms INTEGER GENERATED ALWAYS AS (created_at_ms + 600000) STORED,
    hard_expires_at_ms INTEGER GENERATED ALWAYS AS (created_at_ms + 930000) STORED,
    CHECK ((purpose = 'reauthenticate') = (target_session_id IS NOT NULL)),
    CHECK ((state = 'created') = (launch_ticket_digest IS NOT NULL))
) STRICT;

CREATE INDEX auth_attempts_by_hard_expiry ON auth_attempts (hard_expires_at_ms);
