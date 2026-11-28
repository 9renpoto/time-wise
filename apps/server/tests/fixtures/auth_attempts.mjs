async function expectConstraint(operation, expected) {
  try {
    await operation;
  } catch (error) {
    if (String(error).includes(expected + " constraint failed")) return;
    throw error;
  }
  throw new Error(`Expected a ${expected} constraint failure`);
}

async function verify(db, [migration]) {
  await db.exec(migration);
  const digest = (byte) => new Uint8Array(32).fill(byte);
  const create = (id, ticket, {
    purpose = "login", target = null, code = "ABCDE01234", challenge = digest(0),
  } = {}) => db.prepare(`
    INSERT INTO auth_attempts (
      id, generation, client_profile_id, purpose, target_session_id,
      comparison_code, claim_challenge_digest, launch_ticket_digest, created_at_ms
    ) VALUES (?, 'generation-1', 'synthetic-profile', ?, ?, ?, ?, ?, 1000)
  `).bind(id, purpose, target, code, challenge, ticket).run();
  const describe = (id) => db.prepare(`
    SELECT creation_order, state, callback_expires_at_ms, hard_expires_at_ms
    FROM auth_attempts WHERE id = ?
  `).bind(id).first();

  await create("first", digest(1));
  await create("second", digest(2));
  const first = await describe("first");
  const second = await describe("second");
  if (!(first.creation_order < second.creation_order)) {
    throw new Error("Attempts created at the same time must have distinct, ordered positions");
  }
  await db.prepare("DELETE FROM auth_attempts WHERE id = 'second'").run();
  await create("third", digest(4));
  if (!((await describe("third")).creation_order > second.creation_order)) {
    throw new Error("An attempt position must not be reused after deletion");
  }

  await expectConstraint(create("first", digest(3)), "UNIQUE");
  await expectConstraint(create("duplicate-ticket", digest(1)), "UNIQUE");
  await expectConstraint(create("no-ticket", null), "CHECK");
  await expectConstraint(create("malformed-challenge", digest(3), { challenge: new Uint8Array(31) }), "CHECK");
  await expectConstraint(create("malformed-code", digest(3), { code: "ABCDE0123O" }), "CHECK");
  await expectConstraint(create("missing-target", digest(3), { purpose: "reauthenticate" }), "CHECK");
  await expectConstraint(create("unexpected-target", digest(3), { target: "synthetic-session" }), "CHECK");
  await create("reauth", digest(3), { purpose: "reauthenticate", target: "synthetic-session" });
  await expectConstraint(db.prepare(
    "UPDATE auth_attempts SET state = 'pending' WHERE id = 'first'",
  ).run(), "CHECK");
  await db.prepare(
    "UPDATE auth_attempts SET state = 'pending', launch_ticket_digest = NULL WHERE id = 'first'",
  ).run();
  if ((await describe("first")).state !== "pending") {
    throw new Error("A pending attempt must no longer retain its launch ticket");
  }

  return {
    state: first.state,
    callback_expires_at_ms: first.callback_expires_at_ms,
    hard_expires_at_ms: first.hard_expires_at_ms,
  };
}
