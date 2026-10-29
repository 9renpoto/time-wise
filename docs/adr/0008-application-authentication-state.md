# ADR 0008: Keep authentication state in one D1 database

- Status: Accepted (consistency contract confirmed; implementation pending)
- Date: 2026-10-29
- Related: [ADR 0005](0005-github-authorization-contract.md), issues #269 and #193

## Integrated contract resolution (#193)

[ADR 0009](0009-application-sessions.md) and
[ADR 0010](0010-desktop-authentication-handoff.md) resolve the session, browser
binding, cancellation, timeout, deployment and handoff decisions originally
listed as open below. ADR 0010's explicit issued-session revocation operation
is separate from cancellation: it atomically prevents issuance or revokes only
the session linked to that attempt, without reopening or reissuing a handoff.
The D1 consistency and external restore-generation guarantees remain required.

<details>
<summary>日本語</summary>

下記で未決としたsession・browser関連付け・取消・期限・配置・引き渡しはADR 0009・0010で確定した。
ADR 0010の明示的な発行session失効は取消と別の操作で、将来の発行を原子的に防ぐか、
その試行のsessionだけを失効する。引き渡しの再開・再発行はしない。D1整合性と復元generationの保証は維持する。

</details>

## Persistence decision

Keep authentication attempts, GitHub identity associations, desktop handoff
state, and application sessions in one Cloudflare D1 database. Route
authentication and revocation decisions to the primary database, without the
Sessions API, read-replica routing, or positive authentication caches initially.

Use a database uniqueness constraint for the provider/subject association and
conditional SQL transitions for one-time claims. Keep account association,
publication, handoff consumption, and session creation within the same database
transaction boundary where they must commit together. Provider network calls
must take place outside database transactions.

D1's transactional batch rolls back when a statement fails. An UPDATE affecting
zero rows is successful SQL, so it does not abort a batch. Guard every dependent
write with the winning operation's fresh marker and eligible state; never rely
on examining affected-row metadata after unconditional writes already ran.
Exact SQL and failure behavior require implementation probes and concurrency
tests before adoption.

## Consistency contract

Treat each successful conditional write or transactional batch commit as the
ordering point for competing requests. Every operation checks the expected
state, server-enforced deadline, and current authentication generation. Only
the winner of a one-time claim may perform the corresponding external action.
Do not derive authorization from a preliminary read that can become stale.

Use a fresh unpredictable operation marker to guard every dependent write in
a batch. A losing claim produces no account, association, handoff, session, or
replacement credential through that operation. A SQL error rolls back the
whole batch. Specify and test the behavior through the actual D1 binding rather
than relying on an unverified `changes()` trick or `RETURNING` assumption.

| Boundary | Atomic guarantee |
| --- | --- |
| GitHub identity association | Exactly one account association per `(provider, subject)`, including simultaneous first logins. |
| Provider callback claim | At most one transition from eligible pending state to processing; later requests never repeat provider processing. |
| Verified-result publication | Cancellation or expiry committed first prevents publication and all writes dependent on that publication. |
| Desktop handoff consumption | Consumption and initial session issuance commit together, with at most one issuance. |
| Session refresh | Predecessor consumption, replacement issuance, and lifecycle updates commit together. |
| Session revocation | Subsequent authorization reads the primary and rejects revoked credentials, without a positive-authentication cache. |

Cancellation after a successful claim may be unable to stop an in-flight
provider request. It still prevents later application publication if its state
transition wins. Attempt cancellation does not roll back an already committed
account association or revoke an already issued session; the session contract
defines logout after issuance.

After process interruption, do not reopen an accepted callback or consumed
handoff. A lost HTTP response does not undo a committed transaction. The session
and handoff contracts must define client recovery without issuing credentials
twice. Primary unavailability fails authentication closed; do not fall back to
a replica, cached positive result, or local success.

