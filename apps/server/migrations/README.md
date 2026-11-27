# Authentication migrations

`0001_auth_identity.sql` adds application accounts and GitHub identity
associations. `0002_auth_sessions.sql` adds application sessions and
separate access and refresh credential digests. A session can have one current
refresh credential and retain consumed digests for replay detection.
`0003_auth_attempts.sql` adds OAuth attempt descriptors, including a D1-assigned
creation order for the global logout cutoff and the fixed admission and hard
deadlines.

The deployment runner skips migration execution
when there are no regular `.sql` files, and otherwise runs Wrangler migrations
against the `AUTH_DB` binding before deploying the Worker. Wrangler records applied
migrations; never edit an applied migration or reset an existing database to deploy.
Review every new migration for compatibility with the currently deployed Worker.

<details>
<summary>日本語</summary>

`0001_auth_identity.sql` はアプリアカウントと GitHub 主体の関連付けを追加する。
`0002_auth_sessions.sql` はアプリセッションと、アクセス・更新資格情報の
ダイジェストを追加する。各セッションで有効な更新資格情報は1件までとし、
再使用検出のため消費済みのダイジェストを保持できる。
`0003_auth_attempts.sql` は OAuth 試行の基本情報を追加する。全セッションの
ログアウト時の判定に使う作成順を D1 が割り当て、受付期限と絶対期限を保存する。

配置ツールは通常の.sqlファイルがなければmigrationをskipし、あればAUTH_DB bindingへ
Wrangler migrationを適用してからWorkerを配置する。適用履歴はWranglerが管理する。
適用済みSQLの変更や配置目的のDBリセットは行わず、稼働中Workerとの互換性を確認する。

</details>
