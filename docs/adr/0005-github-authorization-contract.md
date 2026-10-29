# ADR 0005: GitHub authorization contract

- Status: Accepted (contract confirmed; implementation pending)
- Date: 2026-10-29
- Related: [ADR 0004](0004-github-account-authentication.md), issues #251 and #193

## Integrated contract resolution (#193)

The remaining design is delivered by [ADR 0008](0008-application-authentication-state.md),
[ADR 0009](0009-application-sessions.md), and [ADR 0010](0010-desktop-authentication-handoff.md).
ADR 0010 supersedes the outcome-only/no-link page restriction for its first-party
comparison/confirmation/return flow, and adds the reauthentication-only 409
`identity_mismatch` outcome. Its profile-based desktop return is separate from
the fixed per-deployment HTTPS GitHub callback. The remaining-decisions section
below records the original dependency, not an outstanding design blocker.

<details>
<summary>日本語</summary>

#193の残る設計はADR 0008・0009・0010で提供済み。ADR 0010はfirst-partyの照合・確認・復帰画面を
許可し、再本人確認用409 identity_mismatchを追加する。profileによるdesktop復帰と、配置ごとに固定する
GitHub HTTPS callbackは別物。下記の未決事項は当初の依存関係の記録で、現在の未解決blockerではない。

</details>

## App registration decision

Register a GitHub App for identity-only sign-in. Do not request additional
account or repository permissions, and do not require users to install the app
on an account or repository. Keep the application secret and provider callback
handling on Cloudflare, outside the desktop client.

GitHub supports user authorization without installation, and its authenticated
user endpoint accepts a GitHub App user access token without additional
permissions. This gives the selected Rust-on-Cloudflare architecture a login
boundary without requiring access to repository data. A dedicated OAuth App
was considered; the GitHub App registration follows GitHub's general
recommendation while keeping installation outside the sign-in experience.

This records the agreed contract for #251; repository delivery remains subject
to review. It does not register an app or establish a working login flow.
No deployed authentication data requires migration.

## Authorization and retention decisions

Use the external-browser authorization-code flow, with Cloudflare handling the
registered provider callback and token exchange. Require both a random `state`
and PKCE with `S256`. Keep the client secret and PKCE verifier on the server.
The HTTPS origin and application start/cancel/handoff APIs remain #193 decisions.

Identify the user through an authenticated `GET /user` on each login. Retain
only GitHub's numeric user ID as the external identity, and do not retain
login names, avatars, email addresses, or the full provider response. Do not
merge accounts based on an email match. A GitHub ID identifies the external
account; the application's own account ID and session remain separate.

Create an application account on the first verified login for a GitHub numeric
ID, and reuse its association on later logins. Use the fixed provider namespace
`github` for public GitHub and the numeric ID as the identity key. Never
automatically merge accounts or move an association to another GitHub ID.
#193 must provide atomic uniqueness when first logins race.

Disable webhook delivery for this authentication-only GitHub App. This
milestone does not promise immediate application-session revocation when the
user revokes GitHub authorization. #193 defines application-session expiry,
logout, and revocation; a future webhook integration needs its own decision.

Use provider access tokens only transiently during the callback. Do not persist
access or refresh tokens or send them to the desktop, logs, or redirect URLs.
Discard them on success and failure. Attempt individual-token revocation before
discarding an issued access token. Without retained provider credentials,
immediate detection of later GitHub access loss is not guaranteed; application
session expiry and revocation require #193.

## Callback lifetime and one-time processing

An authentication attempt accepts its provider callback for ten minutes from
creation, without extending the deadline. Reject a callback at or after the
deadline and require a new attempt.

Each attempt has independent correlation state and a PKCE verifier. Starting a
second attempt does not cancel the first, and cancelling an attempt affects
only that attempt. Never use one attempt's state or verifier for another.

Accept a callback for processing at most once, including callbacks that end in
provider denial or failure. A repeated callback must not exchange the code or
publish a second verified result. Missing, unknown, or mismatched correlation
must not consume or cancel another valid attempt. A process interruption must
not reopen an already accepted attempt.

These rules require an atomic claim of an unexpired pending attempt before
provider processing. #193 must define the storage, initiating-client and browser
binding, cancellation authorization and races, and recovery of interrupted
processing before implementation. State alone is not a desktop handoff proof.

## Provider-token cleanup

After identity verification, attempt to revoke only the issued user access
token with GitHub's individual-token endpoint. Do not revoke the user's entire
application grant or reset the token. Cleanup also applies when an issued token
could not complete identity verification; cleanup never substitutes for it.

