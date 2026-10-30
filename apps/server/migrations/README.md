# Authentication migrations

Authentication tables and versioned SQL migrations belong to #195. This directory
intentionally contains no SQL yet. The deployment runner skips migration execution
when there are no regular `.sql` files, and otherwise runs Wrangler migrations
against the `AUTH_DB` binding before deploying the Worker. Wrangler records applied
migrations; never edit an applied migration or reset an existing database to deploy.
Review every new migration for compatibility with the currently deployed Worker.

<details>
<summary>日本語</summary>

認証tableとversion付きSQLは#195で実装する。現時点はSQLを置かない。
配置ツールは通常の.sqlファイルがなければmigrationをskipし、あればAUTH_DB bindingへ
Wrangler migrationを適用してからWorkerを配置する。適用履歴はWranglerが管理する。
適用済みSQLの変更や配置目的のDBリセットは行わず、稼働中Workerとの互換性を確認する。

</details>
