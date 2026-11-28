UPDATE auth_attempts
SET state = 'processing',
    processing_started_at_ms = ?3,
    processing_marker = ?6
WHERE id = ?1
  AND generation = ?2
  AND state = 'pending'
  AND ?3 >= created_at_ms
  AND ?3 < callback_expires_at_ms
  AND oauth_state_digest = ?4
  AND browser_cookie_digest = ?5
  AND encrypted_pkce_verifier IS NOT NULL
RETURNING encrypted_pkce_verifier
