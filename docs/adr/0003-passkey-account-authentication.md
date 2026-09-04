# ADR 0003: パスキーによるアカウント認証 / Passkey account authentication

- 状態 / Status: 検討中 / Proposed
- 日付 / Date: 2026-09-04
- 関連 / Related: [ADR 0002](0002-encrypt-usage-data-end-to-end.md), [タスク案 / Task plan](../plans/server-preparation.md)

## コンテキスト / Context

### 日本語

将来の到達点は、同じ利用者の2台のデスクトップ間で暗号化された利用履歴を同期できること。今回のタスク分割は、Cloudflare上のTime Wise認証画面で実際に登録・ログインし、Tauriアプリへ戻って認証済みAPIを利用できるところまでとする。端末承認、鍵共有、履歴同期、結果閲覧用Webアプリは後続に分ける。Rustの実行先はCloudflare Workersとし、DBなどの周辺サービスは未選定。

この文書は開発者向けの設計記録であり、実装済み機能や公開サービスの保証を説明するものではない。以下の合意した方針と、引き続き比較する候補を区別する。

### English

The longer-term milestone is encrypted usage-history synchronization between two desktop devices belonging to one user. The current task decomposition covers real registration and login on Time Wise authentication hosted on Cloudflare, returning to Tauri and using an authenticated API. Device enrollment, key sharing, history synchronization and the results-viewing web application are later scope. Run Rust on Cloudflare Workers; supporting services such as the database remain unselected.

This is a design record for developers, not a description of implemented features or public-service guarantees. It distinguishes agreed directions from candidates still under evaluation.

## 合意した方針 / Agreed direction

### 日本語

- 招待を必要とせず、誰でもアカウントを登録できる形を目指す。公開運用の開始時期は未決。
- 初期版はプロジェクト運営者本人がホストする認証サービスを配布アプリから利用する。利用者によるセルフホストと接続先サーバーの選択は今回の対象外とする。
- 将来はストア配布を目指すが、ストア審査、申請、専用の配布設定とストア版の検証は今回のサーバー準備に含めない。ストアでの承認や互換性を今回の完了条件・保証にしない。
- 今回の認証バックエンドと保存先はCloudflare内で完結させる。アカウント、パスキー検証情報、チャレンジ、セッションのために外部DBや外部認証サービスを導入しない。具体的なCloudflare保存サービスの選定は未完了であり、将来の履歴同期の保存先まで決定するものではない。
- Rustの認証サーバーはCloudflare Workers上で実行する。外部の常駐RustサーバーをCloudflare経由で公開する構成は今回の対象にしない。WebAuthnライブラリと依存する暗号処理のWorkers上での互換性を、実装前の検証タスクに含める。
- パスキーだけで登録できるようにし、メールアドレス、電話番号、実名の入力を登録条件にしない。個人情報の収集をできるだけ抑える。
- アカウントを見分ける識別子はサービス側で自動生成し、ユーザー名の入力を求めない。パスキー選択画面などの表示にも自動生成した識別情報を用いる。
- 登録済みのすべてのパスキーを利用できなくなった場合、Time Wiseはアカウント復旧を提供しない。メール、復旧コード、運営者による本人確認を使った代替の復旧経路は設けない。
- 初期版から、一つのアカウントに複数のパスキーを登録できるようにする。予備のパスキーを追加することは、アカウント復旧や履歴の復号鍵のバックアップとは区別する。
- 既存アカウントへのパスキー追加には、そのアカウントに登録済みのパスキーによる再認証を必須とする。ログイン済みセッションだけでは許可しない。すべてのパスキーを失った場合は、セッションが残っていても追加できない。新規アカウントの最初のパスキー登録は、この追加時の条件とは別に扱う。
- 認証に必要な保存情報と、運用上観測できる情報を区別し、それぞれの目的と保持期間を設計する。「メール不要」は「個人に関連する情報を一切扱わない」という意味ではない。
- アカウントへのログインと、履歴を復号するための端末承認を分けて設計する。ADR 0002 の案に従い、ログイン成功だけを新端末への復号鍵配布の根拠にしない。
- 今回の変更は設計とタスク分割を対象とする。後続Issueはレビュー待ちとして起票済みだが、サーバー実装と依存ライブラリ追加は含めない。
- パスキーの登録解除にも、そのアカウントの登録済みパスキーによる再認証を必須とする。最後の1件は登録解除できない。アカウント自体の削除は別の操作として設計する。
- パスキーを登録解除すると、そのパスキーでログインしたセッションも失効させる。他のパスキーでログインしたセッションを一括失効させる操作とは区別する。

### English

- Target open registration without invitations. The timing of public deployment remains undecided.
- Initially, distributed applications use the authentication service hosted by the project operator. User self-hosting and server selection are outside the current scope.
- Store distribution is a future goal, but store review, submission, store-specific packaging configuration and validation of store builds are outside this server-preparation scope. Store approval and compatibility are neither acceptance criteria nor guarantees of this milestone.
- Keep this authentication backend and its storage within Cloudflare. Do not introduce an external database or identity service for accounts, passkey verification records, challenges or sessions. Specific Cloudflare storage services remain unselected; this does not decide storage for future history synchronization.
- Run the Rust authentication server on Cloudflare Workers rather than proxying to an externally hosted persistent Rust server. Include Workers compatibility of the WebAuthn library and its cryptographic dependencies in pre-implementation validation.
- Allow registration using only a passkey, without requiring an email address, phone number, or real name. Minimize personal-data collection.
- Automatically generate account identifiers in the service without requesting a username. Use generated account-identifying information for displays such as passkey selectors.
- If every registered passkey becomes unavailable, Time Wise does not provide account recovery. Do not offer alternative recovery through email, recovery codes, or operator-assisted identity verification.
- Support registering multiple passkeys for one account from the initial version. Adding a spare passkey is distinct from account recovery or backing up history-decryption keys.
- Adding a passkey to an existing account requires reauthentication with a passkey already registered to that account. An authenticated session alone is insufficient. If all passkeys are lost, a remaining session cannot authorize adding one. Registration of the first passkey for a new account is separate from this addition requirement.
- Distinguish authentication records from operationally observable information and define purposes and retention for both. Email-free registration does not imply processing no information associated with an individual.
- Design account login separately from device authorization to decrypt history. Following the proposal in ADR 0002, successful login alone must not justify distributing decryption keys to a new device.
- This change covers design and task decomposition. Follow-up issues have been created pending review; server implementation and new dependencies are excluded.
- Removing a passkey registration also requires reauthentication with a registered passkey for that account. The final registration cannot be removed. Design account deletion as a separate operation.
- Removing a passkey registration also revokes sessions established by logging in with that passkey. Distinguish this from revoking all sessions established with other passkeys.

