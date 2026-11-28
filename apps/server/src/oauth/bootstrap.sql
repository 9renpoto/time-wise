UPDATE auth_attempts
SET state = 'pending',
    launch_ticket_digest = NULL,
    oauth_state_digest = ?4,
    browser_cookie_digest = ?5,
    encrypted_pkce_verifier = ?6
WHERE launch_ticket_digest = ?1
  AND generation = ?2
  AND state = 'created'
  AND ?3 >= created_at_ms
  AND ?3 < callback_expires_at_ms
RETURNING id
