async function bindIdentity(db, accountId, subject) {
  return db.batch([
    db.prepare("INSERT INTO auth_accounts (id) VALUES (?)").bind(accountId),
    db.prepare(
      "INSERT INTO auth_identities (provider, subject, account_id) VALUES ('github', ?, ?)",
    ).bind(subject, accountId),
  ]);
}

async function rejection(operation) {
  try {
    await operation;
    return null;
  } catch (error) {
    return String(error);
  }
}

export default {
  async fetch(request, { AUTH_DB: db }) {
    const path = new URL(request.url).pathname;
    if (path === "/ready") return Response.json({ ready: true });
    try {
      await db.exec(migration);
      const candidates = Array.from({ length: 8 }, (_, index) =>
        rejection(bindIdentity(db, `candidate-${index}`, "9007199254740992")),
      );
      const outcomes = await Promise.all(candidates);
      await bindIdentity(db, "other-account", "9007199254740993");
      const malformed = await Promise.all(
        ["01", "12a", "12\0tail"].map((subject, index) =>
          rejection(bindIdentity(db, `bad-account-${index}`, subject)),
        ),
      );
      const orphan = await rejection(db.prepare(
        "INSERT INTO auth_identities VALUES ('github', '10', 'missing-account')",
      ).run());
      const sameAccount = await rejection(db.prepare(
        "INSERT INTO auth_identities SELECT 'github', '11', account_id FROM auth_identities WHERE subject = '9007199254740992'",
      ).run());
      const { results: associations } = await db.prepare(
        "SELECT subject, account_id FROM auth_identities ORDER BY subject",
      ).all();
      const accounts = await db.prepare("SELECT COUNT(*) AS count FROM auth_accounts").first("count");
      return Response.json({ outcomes, malformed, orphan, sameAccount, associations, accounts });
    } catch (error) {
      return Response.json({ error: String(error) }, { status: 500 });
    }
  },
};
