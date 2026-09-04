# サーバー準備のタスク案 / Server preparation task plan

- 状態 / Status: 議論用ドラフト / Discussion draft
- 更新日 / Updated: 2026-09-04
- 設計 / Design: [ADR 0003](../adr/0003-passkey-account-authentication.md), [ADR 0002](../adr/0002-encrypt-usage-data-end-to-end.md)

## 目的と範囲 / Goal and scope

開発者が設計上の未決事項を解消し、レビュー可能な単位へ実装を分割するための計画。今回の到達点は、TauriアプリからCloudflare上のTime Wise認証画面を開き、パスキーで登録・ログインした後、アプリへ戻って認証済みAPIを利用できること。Cloudflare管理画面へのログインを指すものではない。端末承認、鍵共有、履歴同期、結果閲覧用Webアプリは後続とする。以下はタスク候補と受け入れ条件案であり、着手済みタスクや確定した公開要件ではない。

This plan helps developers resolve design questions and split implementation into reviewable units. The current milestone is opening Time Wise authentication hosted on Cloudflare from Tauri, registering or signing in with a passkey, and returning to the application with access to an authenticated API. This does not mean signing in to the Cloudflare dashboard. Device enrollment, key sharing, history synchronization and the results-viewing web app are later scope. The tasks and acceptance criteria below are proposals, not work already started or finalized public-release requirements.

## 作成済みIssue / Created issues

ADR 0003はProposed・レビュー待ちのまま、以下を起票済み。Issue作成は設計承認や実装着手を意味しない。レビュー結果を各Issueへ反映してから実装する。

The following issues were created while ADR 0003 remains Proposed and pending review. Issue creation does not approve the design or start implementation. Incorporate review outcomes into each issue before implementation.

