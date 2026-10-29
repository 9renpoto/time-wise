# ADR 0011: Rust Worker runtime and local verification

- Status: Accepted
- Date: 2026-10-29
- Related: ADRs 0006 and 0010; issues #194, #277, #278 and #279

## Context

The agreed server is a Rust Cloudflare Worker, but the reserved server package
does not yet select a runtime SDK or build pipeline. Local verification uses
celld, and operator deployments must build reproducibly without provider secrets
in source or build output.

## Decision

Use Cloudflare's `workers-rs` SDK (`worker`) and `worker-build` to produce the
JavaScript entry point and Wasm artifact. Use Wrangler for Cloudflare packaging
and deployment, and run the generated entry point under celld for local smoke
tests. Keep domain/configuration logic independent of Worker bindings so native
Rust unit tests can cover invalid configuration and protocol responses.

Do not add a separate HTTP framework for the foundation. Use the SDK request
handler and keep provider calls, persistence and session logic in their existing
implementation issues. Keep real authentication disabled during foundation work.

Pin and validate dependency/build-tool versions during implementation, using
the workspace Rust toolchain. Check generated Wasm/JavaScript compatibility in
celld and the Cloudflare deployment pipeline separately. Official documentation
establishes the supported integration path, not successful execution here.

The alternative is handwritten JavaScript glue around Rust Wasm. That creates
an additional binding boundary for future D1 and provider operations; prefer
the maintained Worker SDK rather than owning that glue.

## Validation boundary

Use failing unit tests before configuration/protocol behavior, then implement
the smallest passing change. Require Rust checks, reproducible Wasm build and
a celld HTTP smoke check for #277. #278 owns deployment repeatability and #279
owns actual DNS/TLS/HTTPS acceptance. Local success does not complete them.

## References

- [Cloudflare Rust support](https://developers.cloudflare.com/workers/languages/rust/)
- [celld Wasm and workers-rs entry points](https://celld.dev/docs/wasm/)

<details>
<summary>日本語</summary>

## 背景と決定

Rust Cloudflare Workerは合意済みだが、予約済みserver packageはSDK・build方式を未選定。
workers-rsのworker SDKとworker-buildでJS entry/Wasmを生成し、Wranglerで配置、celldでローカル検証する。
設定・protocolのロジックはbindingから分け、native Rustの単体テストで検証する。
基盤に別HTTP frameworkを追加せず、provider・保存・sessionは既存実装issueへ分離する。
基盤作業中に実認証を有効化しない。手書きJS glueという追加境界を持つ代替よりSDKを推奨する。

依存・build toolを実装時に固定してworkspace toolchainで検証する。公式文書は対応経路の根拠であり、
今回の実行成功の証拠ではない。TDDで設定・protocolを実装し、#277でRust検査・Wasm build・celld HTTPを確認。
#278は配置再現性、#279はDNS/TLS/HTTPS実受け入れを担当し、local成功だけでは完了しない。

</details>
