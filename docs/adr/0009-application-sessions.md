# ADR 0009: Application session renewal and revocation

- Status: Accepted
- Date: 2026-10-29
- Related: [ADR 0005](0005-github-authorization-contract.md), [ADR 0008](0008-application-authentication-state.md), issues #270, #269 and #193

## Integrated contract resolution (#193)

[ADR 0010](0010-desktop-authentication-handoff.md) extends session inspection
with nullable `reauthenticated_at_ms`, while `provider_verified_at_ms` remains
the original thirty-day lifetime anchor. Reauthentication updates freshness on
the same active session without issuing credentials or extending absolute expiry.
ADR 0010 also grants a time-bounded original-attempt proof authority solely to
prevent issuance or revoke that attempt's issued session after ambiguous claim
completion. It does not change ordinary refresh-token logout or cancellation.
Connection profiles and deployment-specific credential namespaces follow ADR 0010.

<details>
<summary>日本語</summary>

ADR 0010によりinspectionへnullableなreauthenticated_at_msを追加する。provider_verified_at_msは
元の30日期限の基準のままとし、同じ有効sessionの鮮度だけを更新して資格情報を再発行しない。
応答喪失時は、期限付きの元試行proofで発行を防ぐか、その試行が発行したsessionだけを失効できる。
通常refresh logoutや取消の意味は変えない。接続profileと配置ごとの資格情報分離はADR 0010に従う。

</details>

## Application-session renewal decision

Use application-owned access tokens valid for fifteen minutes and application
refresh tokens rotated on each successful refresh. Require a new GitHub
authorization and identity verification after seven days without renewal
activity or thirty days from GitHub identity verification, whichever comes
first. Refresh does not extend the thirty-day absolute deadline.

These are application credentials, separate from the GitHub access and refresh
tokens discarded under ADR 0005. Renewing an application session does not call
GitHub or establish fresh provider authentication. With provider webhooks
disabled, a GitHub authorization revoked during the interval is not guaranteed
to revoke an existing application session before its own limits are reached.

The detailed contract below makes the policy reviewable. The chosen durations
are application policy, not GitHub or Tauri defaults.

## Logout decision

Ordinary logout revokes only the application session used by the current
desktop, including its refresh chain. Provide a separate explicit operation to
log out all application sessions for the account, requiring a new GitHub
authorization and identity verification first. Define its freshness and
account-binding checks in the detailed contract.

Logging out of an application session does not revoke the GitHub application
grant, delete usage history, or authorize decryption on a new device.

## Session contract

### Credentials and time

One desktop login and its refresh chain form one application session with an
opaque server-issued `session_id`. Other logins are independent. This is not a
permanent hardware identifier or history-decryption enrollment.

Generate access and refresh credentials independently from at least 32 random
bytes using a cryptographic generator. Store purpose-separated hashes in D1,
and check primary session/account state and authentication generation on every
protected request. Self-contained token claims are insufficient for acceptance.
Raw credentials are returned once over the fixed HTTPS API; never put them in
URIs, browser pages, logs, or frontend events. Rust owns authenticated calls.
Keep access credentials in memory and refresh credentials in environment-specific,
device-local OS storage, separate from history keys and pending-attempt secrets.
Do not fall back to plaintext.

Use absolute server UTC timestamps in milliseconds. The thirty-day deadline
starts at successful GitHub identity verification. The idle deadline starts at
initial session issuance plus seven days and moves only after a successful
refresh. Ordinary API calls do not separately extend it. Refresh when needed
for authenticated work, without an otherwise unnecessary keepalive heartbeat.
An access credential expires at the earliest of issuance plus fifteen minutes,
idle expiry, and absolute expiry. Reject at or after any applicable deadline.
Refresh never changes the GitHub verification time or the absolute deadline.

### Rotation, replay, and response loss

Consume the current refresh credential, issue the replacement pair, and move
the idle deadline in one guarded transaction under #269. Retain consumed
refresh hashes until absolute expiry to recognize reuse. Existing access
credentials can finish their original lifetime while the session remains active.

Known consumed-refresh reuse revokes that whole application session, including
its current refresh and all access credentials. Other sessions stay independent.
Unknown or malformed credentials cannot revoke a session. Revoking this
application's access credentials is an explicit policy beyond the cited RFC's
refresh-token rotation rule.

