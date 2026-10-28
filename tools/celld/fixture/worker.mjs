import probeModule from "./build/probe.wasm";

const { exports: probe } = new WebAssembly.Instance(probeModule);

export class Counter {
  constructor(state) {
    this.sql = state.storage.sql;
    this.sql.exec(
      "CREATE TABLE IF NOT EXISTS counter (id INTEGER PRIMARY KEY, value INTEGER NOT NULL)",
    );
  }

  fetch() {
    const row = this.sql
      .exec(
        "INSERT INTO counter (id, value) VALUES (1, 1) ON CONFLICT(id) DO UPDATE SET value = value + 1 RETURNING value",
      )
      .one();
    return Response.json(row);
  }
}

export default {
  fetch(request, env) {
    const path = new URL(request.url).pathname;
    if (request.method === "GET" && path === "/") {
      return Response.json({ runtime: "celld", wasm: probe.add(19, 23) });
    }
    if (request.method === "POST" && path === "/counter") {
      const id = env.COUNTER.idFromName("synthetic-probe");
      return env.COUNTER.get(id).fetch(request);
    }
    return new Response("Not found", { status: 404 });
  },
};
