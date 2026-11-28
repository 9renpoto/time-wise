SELECT id FROM auth_attempts
WHERE oauth_state_digest = ?1
  AND generation = ?2
  AND state = 'pending'
  AND ?3 >= created_at_ms
  AND ?3 < callback_expires_at_ms