Coalesce concurrent refresh in the native process. Never automatically resend
a predecessor after an ambiguous response: rotation may already have committed.
If the replacement cannot be received, validated, and securely saved, stop using
the local session and require GitHub login again. Do not accept a predecessor
again or reissue a lost secret. A received replacement may be used for
best-effort session revocation before disposal. Save the new refresh credential
before adopting it in memory.

### Logout and fresh identity verification

Current-session logout revokes its entire refresh chain. A current or recognized
consumed refresh credential can authorize only revocation of that same session,
not access or renewal. Repeats and unknown credentials receive the same
idempotent response without revealing whether a session exists.

Global logout requires an active access credential and new GitHub identity
verification for the same application account within the preceding five minutes.
An account mismatch cannot authorize it. Refresh does not satisfy freshness.
New GitHub authorization does not guarantee another password or MFA prompt.

Global logout atomically revokes every session for the account and invalidates
already-created login attempts and handoffs, including anonymous attempts whose
account is learned later. Record a D1-ordered attempt-creation cutoff for the
account, and require publication/claim to check it and the current account
authorization generation. A pre-logout attempt cannot restore a session later;
a genuinely new attempt can log in after GitHub verification. The handoff
contract must implement this boundary.

Revocation rejects subsequent authorization decisions. It cannot undo completed
actions or necessarily interrupt requests already authorized.

For local logout, clear memory and durably erase the saved refresh credential
or record a non-secret signed-out marker before reporting local completion.
The marker prevents loading a credential left behind by failed OS deletion.
Do not remove history keys or unrelated pending-attempt records. Confirm remote
logout only after acknowledgement. Offline, report remote revocation as
unconfirmed; the server session may remain valid until revoked or expired.
Do not silently keep active credentials as a durable logout retry queue. If
neither erasure nor a durable marker succeeds, report incomplete logout rather
than a success that cannot survive restart.

### API and retention

Use one configured HTTPS authentication origin, without browser-cookie
authentication for native requests. Requests cannot choose a target account
through an arbitrary account ID. Send `Cache-Control: no-store` and exclude
credential-bearing bodies and authorization headers from logs.

| Endpoint | Request | Success |
| --- | --- | --- |
| `POST /v1/auth/refresh` | JSON `refresh_token` | 200, one replacement credential pair. |
| `GET /v1/auth/session` | Bearer access credential | 200, session/account IDs, verification time, idle and absolute deadlines; no provider profile. |
| `POST /v1/auth/logout` | JSON `refresh_token` | 204, idempotent revocation of that session. |
| `POST /v1/auth/logout-all` | Bearer access credential and empty JSON object | 204 after fresh same-account verification; includes the calling session. |

Initial issuance belongs to #271's one-time handoff transaction, without a
second public issuance endpoint. Initial issuance and refresh share this schema;
values below are synthetic placeholders on a clock starting at zero:

```json
{
  "token_type": "Bearer",
  "session_id": "synthetic-session",
  "account_id": "synthetic-account",
  "access_token": "synthetic-access",
  "refresh_token": "synthetic-refresh",
  "access_expires_at_ms": 900000,
  "idle_expires_at_ms": 604800000,
  "absolute_expires_at_ms": 2592000000
}
```

Production expiry values are absolute timestamps, not durations. The session
inspection response is the projection `session_id`, `account_id`,
`provider_verified_at_ms`, `idle_expires_at_ms`, `absolute_expires_at_ms`.

Errors use JSON `{"error":"stable_code"}`: 400 `invalid_request`, 401
`invalid_session` for unknown, expired, revoked, or replayed credentials,
403 `reauthentication_required`, 429 `rate_limited`, and 503
`authentication_unavailable` for primary failure. Rate limiting precedes
credential consumption. Never echo credentials or another account's identity.
An ambiguous network failure never permits replay of a one-time refresh.

Keep consumed hashes and revocation metadata until absolute expiry, then
logically remove them from the primary within twenty-four hours. Backup
retention is an operational property; logical deletion is not physical backup
erasure. Enforce #269's external generation change after restore.

## Synthetic acceptance cases