A failed cleanup request does not undo successful identity verification.
Discard access and refresh tokens regardless of cleanup success, without a
persistent retry queue. Record only a sanitized cleanup outcome, without token
values, token fragments, or provider response bodies. A failed revocation can
leave a token valid at GitHub until its upstream expiry or revocation; local
discard does not establish upstream invalidation.

Do not assume deleting an access token invalidates an associated refresh token:
that cascade was not established by the reviewed documentation or live tests.
Discard any returned refresh token without using it. Keep GitHub App user-token
expiration enabled, and do not request `offline_access` for this identity-only
flow.

## Provider and callback interface

This provider contract is confirmed. Implementation also requires agreement on
the remaining storage, session, and handoff decisions in #193.

Generate independent 32-byte cryptographically random values for OAuth state
and the PKCE verifier, each encoded as 43 unpadded base64url characters. The
state size is an application policy. Calculate the S256 challenge as unpadded
base64url of SHA-256 over the ASCII verifier. Bind the transaction to its
initiating client and browser; a valid state lookup alone is insufficient.

| Operation | Exact provider request | Required data / accepted response |
| --- | --- | --- |
| Authorize | `GET https://github.com/login/oauth/authorize` | Configured `client_id`, fixed registered `redirect_uri`, `state`, `code_challenge`, `code_challenge_method=S256`; no permission scope or caller-supplied redirect. |
| Exchange | `POST https://github.com/login/oauth/access_token` | Send `client_id`, `client_secret`, `code`, the same `redirect_uri`, and `code_verifier` in the request body; request JSON. Require a nonempty bearer access token and reject an `error` response even with HTTP 200. |
| Identify | `GET https://api.github.com/user` | Bearer token, `Accept: application/vnd.github+json`, `X-GitHub-Api-Version: 2026-03-10`; require HTTP 200 and a positive integer `id` representable as `u64`. Ignore all other profile fields. |
| Cleanup | `DELETE https://api.github.com/applications/{client_id}/token` | Basic authentication using the client ID and secret; JSON body containing `access_token`. HTTP 204 establishes success; other responses or transport failure establish only cleanup failure. |

Use `GET /auth/github/callback` on the configured HTTPS authentication origin.
The exact environment origin and app registration are #193/deployment inputs;
they are never selected from a request parameter. Local celld HTTP probes do
not validate this HTTPS callback.

The query requires exactly one nonempty `state` and either one nonempty `code`
or one nonempty `error`, never both. Reject duplicate recognized parameters.
Ignore unrecognized parameters; never treat them as a destination or identity.
Ignore untrusted `error_description` and `error_uri` content. Validate syntax,
correlation, and initiating-browser binding before the atomic attempt claim.

| Callback result | HTTP status | Stable outcome |
| --- | --- | --- |
| Identity verified and bound to the application account | 200 | `verified` |
| GitHub returned `access_denied` for a valid attempt | 200 | `cancelled` |
| Malformed callback or invalid correlation/browser binding | 400 | `invalid_callback` |
| The attempt has already been claimed or completed | 409 | `attempt_consumed` |
| The validly correlated attempt expired | 410 | `attempt_expired` |
| The validly correlated attempt was cancelled | 410 | `attempt_cancelled` |
| Token exchange or identity verification failed | 502 | `provider_failure` |
| Required application storage is unavailable | 503 | `authentication_unavailable` |

Return a minimal HTML result containing only the stable outcome and a static
message. Never include the code, state, token, user ID, or provider error text.
Send `Cache-Control: no-store` and `Referrer-Policy: no-referrer`; include no
third-party resources or external links. Redact callback query values from
application, ingress, tracing, and error logs. Provider tokens never appear in
URLs. Browser-history leakage and the exact desktop return flow require #193.

Within the backend, the verified-identity result has this shape, using a
canonical decimal subject string to avoid JSON number precision loss:

```json
{"attempt_id":"synthetic-attempt","provider":"github","subject":"1001"}
```

The attempt ID is server-issued and binds the result to the accepted attempt.
This result stays within the trusted authentication coordinator. It is not a
browser response, application session, or desktop handoff credential. Account
creation, association uniqueness, and publishing a result must respect #193's
atomicity and cancellation rules; never report `verified` after a storage
failure or a cancellation that won the race.

Bound provider requests by an implementation timeout. Do not automatically
resubmit an authorization code after an ambiguous exchange failure. A failed
attempt requires a new attempt; an outage or failed login does not by itself
revoke existing application sessions. Cleanup failure is the explicit exception
that can follow successful identity verification without failing login.

