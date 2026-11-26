function digest(byte) {
  return new Uint8Array(32).fill(byte);
}

async function expectConstraint(operation, expected) {
  try {
    await operation;
  } catch (error) {
    if (String(error).includes(expected + " constraint failed")) return;
    throw error;
  }
  throw new Error(`Expected a ${expected} constraint failure`);
}

async function verify(db) {
  await db.exec(identityMigration);
  await db.prepare("INSERT INTO auth_accounts VALUES ('session-owner'), ('other-owner')").run();
  await db.exec(sessionMigration);
  if (await db.prepare("SELECT count(*) AS n FROM auth_accounts").first("n") !== 2) {
    throw new Error("The session migration must preserve existing accounts");
  }

  const createSession = (id, account) => db.prepare(`
    INSERT INTO auth_sessions (
      id, account_id, generation, provider_verified_at_ms,
      issued_at_ms, idle_expires_at_ms, absolute_expires_at_ms
    ) VALUES (?, ?, 'generation-1', 1000, 2000, 604802000, 2592001000)
  `).bind(id, account).run();
  await createSession("session-1", "session-owner");
  await createSession("session-2", "other-owner");
  await expectConstraint(createSession("orphan", "missing"), "FOREIGN KEY");

  const refresh = (hash, session) => db.prepare(`
    INSERT INTO auth_refresh_credentials (digest, session_id) VALUES (?, ?)
  `).bind(hash, session);
  await refresh(digest(1), "session-1").run();
  await refresh(digest(2), "session-2").run();
  await expectConstraint(refresh(digest(3), "session-1").run(), "UNIQUE");
  await expectConstraint(refresh(digest(4), "missing").run(), "FOREIGN KEY");
  await expectConstraint(refresh(new Uint8Array(31), "session-1").run(), "CHECK");

  const consume = db.prepare(`
    UPDATE auth_refresh_credentials SET consumed_at_ms = 3000, rotation_marker = ?
    WHERE digest = ?
  `).bind(digest(10), digest(1));
  await expectConstraint(db.batch([consume, refresh(digest(2), "session-1")]), "UNIQUE");
  const current = db.prepare(`
    SELECT COUNT(*) AS n FROM auth_refresh_credentials
    WHERE session_id = 'session-1' AND consumed_at_ms IS NULL
  `);
  if (await current.first("n") !== 1) throw new Error("A failed replacement consumed the predecessor");
  await db.batch([consume, refresh(digest(3), "session-1")]);
  if (await current.first("n") !== 1) throw new Error("Expected exactly one replacement");
  const consumedAt = await db.prepare(
    "SELECT consumed_at_ms FROM auth_refresh_credentials WHERE digest = ?",
  ).bind(digest(1)).first("consumed_at_ms");
  if (consumedAt !== 3000) throw new Error("A replacement must retain the consumed digest");
  await expectConstraint(db.prepare(`
    UPDATE auth_refresh_credentials SET consumed_at_ms = 4000 WHERE digest = ?
  `).bind(digest(3)).run(), "CHECK");

  const addAccess = (hash, expiry) => db.prepare(`
    INSERT INTO auth_access_credentials (digest, session_id, issued_at_ms, expires_at_ms)
    VALUES (?, 'session-1', 2000, ?)
  `).bind(hash, expiry).run();
  await addAccess(digest(5), 902000);
  await addAccess(digest(6), 902001);
  await expectConstraint(addAccess(digest(7), 2000), "CHECK");

  await db.prepare("DELETE FROM auth_sessions WHERE id = 'session-1'").run();
  const counts = await db.prepare(`
    SELECT (SELECT COUNT(*) FROM auth_access_credentials) AS access_count,
           (SELECT COUNT(*) FROM auth_refresh_credentials) AS refresh_count,
           (SELECT account_id FROM auth_sessions WHERE id = 'session-2') AS owner
  `).first();
  if (counts.access_count !== 0 || counts.refresh_count !== 1 || counts.owner !== "other-owner") {
    throw new Error("Removing one session must remove its credential history and preserve the other session");
  }
}

export default {
  async fetch(request, { AUTH_DB: db }) {
    if (new URL(request.url).pathname === "/ready") return Response.json({ ready: true });
    try {
      await verify(db);
      return Response.json({ verified: true });
    } catch (error) {
      return Response.json({ error: String(error) }, { status: 500 });
    }
  },
};
