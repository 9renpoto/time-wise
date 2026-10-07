# 認証プロバイダー比較検証 / Authentication provider comparison

- 状態 / Status: 検証中 / In progress
- 更新日 / Updated: 2026-09-08
- 対象 / Scope: Rust on Cloudflare Workers
- 前提 / Based on: [ADR 0003](../adr/0003-passkey-account-authentication.md)

## 目的と対象外 / Purpose and non-goals

Cloudflare Workers上でRustを動かす制約を維持し、人が管理する外部
アカウントによる認証を実現できるか比較する。候補は、Time WiseのWorkerが
Google OIDCを直接扱う方式と、Supabase AuthへGoogle認証を委譲する方式である。
同期データはどちらの方式でもCloudflare側に置き、Supabaseのデータベースへ
移さない。

この文書では最終方式を採用しない。本番設定、秘密情報、公開デプロイ、
デスクトップへの認証結果引き渡し、同期ドメインの実装も対象外とする。
外部アカウントを操作できることは認証できるが、一人一アカウントであることや
Botではないことまでは証明しない。

This investigation keeps Rust on Cloudflare Workers and compares two ways to
authenticate control of an external account: direct Google OIDC in the Time
Wise Worker, and Google authentication delegated to Supabase Auth. In either
case, synchronization data remains on Cloudflare rather than moving to the
Supabase database.

This document does not select a final approach. Production configuration,
secrets, public deployment, desktop handoff and synchronization-domain
implementation are out of scope. Control of an external account authenticates
that account; it does not prove one-account-per-person or resistance to bots.

## 共通の合格条件 / Common acceptance criteria

- Workers用Wasmとしてビルドでき、ローカルのWorkers実行環境で動作する。
- Authorization Code Flow with PKCEを使用し、開始時に十分な乱数からS256
  challengeを生成できる。
- 直接OIDCでは`state`と`nonce`も生成し、callbackで照合する設計にできる。
- HTTPSの固定・設定済みprovider URLだけを使用し、リクエスト入力を任意の
  discovery/JWKS取得先にしない。
- issuer、audience、期限、署名、nonceなど必要なID token claimを検証し、
  改ざんされたtokenを拒否できる。
- providerのaccess tokenとrefresh tokenを同期APIの長期資格情報として
  使用しない。Time Wise独自sessionとデスクトップへの一回限りhandoffは
  後続設計とする。
- 秘密情報、authorization code、PKCE verifier、tokenをログやURLへ露出せず、
  リポジトリへcommitしない。

## Google OIDCを直接扱う方式 / Direct Google OIDC

### 検証結果 / Result

Rustの`openidconnect` 4.0.1を既定機能なし、`reqwest`機能ありで使用した。
`wasm32-unknown-unknown`向けcheckと`worker-build --release`に成功し、
Wrangler 4.129.1のローカルWorkers実行環境で次を確認した。

- `openid` scopeだけの認可URLにPKCE、state、nonceが生成される。
- Googleの固定issuerからdiscovery documentを取得し、issuer、authorization
  endpoint、token endpoint、JWKS URLを期待値と照合できる。
- 固定fixtureのRS256 ID tokenを公開鍵で検証できる。
- 署名を変更したID tokenを拒否できる。

### 残る実装条件 / Remaining conditions

Wasm版`reqwest`はredirect policyを設定できない。現在のprobeはGoogleの固定
issuerだけを取得し、返されたendpointを完全一致で検査しているが、本番では
Cloudflare Worker Fetchを使うHTTP client adapterを作り、discovery、JWKS、
token endpointへのredirectを拒否する。実際のGoogle client ID、callback、
authorization code exchangeとGoogle発行tokenの検証も未実施である。

アカウントの外部識別子はメールアドレスではなく、検証済みの`(issuer,
subject)`を使う。メールやprofile scopeは、別の製品要件が確定しない限り要求
しない。

## Supabase Authへ委譲する方式 / Supabase Auth

### 検証結果 / Result

