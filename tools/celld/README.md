# Local Cloudflare runtime

Run celld locally alongside the desktop application using Docker Compose.
The official v0.6.3 image is pinned by digest in `compose.celld.yaml`; there is
no native Intel macOS release. The prerequisites are a POSIX shell, Docker with
Compose, rustup with the repository's toolchain and Wasm target, for the Rust integration tests.

From the repository root:

```sh
sh tools/celld/build.sh
docker compose -f compose.celld.yaml up -d
curl --fail http://127.0.0.1:9876/
curl --fail -X POST http://127.0.0.1:9876/counter
docker compose -f compose.celld.yaml logs -f
```

The root responds with `{"runtime":"celld","wasm":42}` after calling the
Rust-generated Wasm module. Each `POST /counter` increments a synthetic
Durable Object counter. These are development probes, not application API
contracts. No GitHub, Cloudflare, or object-storage credentials are needed.
The listener is published only on host loopback. Set `CELLD_PORT` before
Compose commands if port 9876 is occupied.

celld watches the fixture for changes. After changing `probe.rs`, rerun the
build script to regenerate its Wasm file. `no_bundle` loads the single Worker
module and Wasm without installing Node.js or esbuild. This does not yet
exercise a workers-rs shim or the production server package.

Stop the environment with:

```sh
docker compose -f compose.celld.yaml down
```

State remains in `tools/celld/fixture/.celld/dev` across stops and container
recreation. It is ignored by Git, together with the generated Wasm and local
environment files. The fixture directory is the only host source directory
mounted into the container; do not copy desktop history or credentials there.
Changing a fixture schema does not migrate existing state.

Run the integration test with:

```sh
cargo test -p time-wise-server --test smoke fixture_wasm -- --ignored
```

The test builds the Rust probe, copies the fixture into a temporary directory,
starts a separate Compose project on an automatically allocated loopback port,
and checks HTTP/Wasm, automatic source reload, and counter persistence after
removing and recreating the container. Local environment files are excluded
from the copy. It stops its own containers and removes its temporary data at exit.
It does not reset the running development instance.

Local results do not establish Cloudflare edge behavior, HTTPS deployment,
production storage guarantees, or GitHub authentication success. See
[ADR 0006](../../docs/adr/0006-celld-local-runtime.md) and
[celld compatibility](https://celld.dev/docs/cloudflare-compat/). Authentication
contracts remain tracked in #251 and #193; #265 tracks this local environment.

<details>
<summary>日本語</summary>

## ローカル実行

Docker Composeでcelldをdesktopと並行して起動する。公式v0.6.3イメージをdigestで
固定する。Intel macOS向けのnative配布はない。POSIX shell、DockerとCompose、リポジトリ指定の
Rust toolchain・Wasm target、結合テスト用のRust/Cargoが必要。

リポジトリrootから上記のbuild・起動コマンドを実行する。`GET /`はRust生成Wasmを
呼び出して`{"runtime":"celld","wasm":42}`を返し、`POST /counter`は合成データの
Durable Object counterを増やす。これらは開発probeでありアプリAPI契約ではない。
GitHub・Cloudflare・object storageの資格情報は不要。公開先はホストのloopbackだけ。
portが使用中ならCompose実行時に`CELLD_PORT`を指定する。

fixture変更はcelldが監視する。Rustコード変更後はbuild scriptを再実行する。
`no_bundle`で単一WorkerとWasmを読み込み、Node.js・esbuildは不要にする。
workers-rsのshimやproduction serverの検証はまだ行わない。

停止には上記の`down`を使う。状態は`tools/celld/fixture/.celld/dev`に残り、
停止・コンテナ再作成でも保持される。生成Wasm・ローカル環境変数ファイルとともに
Git対象外。containerにmountするhost sourceはfixtureだけとし、desktop履歴や
資格情報を置かない。schema変更時の既存状態の移行は自動では行われない。

結合テストは上記のCargoコマンドで実行する。Rust probeをbuildしてfixtureを
一時ディレクトリへコピーし、独立したCompose projectと自動割当loopback portで
HTTP・Wasm・source自動反映・コンテナ再作成後の状態保持を検証する。
ローカル環境変数ファイルはコピーから除外する。終了時はテスト用containerと
一時データだけを片付け、稼働中の開発instanceをresetしない。

local成功はCloudflare edge・HTTPS配置・production保存保証・GitHub認証成功を
意味しない。ADR 0006と公式互換性文書を参照する。認証契約は#251・#193、
ローカル環境の導入は#265で追跡する。

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