Store high-entropy bearer credentials and verifier values as purpose-separated
hashes when only equality or possession checks are needed. Keep provider access
and refresh tokens out of D1. Temporary OAuth material that must be recovered,
such as the server's PKCE verifier before callback processing, requires
authenticated encryption under an environment-specific Worker secret and a
bounded lifetime. Do not log credential values, hashes, or complete records.
Detailed field ownership and deadlines belong to #270 and #271.

## Restore boundary

A database restore can restore an earlier unconsumed or unrevoked state. Before
reopening authentication after restore, drain authentication work and change an
authentication generation stored in Worker configuration outside D1. Bind all
attempts, handoffs, and application sessions to their creation generation, and
reject older generations on every authentication decision.

This deliberately invalidates all outstanding attempts and application sessions
after restore and requires a new GitHub login. Coordinate generation changes
and Worker rollout while authentication traffic is stopped. The restored
database alone cannot establish that this gate passed. Deployment/runbook work
in #202 must verify the gate; local celld does not validate Cloudflare restore
or rollout behavior.

## Synthetic acceptance cases

- Simultaneous first logins for one provider/subject create one association.
- Simultaneous callback claims have one winner and one provider-processing path.
- A zero-row claim leaves every dependent table untouched.
- Injecting a SQL constraint error after an earlier mutation rolls back the batch.
- Committing cancellation before publication prevents all publication writes.
- Competing handoff claims issue one session; a lost response cannot cause reissuance.
- Competing refresh requests obey the session contract without unguarded replacement writes.
- A primary failure never accepts a credential through cache or replica fallback.
- Restored pending or active records from an older generation are rejected.

Durable Objects with D1 were considered. Per-attempt actors would separate
coordination from globally unique identity and session data, requiring recovery
across two stores. A single D1 transaction domain fits the current contract
without that additional boundary. This decision does not select storage for
future synchronization data or approve the proposed E2EE design in ADR 0002.

celld supports D1 for local checks but has no read replicas. Local success
cannot establish Cloudflare replica or cloud failure behavior. Authentication
implementation remains gated on the rest of #193: session lifecycle, initiating
client/browser binding, cancellation races, timeouts, deployment origins, and
the desktop handoff contract. Those decisions remain open.

## References

