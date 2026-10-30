# Deploy the authentication foundation to Cloudflare

This deploys the Rust Worker and a deployment-owned D1 binding. Authentication
currently returns `503 authentication_unavailable`: GitHub login, authentication
tables, desktop handoff and sessions are implemented in later tasks. The button
starts setup; a successful build does not prove that login or HTTPS acceptance works.

[![Deploy to Cloudflare](https://deploy.workers.cloudflare.com/button)](https://deploy.workers.cloudflare.com/?url=https%3A%2F%2Fgithub.com%2F9renpoto%2Ftime-wise)

## Deploy with the button

1. Open the button and create your own repository and Worker in your Cloudflare
   account. Keep the repository root as the build root. `apps/server` depends on
   the root Cargo workspace, Cargo.lock and rust-toolchain.toml; copying only that
   subdirectory cannot build this project.
2. Configure `AUTH_ORIGIN` as your canonical HTTPS origin (for example,
   `https://auth.example.org`), without credentials, a path, query or fragment.
   Replace the example origin in `wrangler.json`. Configure a JSON array in
   `CLIENT_PROFILE_IDS`, enabling only profiles supported by your desktop build.
   These are runtime Worker variables, not build secrets. Preserve the resulting
   configuration in your repository so redeployments use the same values.
3. Keep `AUTH_DB` as the binding name. Cloudflare provisions D1 and replaces the
   template's database ID. Inspect the resulting `wrangler.json`; preserve the
   provisioned ID and database on subsequent runs. Do not create another database
   or restore a template ID when updating an existing deployment.
4. Accept `npm run build` and `npm run deploy` as the build/deploy commands. npm
   installs locked Wrangler; the build bootstraps rustup if absent and resolves the
   compiler from rust-toolchain.toml. worker-build follows the locked Worker SDK.
   The build needs network access to the official Rust distribution and crates.
5. Configure your HTTPS hostname using a Worker custom domain, or select the
   assigned workers.dev hostname as `AUTH_ORIGIN`. If the assigned name becomes
   known only after initial setup, update the origin and redeploy. This template
   enables workers.dev and disables preview URLs and observability logs. A custom
   domain and its DNS/TLS are configured separately; disable workers.dev if your
   deployment must only be reachable through the custom domain.
6. Verify discovery at the exact configured origin as described below. Save the
   configuration changes in the generated repository before subsequent builds.

Cloudflare supports [automatic resource provisioning and deploy commands](https://developers.cloudflare.com/workers/platform/deploy-buttons/).
Use the full-repository button: Cloudflare copies a specified subdirectory as the
new repository root, so workspace dependencies outside it would be lost.

## Deploy manually or redeploy an existing installation

Use Node.js 24 LTS, npm, curl and a native C linker/build toolchain. On Debian/Ubuntu,
install `build-essential pkg-config libssl-dev ca-certificates curl` first;
worker-build's native dependencies need OpenSSL development headers. Cloudflare's
[build image](https://developers.cloudflare.com/workers/ci-cd/builds/build-image/)
includes native build tools and OpenSSL development packages. No desktop
WebKit/Tauri dependencies are needed to build this Worker. The bootstrap installs
only the pinned Rust compiler and Wasm target when missing. Run from the repository
root, after cloning your own repository:

```sh
npm ci
npm run check:deployment
npx wrangler login
```

For a **new** installation only, create D1 and record its returned ID in the
`AUTH_DB` entry of `wrangler.json`:

```sh
npx wrangler d1 create time-wise-auth
```

For an existing installation, use its existing database ID. Update the origin,
profile IDs and Worker name in `wrangler.json`, then run:

```sh
npm run deploy:dry-run
npm run deploy
```

`check:deployment` checks the template without credentials or remote commands;
a template ID is allowed at this stage. `deploy` requires a provisioned, nonzero
UUID and validates the origin/profile syntax before invoking Wrangler. It applies
regular `.sql` files from `apps/server/migrations` to **AUTH_DB**, then deploys.
There are currently no authentication migrations (#195); an empty migration
directory is skipped. No database reset/delete/create command exists in the runner.
Wrangler records applied migrations, so an unchanged set is safe to apply again.
A migration failure stops the Worker deployment. Review future migrations for
compatibility with the currently deployed Worker; deployment is not a rollback
of database changes. Back up existing data before schema changes.

`deploy:dry-run` builds and packages JS/Wasm locally without applying migrations or
changing Cloudflare resources. It does not validate remote account permissions,
D1 ownership, GitHub App registration, DNS or TLS. Authentication stays disabled
even when every configuration field is set; there is no activation flag in this
foundation. Do not treat template validation as authorization to enable login.

## Prepare the identity-only GitHub App

Before provider implementation is activated, create a separate GitHub App for each
independent deployment/environment. Set its callback to the exact configured
origin followed by `/auth/github/callback`. Disable webhook delivery and do not
request repository/organization permissions or repository installation for login.
Use the authorization-code flow with state and PKCE S256, as specified in ADR 0005.

The App client ID is configuration; its client secret is a **runtime Worker secret**.
The authentication restore generation must also live outside D1 (ADR 0008).
Their binding names and consuming checks will be introduced with the provider and
persistence implementations. This foundation consumes neither, so the button does
not request or generate credentials that it cannot yet validate. Do not add secrets
to `wrangler.json`, build variables, source files or logs. Once the consuming
bindings exist, set them through Cloudflare's Worker secret UI or interactive
`wrangler secret put`; do not put values on command lines or print them in evidence.
A D1 restore will require changing the external generation and reauthentication.

Before enabling real authentication, require validated schema/version, runtime
secrets, GitHub App/callback registration, enabled profile catalog, HTTPS origin
and restore generation. Implement those gates in the owning authentication tasks;
this deployment template deliberately cannot enable provider traffic.

## Verify each boundary

- **Local:** Rust tests, `npm run check:deployment`, `npm run deploy:dry-run` and
  celld smoke checks validate configuration/build/runtime behavior. Repeat the
  packaging command to check reproducibility without changing cloud data.
- **CI:** the Ubuntu job packages the same locked Wrangler/SDK combination and
  runs the existing instrumented Rust tests. A passing check is packaging evidence.
- **Cloud deployment:** record the deployed revision, Worker and D1 binding names
  and successful deployment result. Inspect only secret names/presence.
- **HTTPS (#279):** request `GET /v1/auth/config` at the configured origin and check
  status 200, `Cache-Control: no-store`, protocol version 1, matching `auth_origin`
  and the intended `client_profile_ids`. Check a login attempt returns 503 and no
  provider redirect. Separately verify DNS/TLS and the App callback registration.
- **Login acceptance:** real provider, session and physical desktop tests belong
  to their implementation issues and #203. Discovery success does not complete them.

Forked desktop builds may choose their own build-time callback scheme/path and
profile definition (ADR 0010). Changing `AUTH_ORIGIN` does not change a desktop
callback URI. Official-app connection selection and the shared profile catalog are
implemented separately; do not advertise a working desktop login at this stage.

<details>
<summary>日本語</summary>

## Cloudflareへの配置

この手順はRust Workerと運用者所有のD1 bindingを配置する。認証は現時点では503で、
GitHub login・認証table・desktop引き渡し・sessionは後続taskで実装する。
button成功はloginやHTTPS検証の完了を意味しない。

buttonから自分のrepositoryとWorkerを作り、build rootはrepository全体にする。
apps/server単独ではrootのworkspace・Cargo.lock・toolchain設定が欠ける。
AUTH_ORIGINを自分のHTTPS originへ変更し、path/query/fragmentや資格情報を含めない。
CLIENT_PROFILE_IDSはdesktop buildが対応するIDのJSON配列にする。
runtime変数として設定し、再配置でも維持できるようwrangler.jsonへ保存する。
AUTH_DBというbinding名を維持し、Cloudflareが作成したD1 IDを確認・保存する。
既存環境の更新時にDBを作り直したりtemplate IDへ戻したりしない。

buildはnpm run build、deployはnpm run deployを使う。
Rust未導入ならrustupをbootstrapし、repositoryの固定toolchainとSDK対応toolを使う。
公式Rust配布元・cratesへのnetwork接続とnative C linkerが必要。
custom domain・DNS・TLSを設定するか、割り当てられたworkers.devをoriginとして使う。
初回配置後にhostnameが分かった場合はoriginを更新して再配置する。
preview URLとobservabilityは無効。custom domainのみで公開するならworkers.devも無効にする。

## 手動配置と再配置

Node.js 24 LTS・npm・curl・C build toolchainを用意する。Debian/Ubuntuでは
build-essential pkg-config libssl-dev ca-certificates curlを導入する。
worker-buildのnative依存にOpenSSL開発headerが必要。desktop用依存は不要。
repository rootでnpm ci、npm run check:deployment、npx wrangler loginを実行する。
新規環境だけnpx wrangler d1 create time-wise-authでDBを作り、返されたIDを設定する。
既存環境は既存IDを使う。origin/profile/Worker名を設定後、npm run deploy:dry-run、
npm run deployを実行する。checkは資格情報不要でtemplate IDを許すが、deployは実IDを要求する。
SQLがあればAUTH_DBへmigrationを適用してからWorkerを配置し、失敗なら停止する。
現在は#195のschema実装前なのでSQLはなくskipする。runnerにDB reset/delete/createはない。
適用済みmigrationはWranglerが記録する。将来のmigrationは稼働Workerとの互換性を確認し、
schema変更前にbackupする。DB変更はWorker配置の失敗やrollbackでは取り消されない。

## GitHub Appと秘密情報

独立した配置・環境ごとに認証専用GitHub Appを登録し、callbackをorigin配下の
/auth/github/callbackへ設定する。webhookを無効にし、repository/organization権限や
installationをlogin条件にしない。ADR 0005のcode/state/PKCE S256に従う。
client secretはruntime Worker secret、restore generationはD1外へ保存する。
これらのbinding名・検証はprovider/保存実装時に導入する。未使用の秘密をこのbuttonで
生成・要求しない。source・wrangler.json・build変数・logへ秘密を置かず、実装後に
Cloudflareのsecret UIまたは対話的secret putで登録する。証拠には存在だけを記録する。
DB復元時は外部generation変更と再認証が必要。
schema・secret・App登録・profile catalog・HTTPS・generationの全検証が有効化条件になる。
現時点には認証の有効化flagはなく、設定済みでもprovider通信は始まらない。

## 検証の境界

localはRust test・設定check・dry-run・celld、CIは同じlocked依存によるpackage検証。
dry-runを繰り返してもcloud dataは変更しない。cloud配置はrevision・Worker/D1 bindingと
成功結果、HTTPSは#279でconfigの200/no-store/version/origin/profileとloginの503を確認する。
DNS/TLS・App callback登録は別途確認する。実login/session/実機受け入れは後続task/#203。
forkのdesktop callback scheme/pathはbuild時に任意設定でき、server originとは別の設定。
公式appの接続先選択と共有profile catalogは別途実装する。

</details>