Supabase専用Rust SDKには依存せず、Supabase AuthのHTTP APIと公開JWKSを
Workers上のRustから利用する構成とした。現時点では次のコードが
`wasm32-unknown-unknown`向けにbuildできる。

- 設定済み`SUPABASE_URL`からGoogle providerの`/auth/v1/authorize` URLを
  組み立て、`openid` scopeとS256 PKCE challengeを付与する。
- 設定済み`AUTH_CALLBACK_URL`だけをcallbackとして使用する。
- `/auth/v1/.well-known/jwks.json`を取得し、OIDCの公開鍵集合として解析する。
- 設定不足を、設定値を応答へ含めずHTTP 503として扱う。

実Supabaseプロジェクトが未設定のため、JWKS取得、Google画面への遷移、
authorization code exchange、Supabase発行JWTのclaim検証は未確認である。

### 残る実装条件 / Remaining conditions

検証には、利用者側で用意するSupabaseテストプロジェクトURL、publishable key、
Supabaseに設定したGoogle OAuth client ID/secret、許可済みHTTPS callbackが必要に
なる。開始時のPKCE verifierと相関状態は、callbackまで一回限り・期限付きで
サーバー側へ保持する。これを実装する前に、実ブラウザーを外部へredirectしない。

Supabase Authは認証用ユーザーとidentityをSupabase側に保持し、同じ検証済み
メールアドレスを持つidentityを自動linkする挙動がある。このデータ所有境界と
link規則が、個人情報を最小化するTime Wiseの要件に適合するかを選定前に判断する。

## 比較 / Comparison

| 観点 / Concern | Google OIDC直接 / Direct | Supabase Auth |
| --- | --- | --- |
| Rust Workers互換性 | 主要primitiveと外部discoveryを実行確認済み | URL/JWKSコードはbuild済み、実projectで未実行 |
| 認証データの所有 | Time Wiseが`issuer`と`subject`、sessionを設計 | Supabaseがauth user、identity、sessionを保持 |
| 個人情報の最小化 | `openid` scopeだけを要求できる | Google設定とidentity linkingの扱いを要確認 |
| 実装責任 | OIDC検証、session、失効、provider追加を自前実装 | OAuth連携とsession管理の一部を委譲できる |
| 依存 | `openidconnect`とWorkers用HTTP adapter | Supabase Auth HTTP API、Supabaseプロジェクト |
| Cloudflare完結 | 認証バックエンドをCloudflare内に保てる | 認証情報はSupabaseにも保存される |
| provider追加 | providerごとに設定・検証が必要 | Supabaseの対応providerを利用できる |

## 選定前に残る条件 / Decision gates

1. SupabaseテストプロジェクトでJWKS取得からGoogle login、code exchange、JWT
   検証まで通し、実行結果を記録する。
2. 直接Google方式でも実clientを用いたcode exchangeを確認する。
3. 両方式でcallback stateの一回性、期限切れ、並行login、再使用拒否を同じ
   条件で検証する。
4. Supabase側に保存されるユーザーデータとidentity linkingを許容するか決める。
5. provider障害・鍵rotation・アカウント無効化時のTime Wise session失効方針を
   比較する。
6. 比較結果を受けてADR 0003と既存タスクを更新し、その後に方式を採用する。

## 再現方法 / Reproduction

具体的なbuild、ローカル実行、probe endpoint、必要な環境変数は
[`apps/server/README.md`](../../apps/server/README.md)に記載する。外部サービスを
使わない自動検証はCIへ追加できるが、実providerの秘密情報を必要とする検証は
制限した検証環境で実施し、秘密情報をログやfixtureへ保存しない。

Primary references: [Google OpenID Connect](https://developers.google.com/identity/openid-connect/openid-connect),
[Supabase Auth](https://supabase.com/docs/guides/auth),
[Supabase PKCE flow](https://supabase.com/docs/guides/auth/sessions/pkce-flow),
[Supabase JWT signing keys](https://supabase.com/docs/guides/auth/signing-keys), and
[`openidconnect` crate documentation](https://docs.rs/openidconnect/4.0.1/openidconnect/).
