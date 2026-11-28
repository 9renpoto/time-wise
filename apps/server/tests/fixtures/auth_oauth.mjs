async function verify(db, [attempts, correlation]) {
  await db.exec(attempts);
  await db.exec(correlation);
  const blob = (value) => new Uint8Array(32).fill(value);
  const ciphertext = Uint8Array.of(83, 121, 110, 116, 104);
  const generation = "current";
  const deadline = 601000;
  const now = 2000;
  const equal = (actual, expected, label) => {
    if (JSON.stringify(actual) !== JSON.stringify(expected)) throw new Error(label);
  };
  const seed = (id, ticket, gen = generation) => db.prepare(`
    INSERT INTO auth_attempts (
      id, generation, client_profile_id, purpose, comparison_code,
      claim_challenge_digest, launch_ticket_digest, created_at_ms
    ) VALUES (?, ?, 'synthetic-profile', 'login', 'ABCDE01234', ?, ?, 1000)
  `).bind(id, gen, blob(ticket + 100), blob(ticket)).run();
  const bootstrap = (ticket, state, cookie, at = now, gen = generation) =>
    db.prepare(sql.bootstrap).bind(blob(ticket), gen, at, blob(state), blob(cookie), ciphertext).first("id");
  const lookup = (state, at = now, gen = generation) =>
    db.prepare(sql.lookup).bind(blob(state), gen, at).first("id");
  const claim = (id, state, cookie, marker, at = now, gen = generation) =>
    db.prepare(sql.claim).bind(id, gen, at, blob(state), blob(cookie), blob(marker)).first();
  const cancel = (id, proof, at = now, gen = generation) =>
    db.prepare(sql.cancel).bind(id, gen, at, blob(proof)).first("id");
  const finish = (id, marker, outcome, gen = generation) =>
    db.prepare(sql.finish).bind(id, gen, blob(marker), outcome).first("id");

  await seed("first", 1);
  await seed("second", 2);
  await seed("restored", 3, "old");
  equal(await bootstrap(3, 33, 43), null, "Old generation must not redeem");
  equal(await bootstrap(1, 31, 41, deadline), null, "Exact callback deadline must reject bootstrap");
  const bootstrapResults = await Promise.all(Array.from({ length: 8 }, () => bootstrap(1, 31, 41)));
  equal(bootstrapResults.filter(Boolean), ["first"], "Exactly one bootstrap must win");
  equal(await bootstrap(1, 91, 92), null, "Ticket must stay consumed");
  let duplicateRejected = false;
  try {
    await bootstrap(2, 31, 42);
  } catch (error) {
    duplicateRejected = String(error).includes("UNIQUE constraint failed");
  }
  equal(duplicateRejected, true, "Duplicate state must roll back the second bootstrap");
  equal(await bootstrap(2, 32, 42), "second", "Constraint failure must leave ticket usable");
  equal(await lookup(31), "first", "State lookup must identify the correct attempt");
  equal(await lookup(31, deadline), null, "Expired state lookup must fail");
  equal(await lookup(31, now, "old"), null, "Lookup must guard generation");
  equal(await claim("first", 32, 41, 51), null, "Swapped OAuth state must fail");
  equal(await claim("first", 31, 42, 51), null, "Swapped browser must fail");
  equal(await claim("first", 31, 41, 51, deadline), null, "Exact callback deadline must reject claim");
  equal(await claim("first", 31, 41, 51, now, "old"), null, "Old generation must not claim");
  const claims = await Promise.all(Array.from({ length: 8 }, (_, index) => claim("first", 31, 41, 51 + index)));
  equal(claims.filter(Boolean).length, 1, "Exactly one callback claim must win");
  const winner = claims.findIndex(Boolean);
  equal(Array.from(claims[winner].encrypted_pkce_verifier), Array.from(ciphertext), "The winner receives the encrypted verifier");
  equal(await lookup(31), null, "Claimed state must not reopen");
  equal(await finish("first", 99, "failed"), null, "An unrelated processing marker must not erase");
  equal(await finish("first", 51 + winner, "processing"), "first", "The winner may erase provider material before publication");
  const inspect = (id) => db.prepare(`
    SELECT state, oauth_state_digest IS NULL AS state_erased,
           encrypted_pkce_verifier IS NULL AS verifier_erased
    FROM auth_attempts WHERE id = ?
  `).bind(id).first();
  const erased = (row, state) => {
    equal(row.state, state, "Expected the terminal or processing state");
    equal(row.state_erased, 1, "OAuth correlation should be erased");
    equal(row.verifier_erased, 1, "PKCE ciphertext should be erased");
  };
  erased(await inspect("first"), "processing");
  equal(await cancel("second", 101), null, "Another attempt's proof must not cancel");
  equal(await cancel("second", 102, 931000), null, "Exact hard deadline must reject cancellation");
  equal(await cancel("second", 102), "second", "The bound proof can cancel");
  equal(await cancel("second", 102), "second", "Cancellation is idempotent");
  equal(await claim("second", 32, 42, 61), null, "Cancellation must prevent admission");
  erased(await inspect("second"), "cancelled");
  equal(await cancel("first", 101), "first", "Cancellation can win after callback admission");
  equal(await finish("first", 51 + winner, "failed"), null, "Provider completion must not reopen cancellation");
  await seed("failure", 4);
  equal(await bootstrap(4, 34, 44), "failure", "Bootstrap failure fixture");
  if (!await claim("failure", 34, 44, 64)) throw new Error("Claim failure fixture");
  equal(await finish("failure", 64, "failed"), "failure", "The winner may fail the attempt");
  erased(await inspect("failure"), "failed");
  await seed("denied", 5);
  equal(await bootstrap(5, 35, 45), "denied", "Bootstrap denial fixture");
  if (!await claim("denied", 35, 45, 65)) throw new Error("Claim denial fixture");
  equal(await finish("denied", 65, "cancelled"), "denied", "Provider denial must cancel");
  erased(await inspect("denied"), "cancelled");


  return { verified: true };
}