## Synthetic validation cases

| Case | Required result |
| --- | --- |
| First verified ID `1001`, then the same ID with changed profile fields | One application account; retain only the provider/ID association. |
| Two first logins for the same ID race | One account association, enforced by #193 storage. |
| Two attempts complete in reverse order, or one is cancelled | State/verifier/result stay isolated; cancellation affects only its target. |
| Callback accepted at 599 seconds, or received at exactly 600 seconds | Accept the former; expire the latter without code exchange. |
| Duplicate or concurrent callback for one valid attempt | At most one provider-processing claim and one verified result. |
| Missing/unknown/swapped state, wrong browser, duplicate fields, or both code and error | Reject before provider processing; do not consume another valid attempt. |
| GitHub denial, expired code, HTTP 200 with an error, or malformed user ID | Never create an authenticated result; denial is cancellation. |
| Timeout, rate limit, or provider 5xx during identity verification | Fail the current attempt; no automatic code resubmission or session revocation. |
| Cleanup returns 204, fails, or times out after verified identity | Dispose of all provider secrets; only cleanup outcome differs. |
| Cancellation, process interruption, or storage failure races publication | No reopened attempt, duplicate result, or verified result after a winning cancellation/failure. |
| Inspect browser responses and all log/trace sinks | No provider secrets, callback query values, profile payloads, or user IDs. |

## Verified provider constraints

Checked against GitHub documentation on 2026-10-29:

- The authorization request uses the GitHub App client ID, which differs from
  its app ID. Code exchange requires the client secret.
- GitHub supports PKCE `S256`, not `plain`. The provider redirect must match a
  registered callback and cannot add arbitrary parameters.
- `GET /user` accepts a GitHub App user token without additional permissions.
  Its durable numeric ID is suitable for binding; a login name can change.

The application lifetime and independence rules above are application decisions,
not guarantees supplied by GitHub. Desktop handoff remains a separate contract.

## Remaining decisions in #193

#193 owns storage guarantees, browser/client binding mechanisms, cancellation
authorization and races, operational timeouts, application account persistence,
session lifecycle, deployment origins, and desktop handoff. Its application
start/cancel/poll/claim API is separate from this provider callback contract.
Do not start authentication implementation while the #193 contract is unresolved.

## References