- Refresh replaces a predecessor once without extending the thirty-day deadline.
- Access/refresh fail at the exact applicable deadline, including seven idle days.
- Competing refreshes cannot issue two accepted successors; known replay revokes only its session.
- Invalid random credentials cannot revoke a real session or identify its owner.
- Response loss and OS-store failure never cause plaintext fallback or secret reissuance.
- Ordinary logout preserves another desktop's session.
- Global logout rejects stale verification and a different GitHub identity.
- A pre-global-logout anonymous attempt cannot later restore the account's session.
- Durable local logout requires erasure or a signed-out marker; leftover credentials are not reloaded.
- Restored old-generation sessions fail primary authorization.

These are implementation-test specifications, not executed session or Cloudflare
acceptance evidence. #193 still requires the separate handoff contract.

## References

- [Refresh-token rotation and inactivity](https://www.rfc-editor.org/rfc/rfc9700.html#section-4.14.2)

<details>
<summary>日本語</summary>

## アプリsessionの更新方針

アプリ独自のaccess tokenを15分有効とし、refresh tokenは更新成功のたびに交換する。
更新に関する活動が7日ない場合、またはGitHub本人確認から30日で、早い方の期限に
達したらGitHubで新しい認可と本人確認を行う。refreshで30日の絶対期限は延長しない。

これらはADR 0005で破棄するGitHub access・refresh tokenとは別のアプリ資格情報。
アプリsession更新ではGitHubに問い合わせず、providerの新しい本人確認を意味しない。
webhook配信を無効にするため、途中でGitHubの認可が取り消されても、アプリ独自の
期限より先に既存sessionが失効する保証はない。

以下で詳細契約として具体化する。期間はアプリ方針で、GitHubや
Tauriの既定値ではない。


## logoutの方針

通常logoutは、現在のdesktopが使っているアプリsessionとその更新系列だけを失効させる。
accountの全sessionをlogoutする操作は別に明示し、新しいGitHub認可と本人確認を要求する。
本人確認の鮮度と同じaccountへの関連付けは詳細契約で定義する。

アプリsessionのlogoutは、GitHubへのアプリ認可の取消、利用履歴の削除、
新端末で履歴を復号するための承認とは分ける。

## session契約

一つのdesktopログインと更新系列をserver発行のsession_idで識別し、他のログインとは
分ける。恒久的なhardware IDや履歴復号の端末承認とは別物。

access・refreshはそれぞれ32byte以上の暗号学的乱数から作り、D1に用途別hashを保存する。
保護要求ごとにprimaryのsession/account状態と認証generationを確認する。
token自身のclaimだけで認可せず、生の資格情報は固定HTTPS APIで一回だけ返す。
URI・browser・log・frontend eventへは渡さず、Rustが認証済みAPIを扱う。
accessはメモリ、refreshは環境別・端末限定OS storeで管理し、履歴鍵・保留試行とは分ける。
平文保存へ退避しない。

### 期限

server UTCのミリ秒時刻を使い、絶対期限はGitHub本人確認から30日。
無利用期限は初回発行から7日で始まり、更新成功だけで動かす。通常APIの呼出しで
別途延長しない。必要な処理のために更新し、未使用sessionの延命heartbeatは行わない。
access期限は発行から15分・無利用期限・絶対期限の最も早い時刻とし、期限ちょうど以降は
拒否する。refreshでGitHub本人確認時刻と30日期限は変えない。

### 更新と再利用

旧refreshの消費・代替発行・期限更新を#269のガード付きtransactionでまとめる。
旧hashは絶対期限まで保持する。有効なsessionの既存accessは元の短い期限まで利用できる。
既知の消費済みrefreshの再利用では、そのsessionのrefreshと全accessを失効させる。
別sessionや、不正乱数で特定できないsessionは失効させない。
accessも失効させる範囲はアプリ方針で、RFCのrotation規則だけで決まるものではない。

Rust内で同時更新をまとめる。成否不明の応答後に旧refreshを自動再送しない。
代替を受信・検証・安全に保存できなければ、当該ローカルsessionの利用を止めて
GitHubでやり直す。旧資格情報の再受理や秘密の再発行で回復しない。
受信済みの代替は破棄前に失効を試みてもよい。新refreshは保存成功後にメモリへ採用する。

### logoutと本人確認

通常logoutは現在sessionの更新系列全体を失効させる。currentまたは既知の消費済みrefreshを
同じsessionの失効だけに使い、API利用や更新は許可しない。再送・不明資格情報にも
存在を開示しない冪等な応答とする。

全session logoutでは有効なaccessと直近5分以内の新しいGitHub本人確認を要求する。
同じアプリアカウントへ関連付け、別GitHub IDでは認可しない。refreshでは鮮度を満たさず、
新しいGitHub認可でパスワード・MFAの再入力まで必ず行うとは保証しない。

全sessionの失効と、既存の認証試行・引き渡しの無効化を原子的に行う。後からaccountが
判明する匿名試行も対象にするため、D1順序による試行作成cutoffをaccountへ記録する。
公開・claim時にcutoffとaccount認可generationを確認し、古い試行がsessionを復活させない。
新しい試行はGitHub本人確認後にログインできる。この境界は引き渡し契約側でも実装する。
失効は後続の認可判断を拒否するが、完了済み操作を戻したり認可済み処理を必ず中断したりはしない。

ローカルlogoutはメモリを破棄し、保存refreshの消去か非秘密のsigned-out markerの
永続化後に完了と表示する。OS削除失敗で残った資格情報を再読込しない。
履歴鍵や別の保留試行を削除しない。remote logoutはserver応答後にだけ確認済みとする。
offlineならremote失効は未確認と表示し、server側では失効・期限まで有効な場合がある。
active資格情報をlogout再送queueとして黙って保持しない。消去・marker保存に失敗したら
再起動後に持続する成功とは報告せず、logout未完了とする。

### APIと保持

設定済み単一HTTPS originを使い、native要求をbrowser cookieで認証しない。
requestの任意account IDを対象にしない。Cache-Control: no-storeを返し、
資格情報のbody・Authorization headerをlogに残さない。

- POST /v1/auth/refresh: refresh_tokenをJSONで送り、200で代替資格情報を一回だけ返す。
- GET /v1/auth/session: accessのBearer認証でsession/account・本人確認時刻・期限だけを返す。
- POST /v1/auth/logout: refresh_tokenを送り、204で冪等に現在sessionを失効させる。
- POST /v1/auth/logout-all: accessと空JSONで、再本人確認後に呼出元を含めて失効させ204を返す。

初回発行は#271の引き渡しtransaction内とし、別の公開発行APIを設けない。
初回・更新のschemaは英語の合成JSON例を使う。例は時刻0での本人確認・発行を仮定し、
実際の期限は期間ではなくserver絶対時刻。検査応答のfieldは英語のprojectionどおりとする。

errorはJSONのerror fieldとし、400 invalid_request、401 invalid_session、
403 reauthentication_required、429 rate_limited、503 authentication_unavailableを使う。
rate limitは消費前に判定する。資格情報・他accountのidentityを返さず、
成否不明の通信断で旧refreshを再利用しない。

旧hashと失効metadataは絶対期限まで保持し、その後24時間以内にprimaryから論理削除する。
backup保持は運用特性であり、論理削除をbackupの物理消去とは扱わない。
復元時は#269のD1外generation変更を必須にする。

## 合成データの受け入れケース

- 更新は旧refreshを一回だけ交換し、30日期限を延長しない。
- access/refreshは期限ちょうどで拒否し、7日無利用でも更新しない。
- 同時更新で二つの有効な後継を作らず、既知旧refresh再利用はそのsessionだけを失効させる。
- 不正乱数で実在sessionを取り消したり、ownerを特定したりできない。
- 応答喪失・OS store障害で平文保存や秘密再発行へ退避しない。
- 通常logoutは別desktopに影響しない。
- 全体logoutは古い本人確認・別GitHub IDを拒否する。
- 全体logout前の匿名試行が後からsessionを復活させない。
- 消去かsigned-out markerなしに永続的logout成功を報告せず、残った資格情報を再読込しない。
- 復元した旧generationのsessionをprimaryで拒否する。

これらは実装テスト用仕様であり、sessionやCloudflare受け入れを実行した証拠ではない。
#193の完了には引き渡し契約も必要。

</details>
