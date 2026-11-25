# Authentication migrations

`0001_auth_identity.sql` adds application accounts and GitHub identity
associations. The deployment runner skips migration execution
when there are no regular `.sql` files, and otherwise runs Wrangler migrations
against the `AUTH_DB` binding before deploying the Worker. Wrangler records applied
migrations; never edit an applied migration or reset an existing database to deploy.
Review every new migration for compatibility with the currently deployed Worker.

<details>
<summary>日本語</summary>

`0001_auth_identity.sql` はアプリアカウントと GitHub 主体の関連付けを追加する。
配置ツールは通常の.sqlファイルがなければmigrationをskipし、あればAUTH_DB bindingへ
Wrangler migrationを適用してからWorkerを配置する。適用履歴はWranglerが管理する。
適用済みSQLの変更や配置目的のDBリセットは行わず、稼働中Workerとの互換性を確認する。

</details>