- [D1 transactional batch](https://developers.cloudflare.com/d1/worker-api/d1-database/#batch)
- [D1 read replication](https://developers.cloudflare.com/d1/best-practices/read-replication/)
- [D1 concurrency](https://developers.cloudflare.com/d1/platform/limits/#concurrency-and-throughput)
- [celld D1 compatibility](https://celld.dev/docs/services/d1/)

<details>
<summary>日本語</summary>

## 保存先の決定

認証試行・GitHub identityとの関連付け・desktop引き渡し状態・アプリsessionを
一つのCloudflare D1にまとめる。最初はSessions API・read replica・認証成功cacheを
使わず、認証と失効の判断をprimaryで行う。

provider/subjectの関連付けにDBの一意制約を設け、一回限りの確保は条件付きSQL更新で
行う。同時にcommitすべきaccount関連付け・結果公開・引き渡し消費・session発行を
同じDB transactionの境界に収める。providerへの通信はtransactionの外で行う。

D1 batchはSQL失敗時にrollbackするが、UPDATEが0件でもSQL自体は成功となる。
後続の各書込みも、勝った操作固有のmarkerと必要な状態で条件付ける。
無条件の書込みを実行してから更新件数を見て成否を判断する方法にはしない。
具体的なSQLと異常系は実装時のprobeと並行要求テストで検証する。

## 整合性契約

競合要求の順序は、条件付き書込みまたはtransaction batchのcommit時点で確定する。
各操作で期待する状態、serverで判定する期限、現在の認証generationを検証する。
一回限りの確保に勝った処理だけが対応する外部操作を行い、先に読んだ古い状態から
認可を判断しない。

batch内の各書込みを、操作ごとの予測困難なmarkerで条件付ける。確保に負けた操作は
account・関連付け・引き渡し・session・代替資格情報を作らない。SQL失敗時は全体を
rollbackする。未検証のchanges()やRETURNINGの挙動に依存せず、実際のD1 bindingで検証する。

原子的な境界は次のとおり。

- 同時初回ログインでもprovider/subjectごとのaccount関連付けは一つにする。
- callbackはpendingからprocessingへ最大一回だけ進み、provider処理を再実行しない。
- 取消・期限切れが先に成立した場合は、本人確認済み結果とその従属書込みを公開しない。
- 引き渡しの消費と初回session発行を同時にcommitし、発行は最大一回にする。
- session更新では旧資格情報の消費・代替発行・期限更新を同時にcommitする。
- 失効後の認証はprimaryの状態を確認し、成功cacheから許可しない。

取消は処理中のprovider通信そのものを止められない場合があるが、先にcommitすれば
後のアプリ結果公開を防ぐ。既にcommitしたaccount関連付けは取消で巻き戻さず、
発行済みsessionの失効はsession契約のlogoutとして扱う。

中断した受理済みcallbackや消費済み引き渡しは再開可能な状態へ戻さない。
HTTP応答の喪失でcommit済み処理は巻き戻らない。資格情報を二重発行しない回復方法を
session・引き渡し契約で定義する。primaryが使えない場合は認証を失敗させ、
replica・成功cache・ローカル成功判定へ切り替えない。

等値・所持確認だけに使う高entropyの資格情報やverifierは用途別hashで保存し、
GitHub access・refresh tokenをD1へ保存しない。callback前のserver PKCE verifierのように
再取得が必要な一時情報は、環境ごとのWorker secretによる認証付き暗号化と期限で保護する。
資格情報の値・hash・record全体をlogに残さない。項目ごとの所有と期限は#270・#271で定義する。

## 復元の境界

DB復元で消費前・失効前の状態が戻る可能性がある。復元後に認証を再開する前に処理を
drainし、D1外のWorker設定にある認証generationを変更する。全試行・引き渡し・sessionに
作成時generationを関連付け、すべての認証判断で古いgenerationを拒否する。

この方式では、復元後の保留試行とアプリsessionはすべて無効になり、GitHubで再ログインする。
認証trafficを停止した状態でgeneration変更とWorker配置を揃える。復元済みDBだけでは
このgateの通過を証明できない。#202の配置・runbookで検証し、local celld成功を
Cloudflareの復元・rollout検証とは扱わない。

## 合成データの受け入れケース

- 同じprovider/subjectへの同時初回ログインでも関連付けは一つ。
- 同時callbackでも確保とprovider処理の経路は一つ。
- 0件の条件付き確保では後続tableを書き換えない。
- 途中でSQL制約エラーを起こすと、先行更新もrollbackする。
- 取消を先にcommitした場合は公開処理と従属書込みを行わない。
- 同時引き渡しでもsessionは一つで、応答喪失で再発行しない。
- 同時refreshでもsession契約と条件付き書込みを守る。
- primary障害時にcache・replica経由で認証を許可しない。
- 復元した旧generationの保留・有効recordを拒否する。

Durable ObjectsとD1の併用も比較した。試行単位のactorと、全体で一意にするidentity・
sessionの保存先を分けると、二つの保存先をまたぐ失敗回復が必要になる。
今回はD1一つのtransaction境界で扱う。この決定は将来の同期データ保存先を選ぶものでも、
ProposedのADR 0002のE2EE設計を承認するものでもない。

celldはD1のローカル検証に使えるがread replicaはなく、local成功でCloudflareの
replicaやcloud障害時の動作は確認できない。認証実装には#193の残る合意が必要。
session、開始client/browserとの関連付け、取消競合、timeout、配置origin、
desktop引き渡しの契約は引き続き未決とする。

</details>
