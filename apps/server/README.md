# Workers compatibility spike

This directory contains the minimal Rust Worker used by issue #192.

## Successful build check

```sh
cargo check -p time-wise-server --target wasm32-unknown-unknown
```

The server crate builds as a `cdylib` for `wasm32-unknown-unknown` and uses
`workers-rs` version 0.8.5. Its local HTTP response confirms that the Workers
Rust toolchain and generated bundle run successfully.

## WebAuthn result

`webauthn-rs` 0.5.5 was evaluated with default features disabled. Its Workers
Wasm build failed because its dependency graph includes `openssl-sys`, and it
also requires Wasm-specific randomness configuration through `getrandom` and
`uuid`. The failure is recorded in issue #192.

Do not use that candidate for the Workers authentication implementation.
`passkey-auth` 0.1.3 compiles for Workers after enabling `getrandom`'s Wasm
`js` feature, but calling `start_registration` panics because the library uses
`std::time::SystemTime`, which `wasm32-unknown-unknown` does not implement.
The Worker returned HTTP 500 for both a registration-start request and a
deliberately invalid registration response. Do not use that candidate either.

No tested Workers-compatible WebAuthn verification library has been selected.
Keep the Worker runtime spike, but do not start issue #193 until this blocker
is resolved.

## OIDC provider comparison probes

The follow-up comparison keeps the Rust-on-Workers constraint while evaluating
direct Google OIDC and Supabase Auth. Build the Worker with:

```sh
worker-build apps/server --release
```

Then run the generated bundle locally:

```sh
npx wrangler dev apps/server/build/index.js --ip 127.0.0.1 --port 8787
```

The direct OIDC probes require no local secrets:

```sh
curl --fail http://127.0.0.1:8787/spike/oidc/start
curl --fail http://127.0.0.1:8787/spike/oidc/verify
curl --fail http://127.0.0.1:8787/spike/oidc/discovery
```

The Supabase probes read these Worker variables:

- `SUPABASE_URL`: the HTTPS project URL, for example
  `https://project-ref.supabase.co/`
- `AUTH_CALLBACK_URL`: an allowed HTTPS callback owned by the test deployment

`/spike/supabase/start` validates generation of the Google provider URL with
the `openid` scope and an S256 PKCE pair without returning the verifier.
`/spike/supabase/jwks` fetches
and parses the project's public signing keys. Missing configuration returns
HTTP 503 without echoing binding values.

These endpoints do not complete a login. Before redirecting a real browser,
the implementation must retain the PKCE verifier and correlation state in a
single-use, expiring server-side record. A configured Supabase project, Google
provider credentials, an allowed callback and that state store are required to
test authorization-code exchange. Do not commit those credentials.