## 第一候補と代替案 / Preferred candidates and alternatives

### 日本語

- アカウントと認証は `apps/server` のRustサーバーが管理する方式を第一候補とする。外部認証サービスへの委譲は現時点では採用しないが、最終的な技術選定は未完了。
- デスクトップからシステムブラウザーで認証画面を開き、認証後にアプリへ戻る方式を第一候補とする。OSネイティブAPIとTauri内WebViewも比較対象に残す。
- ブラウザー方式は将来のWeb向け認証画面の再利用を検討しやすい一方、アプリへの安全な認証結果の受け渡しが必要。ネイティブ方式はOS別実装、WebView方式は対応環境とドメイン関連付けの検証が必要になる。
- WebAuthnの検証と暗号処理は既存ライブラリを利用する案とし、具体的なライブラリ、Webフレームワーク、DB、セッション方式、OAuth/OIDC採用の有無は未選定。

### English

- Prefer managing accounts and authentication in the Rust server under `apps/server`. Delegation to an external identity service is not the current choice, but technology selection is not final.
- Prefer opening authentication in the system browser and returning to the desktop application. Keep native operating-system APIs and the Tauri WebView as alternatives to evaluate.
- Browser authentication offers an opportunity to reuse authentication pages for a future web client, but requires a secure handoff to the desktop application. Native integration requires platform-specific work; WebView integration requires compatibility and domain-association validation.
- Propose existing libraries for WebAuthn verification and cryptographic operations. Specific libraries, web framework, database, session mechanism, and whether to use OAuth/OIDC remain undecided.

## 影響と未決事項 / Consequences and open questions

### 日本語

- アカウント復旧不可を登録時に説明する。これは履歴の復号鍵の喪失とは別の制約であり、パスキーの喪失だけで端末内の履歴が消えることを意味しない。
- パスキー追加・登録解除の再認証結果の有効期限と操作への結び付け、登録数の上限、その他の重要操作での再認証を決める。
- セッションのログイン元パスキーとの関連、ブラウザーからデスクトップへの引き継ぎや更新後の関連維持、失効の反映タイミングを設計する。別のパスキーで再認証した際の扱いは未決。セッション失効は同期端末の承認取消や端末内の復号鍵の消去と同義ではない。
- 自動生成する識別子の形式、衝突回避、内部IDと表示用識別子を分けるかを決める。メールや実名を生成元にせず、識別子そのものを認証の証明として扱わない。
- RP ID、認証用ドメイン、許可するorigin、開発環境、対応OS・ブラウザーを決める。
- 運営者が保有する `9renpoto.win` を認証ドメインの候補とする。`time-wise.9renpoto.win` はサブドメイン案であり、採用やDNS設定は未実施。長期維持、配布時のアプリ連携、将来のドメイン変更を検討してからRP IDとともに確定する。別ドメインの取得も選択肢に残す。
- チャレンジの有効期限と一回性、登録途中の状態、セッションの失効・更新、ログアウト、アプリへの認証結果の受け渡しを設計する。
- 公開登録の濫用対策と、ログ・認証情報・放置アカウントの保持期間、アカウント削除を設計する。
- 最初の端末での同期鍵生成と承認状態の初期化、追加端末の承認は同期側のタスクとする。ADR 0002 自体は引き続き検討中であり、この文書で承認済みに変更しない。

### English

- Explain the lack of account recovery during registration. This is separate from losing history-decryption keys; losing passkeys alone does not imply erasure of local history.
- Decide expiry and operation binding for reauthentication when adding or removing passkeys, registration limits, and reauthentication for other sensitive operations.
- Design session association with the login passkey, preservation of that association through browser-to-desktop handoff and renewal, and revocation enforcement timing. The effect of reauthentication with another passkey remains undecided. Session revocation is not synonymous with revoking synchronization-device authorization or erasing local decryption keys.
- Define the generated identifier format, collision handling, and whether internal IDs and display identifiers are separate. Do not derive identifiers from email addresses or real names or treat an identifier itself as authentication proof.
- Select the RP ID, authentication domain, allowed origins, development setup, and supported operating systems and browsers.
- Consider the operator-owned `9renpoto.win` for authentication. `time-wise.9renpoto.win` is a proposed subdomain, not an adopted or configured domain. Decide it together with the RP ID after considering long-term maintenance, distributed-app integration and future domain changes. Acquiring another domain remains an option.
- Design challenge expiry and single use, incomplete registration state, session revocation and renewal, logout, and authentication handoff to the desktop application.
- Design public-registration abuse controls, retention of logs, authentication records and abandoned accounts, and account deletion.
- Treat first-device synchronization-key generation and authorization bootstrap, followed by additional-device enrollment, as synchronization tasks. ADR 0002 remains Proposed; this record does not accept it.