| タスク / Task | Issue | 内容 / Scope |
| --- | --- | --- |
| T01 | [#192](https://github.com/9renpoto/time-wise/issues/192) | Workers上のRust・WebAuthn互換性検証 |
| T02 | [#193](https://github.com/9renpoto/time-wise/issues/193) | 認証・保存・ドメインの設計確定 |
| T03 | [#194](https://github.com/9renpoto/time-wise/issues/194) | Workersアプリと検証環境の基盤 |
| T04 | [#195](https://github.com/9renpoto/time-wise/issues/195) | 認証データの永続化 |
| T05 | [#196](https://github.com/9renpoto/time-wise/issues/196) | パスキーだけの新規登録 |
| T06 | [#197](https://github.com/9renpoto/time-wise/issues/197) | ログインとサーバーセッション |
| T07 | [#198](https://github.com/9renpoto/time-wise/issues/198) | 複数パスキーの管理 |
| T08 | [#199](https://github.com/9renpoto/time-wise/issues/199) | アプリ向け認証引き渡しAPI |
| T09 | [#200](https://github.com/9renpoto/time-wise/issues/200) | Tauriからブラウザー認証と復帰 |
| T10 | [#201](https://github.com/9renpoto/time-wise/issues/201) | デスクトップの資格情報と状態管理 |
| T11 | [#202](https://github.com/9renpoto/time-wise/issues/202) | 公開前の濫用対策と運用ルール |
| T12 | [#203](https://github.com/9renpoto/time-wise/issues/203) | Cloudflare・Tauriの受け入れテスト |

## 実行タスク / Execution tasks

以下を着手可能なタスク単位とする。すべて未着手。T01・T02で技術的な未決事項を解消し、結果をADRへ記録してから依存する実装へ進む。タスクの完了には記載の検証結果を残す。1タスクはレビュー単位の目安であり、必要に応じて複数PRに分ける。

Use the following units for execution; none has started. Resolve technical choices in T01 and T02 and record them in the ADR before dependent implementation. Completion requires recorded validation results. Each task is a review boundary and may span multiple PRs.

| ID | タスク / Task | 依存 / Depends on | 成果物と完了条件 / Deliverable and acceptance |
| --- | --- | --- | --- |
| T01 | Workers上のRust・WebAuthn互換性検証 / Rust and WebAuthn compatibility spike | — | 最小の検証コードで登録応答・認証応答の検証をWorkers上で実行し、正常・不正な入力の結果、ビルド手順、採用可能なライブラリを記録する。実行不能なら理由と代替案を示し、後続実装へ進まない。 / Run registration and authentication response verification on Workers; record valid/invalid cases, build instructions and viable libraries. Stop dependent implementation if incompatible. |
| T02 | 認証・保存・ドメインの設計確定 / Authentication, storage and domain contracts | T01 | Cloudflare内の保存サービス、データモデル、ドメイン・RP ID・origin、セッション期限・失効、再認証の有効範囲、アプリへの引き渡しをADRとAPI契約にする。同時更新と一回限りの処理を検証する。 / Record storage, model, domain, RP ID, origins, session lifecycle, reauthentication and handoff contracts; validate concurrency and single-use semantics. |
| T03 | Workersアプリと検証環境の基盤 / Workers application and test deployment | T02 | `apps/server` に選定済みの基盤とテスト・ビルド手順を用意する。秘密情報をリポジトリへ入れず、制限したCloudflare検証環境へ再現可能に配置してHTTPS疎通を確認する。 / Set up the selected foundation under `apps/server`, tests and builds; validate reproducible restricted HTTPS deployment without committed secrets. |
| T04 | 認証データの永続化 / Authentication persistence | T03 | アカウント、パスキー、チャレンジ、セッションの保存境界とマイグレーションを作る。一意性、期限、一回限りの消費、アカウント分離、同時更新のテストを通す。 / Implement persistence and migrations; test uniqueness, expiry, single-use consumption, account isolation and concurrency. |
| T05 | パスキーだけの新規登録 / Passkey-only registration | T04 | 認証用Web画面と登録APIを用意し、識別子自動生成、復旧不可の説明、最初のパスキー登録を実現する。中断・期限切れ・再送・不正originで不完全な有効アカウントを作らない。 / Provide registration UI/API, generated identifiers and recovery warnings; handle cancellation, expiry, replay and invalid origins safely. |
| T06 | ログインとサーバーセッション / Login and server sessions | T05 | 既存パスキーでログインし、認証済みAPIから自分の識別子を取得できる。セッション更新・失効・ログアウトと未認証／他アカウントの拒否を検証する。 / Implement login, authenticated identity retrieval, renewal, revocation and logout; reject unauthenticated and cross-account access. |
| T07 | 複数パスキーの管理 / Multiple-passkey management | T06 | 一覧・追加・登録解除を実装する。追加・解除時に既存パスキーで再認証し、最後の1件の同時削除を拒否する。解除したパスキー由来のセッションだけを失効させる。 / Implement listing, addition and removal with reauthentication, concurrent final-passkey protection and related-session revocation. |
| T08 | アプリ向け認証引き渡しAPI / Desktop authentication handoff API | T06 | アプリが開始した要求に結び付く、一回限り・期限付きの引き渡しを実装する。横取り・要求差し替え・再使用・期限切れを拒否し、長期資格情報をURL・ログに出さない。 / Implement request-bound, single-use, expiring handoff; reject interception, substitution, replay and expiry without exposing long-lived credentials in URLs or logs. |
| T09 | Tauriからブラウザー認証と復帰 / Tauri browser authentication and return | T08 | Windows・macOSでシステムブラウザーを開き、認証後に開始元アプリへ戻る。認証済みAPIによる本人確認まで行い、キャンセル・タイムアウト・多重起動・不正な戻り値を扱う。 / Open browser authentication and return on Windows/macOS; verify identity through the authenticated API and handle cancellation, timeout, multiple launches and invalid returns. |
| T10 | デスクトップの資格情報と状態管理 / Desktop credentials and login state | T07, T09 | OS資格情報ストアで保存し、再起動後の状態復元、更新、ログアウト、パスキー解除による失効を扱う。保存先が使えない場合は平文保存へ退避せず、エラーを表示する。 / Use OS credential storage; handle restart, renewal, logout and revocation, reporting unavailable storage without plaintext fallback. |
| T11 | 公開前の濫用対策と運用ルール / Abuse controls and operational readiness | T07, T10 | 登録・認証の制限、ログ最小化、秘密情報の非記録、保持・削除の運用手順を実装・検証する。復旧経路がないことを確認し、公開可能かの判断材料を残す。 / Implement and validate abuse limits, minimal secret-free logs and retention/deletion procedures; verify no recovery bypass and record publication readiness. |
| T12 | Cloudflare・Tauriの受け入れテスト / Cloudflare and Tauri acceptance testing | T11 | Windows・macOS実機で新規登録と再ログインからアプリ復帰・API利用まで確認する。予備パスキー、削除と失効、再起動、通信断、異常な引き渡しを含め結果を記録する。ストア申請や同期は行わない。 / Record Windows/macOS end-to-end results including spare passkeys, removal/revocation, restart, network failure and invalid handoff; exclude store submission and synchronization. |

T07とT08はT06の完了後に並行して進められる。T11は最終的な横断検証の位置付けであり、入力検証・アカウント分離・秘密情報の保護をそこまで先送りしない。T03以降の検証環境は公開準備が整うまでアクセスを制限する。

T07 and T08 may proceed in parallel after T06. T11 is cross-cutting validation, not permission to defer input validation, isolation or secret protection. Keep test deployments restricted until ready for public access.

### 先行設計で解消する事項 / Design gates

- T01: Rust/WebAuthnのWorkers実行互換性。候補が成立しない場合は技術方針を再検討する。
- T02: `9renpoto.win` のサブドメインまたは別ドメイン、Cloudflare保存サービス、認証用Web画面の配置、アプリへの戻り方、セッションと再認証の期限・失効、パスキー数上限、保持・削除ルール。
- 認証基盤の構築は、ストア互換性の保証、セルフホスト対応、同期設計の承認を意味しない。

T01 gates runtime compatibility; reconsider the technical approach if no candidate works. T02 resolves domains, Cloudflare storage, authentication-page placement, app return, session/reauthentication lifecycle, credential limits and retention/deletion. Building authentication does not guarantee store compatibility, add self-hosting, or accept the synchronization design.

## 設計上の区分 / Design workstreams

以下のS1〜S8は議論時の区分として保持する。実行・起票には上のT01〜T12を使い、二重にタスク化しない。

Retain S1–S8 below as discussion workstreams. Use T01–T12 for execution or issue creation; do not create duplicate tasks from both lists.

| ID | タスク / Task | 依存 / Depends on | 受け入れ条件案 / Proposed acceptance criteria |
| --- | --- | --- | --- |
| S1 | Workers互換性検証と認証設計 / Workers compatibility and authentication design | — | Rustの実行先をCloudflare Workersとし、WebAuthnライブラリ・暗号処理のビルドと実行を検証する。DB、RP ID・origin、最小保存情報、セッションとアプリへの引き渡し方式を決める。 / Target Cloudflare Workers for Rust; validate building and running the WebAuthn library and cryptographic dependencies. Select the database, RP ID and origins, minimum stored data, sessions and desktop handoff. |
| S2 | サーバー・デプロイ基盤 / Server and deployment foundation | S1 | 設定・秘密情報の管理、DBマイグレーション、再現可能なデプロイ手順を用意し、Cloudflare検証環境へHTTPSで疎通できる。認証未完成の登録APIを無制限に公開しない。 / Provide configuration and secret management, database migrations and reproducible deployment; verify HTTPS connectivity to the Cloudflare test environment without unrestricted exposure of unfinished registration APIs. |
| S3 | 公開登録とログイン / Open registration and login | S2 | メールなしで登録・再ログインできる。無効・期限切れ・再使用のチャレンジ、誤ったoriginを拒否し、アカウント間のアクセスを分離する。 / Register and sign in without email; reject invalid, expired or reused challenges and incorrect origins; isolate accounts. |
| S4 | パスキー管理と関連セッション失効 / Passkey management and related-session revocation | S3 | 複数登録、各パスキーでのログイン、追加・削除時の再認証、最後の1件の保護、削除したパスキー由来のセッション失効を検証する。 / Validate multiple registration, login with each passkey, reauthentication for addition/removal, final-passkey protection and related-session revocation. |
| S5 | ブラウザーからTauriへの引き渡し / Browser-to-Tauri handoff | S3 | アプリが開始した認証要求へ結果を結び付け、アプリへ戻る。再使用、期限切れ、別の認証要求への差し替えを拒否する。キャンセル、タイムアウト、多重起動を扱い、URLやログへ長期利用可能な認証情報を露出させない。 / Bind results to the initiating app request and return to Tauri; reject replay, expiry and request substitution; handle cancellation, timeout and multiple launches without exposing long-lived credentials in URLs or logs. |
| S6 | Tauriのログイン状態と資格情報管理 / Tauri login state and credential management | S4, S5 | 対応OSの資格情報ストアへ保存し、認証済みAPIから取得したアカウント識別子を表示する。再起動後の状態復元、期限切れ・更新、ログアウト、サーバーでの失効を扱う。 / Store credentials in supported OS credential stores and display the account identifier from an authenticated API; handle restart, expiry/renewal, logout and server revocation. |
| S7 | 公開前の安全性・運用検証 / Pre-publication security and operational validation | S4, S6 | 登録・認証の濫用対策、ログ最小化と保持・削除、アカウント分離、復旧不可の説明、代替復旧経路がないことを検証する。 / Validate registration/authentication abuse controls, minimized logs and retention/deletion, account isolation, recovery warnings and absence of alternative recovery. |
| S8 | CloudflareからTauriまでの受け入れ検証 / Cloudflare-to-Tauri acceptance | S2, S7 | Windows・macOS実機で、新規登録と既存アカウントのログインからアプリへの復帰・認証済みAPI利用まで確認する。パスキー削除後のデスクトップセッション失効、再接続、異常な引き渡しの拒否も確認する。 / On Windows and macOS hardware, validate registration and existing-account login through app return and authenticated API access; also validate desktop-session revocation after passkey removal, reconnection and rejection of invalid handoff. |

## 分割上の注意 / Planning boundaries

ストア配布は将来の目標とし、今回の対象外とする。S8はストアへの申請や審査通過を必要とせず、Windows・macOSのTauriアプリからCloudflareで認証しアプリへ戻れることを検証する。ストア固有の配布設定と互換性確認は後続タスクに分け、今回のドメイン選定でストア配布可否まで保証しない。

Store distribution is a future goal outside this milestone. S8 validates authentication on Cloudflare and return to the Windows and macOS Tauri application without requiring store submission or approval. Defer store-specific packaging and compatibility checks; domain selection here does not guarantee eligibility for store distribution.

初期版の認証サービスはプロジェクト運営者本人がホストする。利用者向けのセルフホスト手順や接続先切り替えは今回のタスクに含めない。S1で保有ドメイン `9renpoto.win` のサブドメイン案と別ドメイン案を評価し、認証ドメインとRP IDを確定する。ドメインの採用、購入、DNS変更はこの計画では行わない。

The project operator hosts the initial authentication service. Exclude user self-hosting instructions and server switching from this milestone. In S1, evaluate a subdomain of the owned `9renpoto.win` against a separate domain and decide the authentication domain and RP ID. This plan does not adopt or purchase a domain or modify DNS.

認証バックエンドの保存先はCloudflare内で完結させ、外部DB・外部認証サービスは今回の候補から外す。S1でアカウント、パスキー検証情報、チャレンジ、セッションの保存サービスを選定する。チャレンジの一回限りの使用、同時削除時の最後のパスキー保護、セッション失効の反映に必要な整合性を検証する。具体的な製品の採用はこの計画だけでは確定しない。

Keep authentication-backend storage within Cloudflare; exclude external databases and identity services from this scope. In S1, select storage services for accounts, passkey verification records, challenges and sessions. Validate the consistency needed for single-use challenges, final-passkey protection under concurrent removal, and session revocation enforcement. This plan does not yet select specific products.

S1は、Workers上でのRust疎通とWebAuthn検証処理の互換性確認、永続化と認証フローの設計に分けて進める。ライブラリ名だけで対応済みと判断せず、必要な登録・認証の検証処理を実行してからS2以降へ進む。DBやセッション保存先はまだ採用しない。

Split S1 into Rust connectivity and WebAuthn-verification compatibility on Workers, followed by persistence and authentication-flow design. Validate the required registration and authentication verification paths rather than assuming compatibility from a library name before proceeding to S2 and later work. Database and session storage choices remain open.

S3では、ユーザー名やメールなどの入力なしに識別子が自動生成され、パスキー選択画面でアカウントを識別できることを確認する。生成形式と衝突回避はS1で決め、識別子だけで認証やアカウントへのアクセスができないことを検証する。

For S3, verify automatic identifier generation without username or email input and account identification in the passkey selector. Define format and collision handling in S1; verify that an identifier alone cannot authenticate or grant account access.

S4では、登録解除したパスキーによるログイン済みセッションが失効し、他のパスキーによるセッションがこの操作だけでは失効しないことを検証する。S5ではデスクトップへ引き継いだセッションと更新後のセッションにも失効が反映されることを検証する。失効の反映タイミングと再認証時の関連付けはS1で決める。

For S4, verify revocation of sessions established with the removed passkey without revoking sessions established with other passkeys solely because of this operation. For S5, validate revocation after desktop handoff and session renewal. Decide enforcement timing and associations on reauthentication in S1.

S4ではパスキー登録解除にも同じアカウントの既存パスキーによる再認証を要求し、最後の1件の解除を拒否する。同時に複数の解除要求が来ても登録数が0件にならないことを検証する。アカウント削除は別操作として設計する。

For S4, require reauthentication with an existing passkey for the same account to remove a registration, and reject removal of the final one. Verify that concurrent removal requests cannot leave zero registrations. Design account deletion separately.

S4では、既存アカウントへのパスキー追加に登録済みパスキーでの再認証が必要なことを検証する。ログイン済みセッションだけの要求と、別アカウントのパスキーによる再認証は拒否する。新規アカウントの最初の登録はS3で別途検証する。

For S4, verify that adding a passkey to an existing account requires reauthentication with a registered passkey for that account. Reject session-only requests and reauthentication with another account's passkey. Test first-passkey registration for new accounts separately in S3.

- S1の議論を進めてから各タスクを小さなPRへ分割する。フレームワークやAPIの詳細はこの一覧で先に確定しない。
- 公開登録を可能にする製品方針と、インターネットへ実際に公開する判断を分ける。公開前にS7の対策と運用上の保持・削除ルールを確認する。
- 結果閲覧用Webアプリは後続。認証用Web画面は必要に応じて今回の対象に含むが、配置場所は未決。
- 既存のTauri構造を維持する。共有ドメインの `crates/` への切り出しはサーバー実装の必要性が見えてから判断する。
- 後続Issueは起票済みだが、この文書変更はコード実装やデプロイを含めない。レビュー結果をIssueへ反映してから着手する。

Discuss S1 before splitting tasks into small PRs; this list does not select frameworks or API details. Open registration is a product direction, not immediate authorization to deploy publicly. Review S7 controls and operational retention/deletion rules before public deployment. Defer the results-viewing web application, while allowing authentication pages within this scope; their location remains undecided. Preserve the conventional Tauri structure and defer shared-domain extraction into `crates/` until implementation demonstrates a need. Follow-up issues have been created, but this documentation change does not implement code or deploy services. Incorporate review outcomes into the issues before starting.

## 後続の同期タスク / Later synchronization work

今回の完了条件には含めない。従来の同期タスク案は、最初の端末での鍵生成 → 追加端末の承認・鍵共有 → 暗号化レコード・送信待ちキュー・同期API → 2端末での同期・履歴削除・端末失効検証、の順で別途分割する。ADR 0002の未決事項は維持する。

Exclude synchronization from this milestone. Split later work into first-device key generation, additional-device enrollment and key sharing, encrypted records/outbox/synchronization API, and two-device synchronization with history deletion and device-revocation validation. Preserve the open questions in ADR 0002.
