export default {
  async fetch(request, { AUTH_DB: db }) {
    if (new URL(request.url).pathname === "/ready") return Response.json({ ready: true });
    try {
      return Response.json(await verify(db, migrations));
    } catch (error) {
      return Response.json({ error: String(error) }, { status: 500 });
    }
  },
};
