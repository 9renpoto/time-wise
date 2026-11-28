UPDATE auth_attempts
SET state = ?4,
    oauth_state_digest = NULL,
    encrypted_pkce_verifier = NULL,
    processing_started_at_ms = CASE WHEN ?4 = 'processing' THEN processing_started_at_ms END,
    processing_marker = CASE WHEN ?4 = 'processing' THEN processing_marker END
WHERE id = ?1
  AND generation = ?2
  AND state = 'processing'
  AND processing_marker = ?3
  AND encrypted_pkce_verifier IS NOT NULL
  AND ?4 IN ('processing', 'failed', 'cancelled')
RETURNING id
