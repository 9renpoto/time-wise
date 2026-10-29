# Rust Worker foundation

This package exposes `GET /v1/auth/config` with protocol version, canonical
HTTPS authentication origin and enabled client profile IDs. Configuration uses
`AUTH_ORIGIN` and a JSON array in `CLIENT_PROFILE_IDS`. Origins must be canonical
HTTPS origins (an optional root slash is accepted); credentials, paths, queries,
fragments and parser-normalized alternate spellings are rejected. Profile IDs
are unique ASCII letters/digits/dot/underscore/hyphen, 1–64 characters, at most 16.
Invalid configuration returns a generic 503 without echoing values.

All other authentication routes return 503. No provider requests, credentials,
D1 writes or real login are implemented. The checked-in Wrangler configuration
uses synthetic values for local verification, not an official deployment.

## Local verification

Use the workspace Rust toolchain and install the pinned build tool:

```sh
cargo install worker-build --version 0.8.7 --locked
cargo test -p time-wise-server --locked
cargo clippy -p time-wise-server --all-targets --locked -- -D warnings
sh apps/server/scripts/build.sh
python3 apps/server/scripts/smoke.py
```

The build script requires worker-build 0.8.7 and generates ignored JS/Wasm in
`apps/server/build/`. The entry point is `build/index.js`; the SDK's
`build/worker/shim.mjs` is a compatibility alias, not the direct bundle.
`no_bundle` prevents a second bundling step in celld/Wrangler.
Use Rustup proxies on PATH; the build script resolves the repository-pinned
compiler explicitly. `WORKER_BUILD` may point to that exact tool installed in a
custom directory. The smoke test builds the Worker, copies only its generated
bundle and synthetic configuration to a temporary project, starts the pinned
celld Docker image on an ephemeral localhost port, verifies HTTP behavior and
headers, then restarts with invalid configuration and verifies fail-closed
responses. It removes its own containers and temporary project on completion;
it does not change an existing celld instance or deployment.

Docker must be running. Initial build/image downloads require network access.
Do not place real provider secrets in the local synthetic configuration.
Self-host deployment is tracked in #278 and official DNS/HTTPS acceptance in
#279. Neither is established by local smoke success.

<details>
<summary>日本語</summary>

# Rust Worker基盤

GET /v1/auth/configでprotocol version・正規HTTPS origin・有効profile IDを返す。
AUTH_ORIGINとCLIENT_PROFILE_IDS（JSON配列）を設定する。originは正規形で末尾root slashのみ許可し、
user情報・path・query・fragment・parserが補正する別表記を拒否する。profileは一意の英数字・dot・
underscore・hyphenから1〜64文字、最大16件。不正設定は値を表示せず503にする。
他の認証経路は503のまま。provider通信・資格情報・D1書込み・実loginは未実装。
Wrangler設定は合成値によるlocal検証専用で、公式配置ではない。

上記commandで固定worker-build 0.8.7を導入し、workspace toolchainで単体/lint/Wasm/HTTPを検証する。
WORKER_BUILDで固定版toolのpathを指定可能。生成JS/Wasmはignoredなapps/server/buildへ出力する。
smokeは生成物と合成設定だけを一時directoryへコピーし、隔離celldをlocalhostの一時portで起動する。
HTTP・headerと、不正設定で再起動後の拒否を検証し、自分のcontainerと一時directoryを削除する。
既存celldや配置を変更しない。Docker起動と初回download用networkが必要で、実provider秘密情報は使わない。
セルフホスト配置は#278、公式DNS/HTTPSは#279で別途検証する。

</details>
