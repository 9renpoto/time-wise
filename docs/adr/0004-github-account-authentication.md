# ADR 0004: Use GitHub authentication and defer passkeys

- Status: Accepted (decision confirmed; implementation pending)
- Date: 2026-10-27
- Supersedes: [ADR 0003](0003-passkey-account-authentication.md)
- Related: [ADR 0002](0002-encrypt-usage-data-end-to-end.md), issues #249, #229, #252

Use GitHub authentication for the current desktop authentication milestone and
defer application-managed passkeys. Keep the application server in Rust on
Cloudflare Workers. The implementation and restricted-environment validation
are still pending; selecting GitHub does not establish a working login flow.

## Evidence and trade-off

The earlier spike demonstrated Rust Worker builds and local HTTP responses.
It did not complete real WebAuthn registration or authentication. The recorded
`webauthn-rs` 0.5.5 attempt failed on Wasm dependencies including OpenSSL, and
`passkey-auth` 0.1.3 panicked on `SystemTime` at registration start. PR #206 was
closed without merging; PR #211 merged into its spike branch rather than `main`.

A separate minimal probe checked `passkey-server` 0.1.4 for
`wasm32-unknown-unknown`. The check passed with the crate's `wasm` feature,
`uuid/js`, and `getrandom` 0.2's `js` feature. Source inspection found that
registration and login enforce user presence but do not require the user
verification flag, and challenge retrieval and deletion are separate store
operations. These findings leave verification-policy and concurrent-consumption
work before adoption. No Workers runtime ceremony or physical-authenticator
test was performed for this candidate.

This evidence does not prove that Rust and Workers cannot support passkeys.
It does not establish a sufficiently validated implementation for this milestone
either. Defer further passkey investigation, compatibility patches, and verifier
maintenance, and use GitHub as the selected external identity provider. Google
and Supabase comparison work is no longer part of this milestone.

## Consequences

- GitHub controls the external identity and its authentication and recovery
  mechanisms. Application-owned accounts, OAuth correlation state, sessions,
  and later synchronization data remain on Cloudflare. This replaces the old
  restriction against using an external identity provider.
- GitHub login, application sessions, and authorization to decrypt history are
  separate concepts. A GitHub credential or provider token is not the desktop's
  application session or a history-decryption key. ADR 0002 remains Proposed.
- The application does not register, list, add, or remove passkeys. Backup
  passkeys, final-passkey protection, and passkey-derived session revocation
  are removed from the current acceptance criteria. Credentials managed by
  GitHub are outside the application's credential-management boundary.
- The passkey-only account-recovery policy is superseded. GitHub account
  recovery and the application's handling of inaccessible or disabled external
  accounts must be distinguished in the authentication contract.
- There is no deployed authentication data or session schema to migrate in
  `main`. The migration affects decisions, terminology, and issue scope.
- Track execution and dependencies in issue #249 and its sub-issues. Retire
  the old server-preparation plan rather than maintain a second task list.

## Decisions still required before implementation

[ADR 0005](0005-github-authorization-contract.md), tracked in #251, records the
confirmed GitHub App authorization contract, permissions, retained identity,
account binding, callback interface, token cleanup, and failure cases.
Issue #193 still requires agreement on Cloudflare storage, concurrency
guarantees, session lifecycle, reauthentication, and desktop handoff. Application
start/cancel/poll/claim APIs, domain values, and session retention periods remain
open until that contract is recorded. Authentication implementation remains gated
on #193.

## References

- [GitHub OAuth authorization](https://docs.github.com/en/apps/oauth-apps/building-oauth-apps/authorizing-oauth-apps)
- [GitHub authenticated-user API](https://docs.github.com/en/rest/users/users#get-the-authenticated-user)
- [passkey-server 0.1.4 source](https://docs.rs/crate/passkey-server/0.1.4/source/src/protocol.rs)
- [Earlier compatibility evidence](https://github.com/9renpoto/time-wise/issues/192#issuecomment-5556901952)

<details>
<summary>日本語</summary>

## 決定

状態はAccepted（方針合意済み・実装は未着手）、日付は2026-10-27。
今回のデスクトップ認証にはGitHub認証を採用し、アプリ自身が管理するパスキーは
見送る。Rust on Cloudflare Workersは維持する。ADR 0003を置き換えるが、
実際のログイン成功や制限した実環境での動作を確認した意味ではない。

## 根拠とトレードオフ

従来のspikeではRust Workerのbuildとlocal HTTP応答を確認したが、実際の
WebAuthn登録・認証は未完了。webauthn-rs 0.5.5はOpenSSL等のWasm依存、
passkey-auth 0.1.3は登録開始時のSystemTimeで失敗した。
PR #206は未マージでcloseされ、PR #211はmainではなくspike branchへmergeされた。

追加の最小probeではpasskey-server 0.1.4のwasm機能、uuid/js、
getrandom 0.2のjs機能を有効にするとwasm32-unknown-unknownのcheckが通った。
一方、ソースでは登録・ログイン時にUPを検証するがUVを必須にせず、
challengeの取得と削除は別のstore操作だった。採用前には本人確認方針と
同時要求での一回性を検証・解決する必要がある。この候補のWorkers実行や
実際の認証器による登録・認証は試していない。

RustとWorkersでパスキーが実現不能とは断定しない。今回の採用に十分な
検証結果がないため、追加調査・互換性修正・verifier保守を見送り、
外部認証providerとしてGitHubを採用する。Google・Supabase比較も今回の対象から外す。

## 影響

- 外部identity、認証、復旧はGitHubの管理範囲。アプリのアカウント、OAuth相関状態、
  session、将来の同期データはCloudflareに置く。「外部認証を使わない」という制約を置き換える。
- GitHubログイン、アプリsession、履歴を復号する端末承認を分ける。
  provider tokenをデスクトップ用sessionや復号鍵として扱わない。ADR 0002はProposedのまま。
- パスキー登録・一覧・追加・解除、予備パスキー、最後の1件の保護、
  パスキー由来session失効を今回の完了条件から外す。GitHubが管理する資格情報は
  アプリの資格情報管理範囲に含めない。
- パスキー専用の復旧不可方針を置き換える。GitHubの復旧と、外部アカウントへ
  アクセスできない場合や無効化された場合のアプリ側の扱いは認証契約で分けて定義する。
- mainに稼働中の認証データ・session schemaはなく、データ移行は不要。
  変更対象は設計判断、用語、Issueの範囲となる。
- 実行順とタスクは#249配下で管理し、旧サーバー準備計画書は廃止する。

## 実装前の未決事項

#251で追跡する[ADR 0005](0005-github-authorization-contract.md)に、GitHub Appの
認可契約・権限・保持情報・identityとの関連付け・callback・token後処理・異常系を記録した。
#193ではCloudflare保存先、同時要求保証、session、再認証、desktopへの引き渡しの合意が
引き続き必要。アプリの開始・取消・poll・claim API、domain、session保持期間はその契約で
定義する。#193の合意までは認証実装に着手しない。

</details>
