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

Use the workspace Rust toolchain. The build script prepares the matching tool automatically:

```sh
cargo test -p time-wise-server --locked
cargo clippy -p time-wise-server --all-targets --locked -- -D warnings
sh apps/server/scripts/build.sh
cargo test -p time-wise-server --test smoke worker_discovery -- --ignored
```

The build script resolves the server's Wasm SDK from locked Cargo metadata,
installs the same worker-build version into ignored `target/worker-tools` when
needed, and generates ignored JS/Wasm in
`apps/server/build/`. The entry point is `build/index.js`; the SDK's
`build/worker/shim.mjs` is a compatibility alias, not the direct bundle.
`no_bundle` prevents a second bundling step in celld/Wrangler.
Use Rustup proxies on PATH; the build script resolves the repository-pinned
compiler explicitly. `WORKER_BUILD` may point to a matching-version tool in a
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

build scriptがCargo.lockに従うWasm SDK版を解決し、同じworker-buildを必要ならtarget/worker-toolsへ自動導入する。
workspace toolchainで単体/lint/Wasm/HTTPを検証する。
WORKER_BUILDで対応版toolのpathを指定できるが、SDK版との不一致は拒否する。生成JS/Wasmはignoredなapps/server/buildへ出力する。
smokeは生成物と合成設定だけを一時directoryへコピーし、隔離celldをlocalhostの一時portで起動する。
HTTP・headerと、不正設定で再起動後の拒否を検証し、自分のcontainerと一時directoryを削除する。
既存celldや配置を変更しない。Docker起動と初回download用networkが必要で、実provider秘密情報は使わない。
セルフホスト配置は#278、公式DNS/HTTPSは#279で別途検証する。

</details>

## Smoke harness coverage

The Rust integration tests live in `apps/server/tests/smoke.rs`, with shared
subprocess/HTTP/cleanup support. Docker tests are ignored in ordinary workspace
runs and executed explicitly by CI before grcov collection. Both normal and
ignored test executions use Rust coverage instrumentation and feed the existing
LCOV/Octocov report. This measures the native harness, not Rust/Wasm running
inside celld. Child Wasm builds remove coverage instrumentation and use a
separate target directory. No Python runtime is required.

<details>
<summary>日本語</summary>

Rust統合テストはapps/server/tests/smoke.rsに置き、process/HTTP/cleanupを共有する。
Docker検証は通常のworkspace実行ではignoreし、CIでgrcov収集前に明示実行する。
通常・ignore双方の実行を計測し、既存LCOV/Octocovへ集約する。計測対象はnativeハーネスで、
celld内のRust/Wasmとは別。子Wasm buildは計測flagを除去し独立target directoryを使う。
Python runtimeは不要。

</details>

The smoke harness runs celld with the temporary directory owner's UID/GID on
Unix hosts (`CELLD_UID`/`CELLD_GID`), so Linux bind mounts do not leave root-owned
state that prevents cleanup. Interactive Compose defaults remain unchanged.

<details>
<summary>日本語</summary>

Unixのsmokeでは一時directoryのUID/GIDをCELLD_UID/CELLD_GIDで渡し、Linuxのbind mountに
root所有dataを残さず削除できるようにする。通常のCompose実行の既定値は変更しない。

</details>

## Dependency updates

Cargo.lock is the version source for the SDK and its matching build tool.
Dependabot's Cargo `rustwasm` group includes `worker`, its support crates and
Wasm bindings; an SDK update PR automatically builds and tests the corresponding
worker-build version through the smoke step. There are no CLI version literals
in CI or the build script to update separately. Do not run `cargo update` during
normal builds; committed lockfile changes remain reviewable.

This policy deliberately tracks SDK/tool pairs, not independent CLI-only
releases. A missing or incompatible matching tool fails the update's build/CI;
it never silently falls back to another global installation. Actual Dependabot
PR creation and hosted CI results are checked after configuration reaches main.

<details>
<summary>日本語</summary>

SDKと対応build toolの版はCargo.lockを基準にする。DependabotのCargo rustwasm groupへworker・
関連crate・Wasm bindingを含め、SDK更新PRで対応worker-buildのbuild/smokeも検証する。
CIやshellの版番号を別更新せず、通常buildでcargo updateは行わない。
SDK/toolの組を追跡し、CLI単独releaseの独立監視はしない。対応版がなければ更新CIを失敗させ、
globalの別版へ暗黙fallbackしない。Dependabot PRの実生成とhosted CIはmain反映後に確認する。

</details>
