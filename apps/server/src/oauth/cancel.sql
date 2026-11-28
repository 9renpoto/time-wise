UPDATE auth_attempts
SET state = 'cancelled',
    launch_ticket_digest = NULL,
    oauth_state_digest = NULL,
    encrypted_pkce_verifier = NULL,
    processing_started_at_ms = NULL,
    processing_marker = NULL
WHERE id = ?1
  AND generation = ?2
  AND ?3 >= created_at_ms
  AND ?3 < hard_expires_at_ms
  AND claim_challenge_digest = ?4
  AND state IN ('created', 'pending', 'processing', 'verified', 'ready', 'cancelled')
RETURNING id
