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
