# ADR 0006: Use celld for local Cloudflare runtime verification

- Status: Accepted
- Date: 2026-10-29
- Related: [ADR 0004](0004-github-account-authentication.md), issues #265, #194, #249

Use celld as the local Cloudflare-compatible development runtime, following the
explicit choice to adopt it. Run the official v0.6.3 container pinned by digest
through Docker Compose, with host access restricted to loopback. The official
release has no Intel macOS binary, so Docker also provides a reproducible path
for that development environment.

Use `celld dev` and its persistent local store without cloud credentials or a
remote bucket. Keep the initial synthetic Worker under `tools/celld`, separate
from the reserved `apps/server` package. It checks HTTP, a Rust-generated Wasm
module, and Durable Object persistence across container recreation; it does not
select authentication endpoints, account storage, or a session contract.

This replaces reliance on a Wrangler local runtime for this development probe.
It does not change the Rust-on-Cloudflare deployment target. celld accepts a
subset of Wrangler configuration and runtime features; a local pass cannot
establish Cloudflare HTTPS, production storage guarantees, or authentication
acceptance. The application contracts in #251 and #193 remain prerequisites for
production server implementation. A workers-rs shim needs its own integration
verification when that implementation exists.

The local state is disposable development data retained across normal restarts
and stops. Never mount desktop usage history or pass cloud credentials to this
probe. Exclude celld state, Wasm output, and local environment files from Git.

## References

- [celld v0.6.3 release and assets](https://github.com/denoland/celld/releases/tag/v0.6.3)
- [Local development](https://celld.dev/docs/#develop-an-application-locally)
- [Cloudflare compatibility](https://celld.dev/docs/cloudflare-compat/)
- [WebAssembly and workers-rs](https://celld.dev/docs/wasm/)

<details>
<summary>日本語</summary>

## 決定

celldを導入する明示的な選択に従い、Cloudflare互換のローカル開発環境として採用する。
公式v0.6.3コンテナをdigestで固定し、Docker Composeで起動する。ホスト側の公開先は
loopbackだけにする。公式配布にIntel macOS用バイナリがないため、この環境でも
Docker経由で再現可能な導入方法を用意する。

`celld dev`のローカル永続ストアを使い、cloud資格情報やremote bucketを不要にする。
最初の合成Workerは`tools/celld`に置き、予約済みの`apps/server`から分離する。
HTTP・Rust生成Wasm・コンテナ再作成後のDurable Object状態保持を検証するが、
認証endpoint・account保存先・session契約は決めない。

この開発probeではWranglerのローカルruntimeへの依存を置き換える。
Rust on Cloudflareの配置先は変更しない。celldの対応範囲はWrangler設定・runtimeの
一部であり、local成功をCloudflare実環境のHTTPS・保存保証・認証受け入れの成功とは
扱わない。production server実装には#251・#193の契約が必要で、workers-rsのshimも
実装時に別途結合検証する。

ローカル状態は通常の再起動・停止では保持する開発データとする。このprobeへ
desktop利用履歴やcloud資格情報を渡さない。celld状態・生成Wasm・ローカル環境変数
ファイルはGit対象から除外する。

</details>