- [GitHub App user authorization without installation](https://docs.github.com/en/apps/creating-github-apps/authenticating-with-a-github-app/authenticating-with-a-github-app-on-behalf-of-a-user)
- [Get the authenticated user](https://docs.github.com/en/rest/users/users#get-the-authenticated-user)
- [GitHub App authorization-code flow](https://docs.github.com/en/apps/creating-github-apps/authenticating-with-a-github-app/generating-a-user-access-token-for-a-github-app)
- [GitHub App security practices](https://docs.github.com/en/apps/creating-github-apps/about-creating-github-apps/best-practices-for-creating-a-github-app)
- [Delete an app token](https://docs.github.com/en/rest/apps/oauth-applications#delete-an-app-token)
- [Authentication-only app registration](https://docs.github.com/en/apps/creating-github-apps/registering-a-github-app/registering-a-github-app)
- [PKCE generation and S256](https://www.rfc-editor.org/rfc/rfc7636.html#section-4.1)
- [OAuth transaction binding](https://www.rfc-editor.org/rfc/rfc9700.html#section-2.1.1)
- [OAuth callback leakage defenses](https://www.rfc-editor.org/rfc/rfc9700.html#section-4.2.4)
- [Differences between GitHub Apps and OAuth Apps](https://docs.github.com/en/apps/oauth-apps/building-oauth-apps/differences-between-github-apps-and-oauth-apps)

<details>
<summary>日本語</summary>

## アプリ登録方式の決定

状態はAccepted（契約合意済み・実装は未着手）。
日付は2026-10-29。
identity確認のみのログイン用にGitHub Appを登録する。追加のaccount・repository
権限を要求せず、利用者によるaccountやrepositoryへのインストールも必須にしない。
アプリのsecretとprovider callbackの処理はCloudflare側で扱い、desktopに置かない。

GitHub Appはインストールなしでユーザー認可でき、認証済みユーザーAPIも追加権限
なしで利用できる。選択済みのRust on Cloudflare構成で、repositoryデータへの
アクセスを要求せずにログインの境界を設けられる。専用OAuth Appも比較したが、
GitHubの一般的な推奨に沿うGitHub Appを選び、ログインにインストール操作を含めない。

本ADRは#251の合意済み契約を記録し、リポジトリへの提供はレビューを経て行う。
アプリの実登録やログイン成功を意味しない。移行が必要な稼働中の認証データはない。

## 認可と保持情報の決定

外部ブラウザの認可コード方式を使い、登録済みprovider callbackとtoken交換を
Cloudflare側で処理する。ランダムなstateとPKCE S256の両方を必須にする。
client secretとPKCE verifierはserver内に置く。HTTPS originとアプリの開始・取消・
引き渡しAPIは#193で定義する。

ログインごとに認証済みGET /userで本人確認する。外部identityとして保持するのは
GitHub数値IDのみで、login名・画像・メール・provider応答全体は保持しない。
メール一致でaccountを統合しない。GitHub IDは外部accountの識別子であり、
アプリ自身のaccount IDやsessionとは分ける。

初めて本人確認できたGitHub数値IDではアプリアカウントを自動作成し、以後は同じ
関連付けを使う。公開GitHubの固定provider値`github`と数値IDをidentityのkeyとする。
accountの自動統合や別のGitHub IDへの付け替えは行わない。同時初回ログインの
関連付けの一意性は#193の保存処理で保証する。

今回の認証専用GitHub Appではwebhook配信を無効にし、GitHubの認可取消を
既存アプリsessionへ即時反映する保証は設けない。期限・logout・失効は#193で定義し、
将来webhookを導入する場合は別途判断する。

providerのaccess tokenはcallback処理中だけ一時利用する。access・refresh tokenを
永続化せず、desktop・log・redirect URLにも渡さない。成功・失敗の両方で破棄する。
発行済みaccess tokenは個別失効を試みてから破棄する。provider資格情報を保持しないため、
後のGitHub利用不能を即時検知する保証はできない。アプリsessionの期限・失効は
#193で定義する。

## callbackの期限と一回性

認証試行の作成から10分間だけprovider callbackを受け付け、期限は延長しない。
期限ちょうど以降のcallbackは拒否し、新しい試行を要求する。

各試行で相関stateとPKCE verifierを独立させる。新しい試行は古い試行を取り消さず、
取消は対象の試行だけに適用する。別の試行のstate・verifierを流用しない。

provider拒否や失敗を含め、callbackを処理として受理するのは一回だけとする。
再送でcode交換や本人確認済み結果の発行を繰り返さない。相関情報の欠落・不明・不一致で
別の有効な試行を消費・取消してはならない。処理中断で受理済み試行を再び開かない。

provider処理前に、期限内のpending試行を原子的に確保する必要がある。
保存先、開始client・browserへの関連付け、取消の認可と競合、処理中断時の回復は
実装前に#193で定義する。stateだけをdesktopへの引き渡しの証明にはしない。

## provider tokenの後処理

本人確認後に、GitHubの個別token失効APIで発行済みuser access tokenだけを失効させる。
アプリへの認可全体の取消やtokenのresetは行わない。発行済みtokenで本人確認が
完了しなかった場合も後処理を試みるが、後処理を本人確認の代わりにはしない。

後処理の失敗で本人確認済みの結果を取り消さない。成否にかかわらずaccess・refresh tokenを
破棄し、永続的なretry queueは作らない。記録するのは無害化した後処理の結果だけとし、
token値・断片・provider応答bodyを含めない。失効に失敗したtokenはGitHub側で期限切れ・
失効するまで有効な場合があり、ローカル破棄はGitHub側の無効化を保証しない。

access tokenの削除で対応するrefresh tokenも必ず無効になるとは仮定しない。
文書・実機テストではその連動を確認していない。返されたrefresh tokenは使わず破棄し、
GitHub Appのuser token期限切れ機能は有効のままとする。identity確認だけのflowで
offline_accessは要求しない。

## provider・callbackインターフェース

provider契約は合意済み。実装には#193の保存・session・引き渡しに関する合意も必要。
stateとPKCE verifierはそれぞれ独立した暗号学的乱数32byteをpaddingなしの
base64urlで43文字にする。stateの長さはアプリ方針。S256はASCII verifierのSHA-256を
paddingなしbase64urlにする。試行は開始clientとbrowserに関連付け、stateの存在確認
だけでは十分な関連付けと扱わない。

providerへの具体的なrequestは英語の表に示す4操作とする。
認可はGitHub authorize endpointへのGET、交換は同じredirect URIとverifierを使う
token endpointへのPOST、本人確認は認証済みGET /user、後処理は個別tokenへのDELETE。
secretはrequest bodyまたは所定の認証headerだけに載せ、URLに入れない。
token交換はJSONを要求し、HTTP 200でもerrorがあれば失敗。本人確認はHTTP 200と
u64で表せる正の整数idを必須にし、他のprofile情報は無視する。
REST API versionは2026-03-10、個別失効は204だけを成功として扱う。

callbackは設定済みHTTPS originのGET /auth/github/callbackとする。環境ごとのoriginと
アプリ実登録は#193・配置時の入力であり、request parameterから選ばない。
celldのローカルHTTP probeはこのHTTPS callbackを検証しない。

queryは空でないstate一つと、codeまたはerrorの一方一つを要求する。
既知parameterの重複とcode/error両方の指定を拒否する。不明parameterは無視し、
戻り先やidentityに使わない。error_descriptionとerror_uriは信用せず無視する。
構文・相関・開始browserとの関連付けを検証してから試行を原子的に確保する。

HTTP結果は、本人確認・account関連付け成功=200 verified、GitHubのaccess_denied=
200 cancelled、不正callback=400 invalid_callback、受理済み=409 attempt_consumed、
期限切れ=410 attempt_expired、取消済み=410 attempt_cancelled、provider処理失敗=
502 provider_failure、必要なアプリ保存先が利用不能=503 authentication_unavailable。

ブラウザには固定outcomeと静的messageだけの最小HTMLを返し、code・state・token・
user ID・provider error本文を含めない。Cache-Control: no-storeと
Referrer-Policy: no-referrerを設定し、外部resourceや外部linkを含めない。
アプリ・ingress・trace・error logではcallback query値を除去する。
browser履歴からの漏えい対策とdesktopへの戻り方の詳細は#193で定義する。

backend内部の本人確認結果は英語の合成JSON例の形とし、数値精度を失わないよう
subjectは正規化した10進文字列にする。server発行のattempt_idで試行に関連付ける。
この結果は信頼する認証coordinator内だけで扱い、browser応答・アプリsession・
desktop引き渡し資格情報にはしない。account作成・一意性・結果公開は#193の原子性・
取消ルールを満たす。保存失敗や取消が先に成立した後はverifiedを返さない。

provider要求には実装上のtimeoutを設ける。code交換の成否が不明な場合に同じcodeを
自動再送せず、失敗した試行は最初からやり直す。provider障害やログイン失敗だけで
既存アプリsessionを取り消さない。後処理失敗だけは、本人確認済みのログインを妨げない。

## 合成データによる検証ケース

- ID 1001の初回・再ログインでprofileが変わっても同じaccountとprovider/ID関連だけを保持する。
- 同じIDへの初回ログイン競合では、#193の保存保証によって関連付けを一つにする。
- 並行する二試行の逆順完了・片方の取消でも、state・verifier・結果を混用しない。
- 599秒での受理は可能、600秒ちょうど以降は交換前に期限切れにする。
- 同時・再送callbackでもprovider処理の確保と本人確認済み結果は最大一回。
- state欠落・不明・入れ替え、別browser、parameter重複、code/error両指定は処理前に拒否する。
- GitHub拒否・期限切れcode・HTTP 200のerror・不正IDでは本人確認済み結果を作らない。
- 本人確認のtimeout・rate limit・5xxでは当該試行だけ失敗させ、codeを自動再送しない。
- 失効204・失効失敗・timeoutのすべてでprovider秘密情報を破棄する。
- 取消・中断・保存失敗の競合で試行再開や二重公開、先に成立した取消後の成功を許さない。
- browser応答と全log/trace経路に秘密情報・callback query値・profile応答・user IDを含めない。

## 確認したproviderの制約

2026-10-29に公式文書を確認した。

- 認可要求はapp IDとは別のclient IDを使い、code交換にはclient secretが必要。
- PKCEはS256に対応し、plainには非対応。redirect先は登録済みcallbackと一致させ、
  任意の追加parameterは付けられない。
- GET /userはGitHub App user tokenで追加権限なしに利用できる。
  login名は変わり得るため、安定した数値IDを関連付けに使う。

上記の期限と試行独立性はアプリ側の決定であり、GitHubが保証するものではない。
デスクトップへの引き渡しは別途契約を定義する。

## #193に残る決定

保存保証、browser/clientとの具体的な
関連付け、取消の認可・競合、運用timeout、account永続化、session、配置origin、
desktop引き渡しは#193で定義する。アプリの開始・取消・poll・claim APIは今回の
provider callback契約とは分ける。#193の契約が未決の間は認証実装に着手しない。

</details>
