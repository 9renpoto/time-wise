# ADR 0002: 利用データをエンドツーエンド暗号化する / End-to-end encrypt usage data

- 状態 / Status: 検討中 / Proposed
- 日付 / Date: 2026-08-29
- 初期実装 / Initial implementation: [#181 Encrypt local usage history at rest](https://github.com/9renpoto/time-wise/issues/181)

## コンテキスト / Context

### 日本語

Time Wise は現在、利用履歴を端末内の SQLite データベースへ保存し、サーバーとは同期しない。将来、同じ利用者が複数端末のデータを管理できるようにするには、端末間で履歴を共有する同期サービスが必要になる。

利用履歴には、利用したアプリとその時間帯が含まれる。この情報をサーバー運営者やサーバーへ侵入した第三者が読めないようにしながら、保存形式と同期モデルを設計する必要がある。

### English

Time Wise currently stores usage history in an on-device SQLite database and does not synchronize it with a server. Supporting management of one user's data across multiple devices will eventually require a synchronization service that shares history between those devices.

Usage history reveals which applications were used and when. The storage format and synchronization model must prevent both the server operator and an attacker who compromises the server from reading that information.

## 参考仕様 / Reference behavior

### 日本語

Time Wise が製品体験の参考にしている Apple スクリーンタイムは、「デバイス間で共有」を各端末で有効にすると、同じ Apple Account の端末間で設定、レポートおよび利用データを共有する。レポートでは特定の端末と「すべてのデバイス」を切り替え、「すべてのデバイス」では結合統計を表示できる。

Apple の公開ユーザーガイドは、複数端末を同時利用した区間を単純加算するか、一度だけ数えるかを定義していない。そのため、Time Wise は端末別表示と全端末表示を参考にできるが、重複時間の計算規則は独自に明文化する必要がある。

参照:

- [Get started with Screen Time on iPhone](https://support.apple.com/en-ie/guide/iphone/iphbfa595995/ios)
- [View App & Website Activity settings in Screen Time on Mac](https://support.apple.com/en-gb/guide/mac-help/mchle37ec855/mac)

### English

Apple Screen Time, which Time Wise uses as a product-experience reference, shares settings, reports, and usage data between devices signed in to the same Apple Account when Share Across Devices is enabled on each device. Reports can switch between a specific device and All Devices, with All Devices showing combined statistics.

Apple's public user guides do not define whether simultaneously used intervals on multiple devices are summed or counted once. Time Wise can follow the device-specific and all-device views, but must explicitly define its own overlap rule.

References:

- [Get started with Screen Time on iPhone](https://support.apple.com/en-ie/guide/iphone/iphbfa595995/ios)
- [View App & Website Activity settings in Screen Time on Mac](https://support.apple.com/en-gb/guide/mac-help/mchle37ec855/mac)

## 決定 / Decision

### 日本語

- 同期対象の利用データは端末上で暗号化し、ローカル永続化およびサーバーへの送信を行う。復号は承認済み端末上でだけ行う。
- サーバーは暗号文を保存・配信するが、利用データの平文または復号可能な鍵を保持しない。
- サーバー運営者が利用データを復号できる復旧用コピーやマスターキーを設けない。
- 復号に必要な認証情報と、鍵を保持する承認済み端末をすべて失った場合、利用データは復旧不能とする。運営者によるデータ復旧は提供しない。
- 新しい端末は、既存の承認済み端末による明示的な承認を受けた場合だけ、利用データの復号に必要な鍵情報を取得できる。アカウントへのログイン成功だけでは復号権限を与えない。
- 個々の利用セッションを暗号化された同期元データとする。日別、週別、アプリ別などの集計値は同期せず、承認済み端末が復号した利用セッションから生成する。この判断は実装検証前の候補とし、ADR が承認されるまで確定しない。
- 同期済み履歴は、特定の端末だけを対象とする端末別表示と、すべての承認済み端末を対象とする全端末表示の両方で確認できるようにする。
- 全端末表示の総利用時間では、複数端末の利用セッションが重なる時間区間を一度だけ数える。端末別の利用時間は重複排除せず、その端末で計測した時間を表示する。
- 全端末表示のアプリ別時間は、各端末で観測した利用時間をアプリごとにすべて計上する。異なる端末の利用セッションが重なる場合、アプリ別時間の合計は重複排除した総利用時間を超えてよい。
- 全端末表示では、アプリ別時間が同時利用を含み、総利用時間と一致しない場合があることを画面上で説明する。
- 初期の全端末表示では、Windows と macOS の OS 固有識別子を持つアプリを別アプリとして扱う。同じ製品名であっても推測で統合しない。
- 将来、管理された製品カタログまたは利用者による明示的な関連付けを追加できるようにするが、初期実装には含めない。
- 初期の同期対象となるアプリ情報は、同期用の不透明な安定識別子と表示名に限定する。実行ファイルパス、アイコン取得元およびアイコン画像は端末内だけに保持する。
- 現在の Windows 用フォールバック識別子は実行ファイルパスを含むため、その値を同期用識別子として使用しない。端末内の識別根拠と、同期レコードが参照する識別子を分離する。
- 端末内では SQLite データベース全体を暗号化し、復号後のリレーショナルデータに対して検索と集計を行う。
- サーバー同期では、ローカルDBそのものではなく、個々の利用セッションから生成したE2EE同期レコードを暗号化して送信する。
- ローカルDBを暗号化する鍵と、端末間で共有する同期データ暗号鍵は、役割の異なる鍵として分離する。
- 端末固有の鍵情報は、macOS Keychain または Windows のOS資格情報保護機構へ、他端末へ同期されない端末限定データとして保存する。
- 承認済み端末ではOSユーザーへのログインをローカルの信頼境界とし、Time Wiseの起動時に端末鍵を自動解錠する。Time Wise専用パスフレーズの入力は要求しない。
- 初期の新端末承認では、既存の承認済み端末と新端末が同時にオンラインであることを必須とする。両端末に同じ短い確認コードを表示し、利用者が一致を確認して既存端末から明示的に承認する。
- サーバーは端末間の接続を仲介するが、同期用データ鍵は確認済みの端末間チャネルで暗号化して渡す。承認要求は一回限りかつ短時間で失効させる。具体的な暗号プロトコルは実装前に別途選定する。
- 同期有効時の「全履歴削除」はアカウント全体へ適用し、サーバー上の同期用暗号文と、すべての承認済み端末にある同期対象の利用履歴を削除する。設定と起動メトリクスは対象に含めない。
- 全履歴削除時に履歴世代を更新し、サーバーは古い世代のレコードを受け付けない。承認済み端末は新しい世代を確認した後、未送信レコードを含む古い履歴を削除してから同期を再開する。
- オフラインまたは到達不能な承認済み端末については即時削除を保証せず、次回同期時に古い履歴を削除する。これは全履歴削除の保証境界として許容する。
- 承認済み端末を削除した場合、サーバー上の端末認可を取り消すとともに同期用データ鍵の世代を更新する。削除後に作成する同期レコードは新しい鍵世代で暗号化する。
- 新しい同期用データ鍵は、残る承認済み端末だけが取得できるよう端末ごとに暗号化して配布する。削除端末には配布せず、削除後の履歴を復号できないようにする。
- 端末削除時に過去の同期レコードを削除または再暗号化しない。残る承認済み端末は旧鍵を保持し、後から追加する承認済み端末にも過去履歴の復号に必要な旧鍵を端末間で渡す。
- 端末削除と履歴削除を別操作として扱う。過去履歴も消す場合は、端末削除ではなくアカウント全体の「全履歴削除」を使用する。
- 初期の同期対象は、利用セッションと、その表示に必要な最小限のアプリ情報だけに限定する。設定と起動メトリクスは端末内に保持し、同期しない。
- E2EE同期レコードは不変の追記専用データとする。送信元端末は暗号化前に推測困難な一意のレコードIDを生成し、一度同期した内容を更新しない。
- 送信に失敗した場合は同じレコードIDと暗号文を再送する。サーバーはレコードIDを使って冪等に受け付け、同じIDに異なる暗号文が送られた場合は拒否する。
- サーバーが同期レコードごとに永続化する平文メタデータは、アカウントへの所属、無作為なレコードID、履歴世代、同期鍵世代、サーバーが割り当てる同期順序および暗号文サイズに限定する。
- 利用日時、計測日、タイムゾーン、アプリ情報、終了理由および送信元端末は暗号文内に置く。平文メタデータも暗号文と暗号学的に結び付け、改ざんを検知できるようにする。
- クライアントは同期順序カーソルより新しい暗号文を取得し、復号後に期間、端末およびアプリで絞り込む。サーバーは利用データの内容による検索APIを提供しない。
- 初期版は通信タイミングと通信量を隠すことを要件に含めない。同期可能になった時点で暗号文を送信し、一定サイズへのパディング、固定間隔のバッチ送信およびダミー通信は行わない。
- 公開リリース前に平文 `usage_history.sqlite` から暗号化ローカルDBへ切り替える際は、既存の開発用履歴を移行せず破棄し、空の暗号化DBを作成する。この破壊的変更は未リリースデータだけを対象とする。
- 公開リリース後の暗号化DBについては、破壊的な初期化を通常のマイグレーション手段にしない。互換性を失う変更には別の移行設計と判断を必要とする。
- 実装は段階的に行う。最初の実装範囲はローカルSQLiteデータベース全体の暗号化と、OS資格情報ストアを使った端末限定の鍵管理までとする。
- E2EE同期レコード、送信待ちキュー、アカウント認証、端末承認および同期サーバーは最初の実装に含めず、このADRの後続タスクとして扱う。
- ローカルDB暗号化ライブラリ、同期レコードの暗号プリミティブと直列化形式、および端末間鍵転送プロトコルの具体的な選定は、このADRの未決事項として引き続き設計する。

### English

- Encrypt synchronized usage data on the device before local persistence or transmission to the server. Decrypt it only on authorized devices.
- The server stores and distributes ciphertext but does not possess usage-data plaintext or a key capable of decrypting it.
- Do not provide an operator-accessible recovery copy or master key that can decrypt usage data.
- If the user loses both the credentials required for decryption and every authorized device holding key material, the usage data is unrecoverable. The operator does not provide data recovery.
- A new device obtains the key material required to decrypt usage data only after explicit approval by an existing authorized device. Successfully authenticating to the account alone does not grant decryption access.
- Treat individual usage sessions as the encrypted source records for synchronization. Do not synchronize daily, weekly, or per-application aggregates; authorized devices derive them from decrypted usage sessions. This is a candidate decision pending implementation validation and remains provisional until the ADR is accepted.
- Make synchronized history available in both a per-device view scoped to one device and an all-device view scoped to every authorized device.
- In the all-device view, count overlapping usage intervals from multiple devices only once toward total usage time. Per-device views do not deduplicate overlaps and show the time measured on that device.
- In the all-device view, attribute all observed usage on every device to each corresponding application. When sessions overlap across devices, the sum of per-application usage may exceed the deduplicated total usage time.
- Explain in the all-device view that per-application usage includes simultaneous device use and therefore may not equal total usage time.
- In the initial all-device view, treat applications with Windows-specific and macOS-specific identities as separate applications. Do not merge them speculatively even when their product names match.
- Preserve the option to add a managed product catalog or explicit user-defined associations later, but exclude both from the initial implementation.
- Limit initially synchronized application information to an opaque stable synchronization identifier and a display name. Keep executable paths, icon sources, and icon images on the originating device.
- The current Windows fallback identity contains an executable path, so do not use that value as a synchronization identifier. Separate on-device identity evidence from the identifier referenced by synchronized records.
- Encrypt the entire on-device SQLite database, then run searches and aggregates over relational data after local decryption.
- For server synchronization, encrypt E2EE synchronization records generated from individual usage sessions rather than transmitting the local database itself.
- Separate the key that encrypts the local database from the synchronization data-encryption key shared between devices because they serve different roles.
- Store device-specific key material in macOS Keychain or the Windows operating-system credential protection facility as device-only data that is not synchronized to other devices.
- Treat operating-system user login as the local trust boundary on an authorized device and unlock the device key automatically when Time Wise starts. Do not require a separate Time Wise passphrase.
- Initial device enrollment requires the existing authorized device and the new device to be online simultaneously. Display the same short authentication code on both devices and require the user to compare it and explicitly approve enrollment on the existing device.
- The server brokers the device connection, but the synchronization data key is transferred through an encrypted channel between the verified devices. Enrollment requests are single-use and expire quickly. Select the concrete cryptographic protocol separately before implementation.
- When synchronization is enabled, Delete all history is an account-wide operation that removes synchronization ciphertext from the server and synchronized usage history from every authorized device. It does not delete settings or startup metrics.
- Advance a history generation when all history is deleted, and reject records from older generations at the server. After observing the new generation, authorized devices delete old history, including pending uploads, before resuming synchronization.
- Do not guarantee immediate deletion on offline or unreachable authorized devices; delete their old history when they next synchronize. This is an accepted boundary of Delete all history.
- When an authorized device is removed, revoke its server authorization and advance the synchronization data-key generation. Encrypt synchronization records created after removal with the new key generation.
- Distribute the new synchronization data key encrypted separately for each remaining authorized device. Do not distribute it to the removed device, preventing that device from decrypting history created after removal.
- Do not delete or re-encrypt historical synchronization records when removing a device. Remaining authorized devices retain old keys, and a subsequently enrolled authorized device receives the old keys required to decrypt history through device-to-device transfer.
- Treat device removal and history deletion as separate operations. To remove historical data as well, use the account-wide Delete all history operation rather than device removal.
- Limit initial synchronization to usage sessions and the minimum application information needed to display them. Keep settings and startup metrics on the device and do not synchronize them.
- Make E2EE synchronization records immutable and append-only. Before encryption, the source device generates an unpredictable unique record identifier and never updates a record after synchronization.
- Retry a failed upload with the same record identifier and ciphertext. The server accepts it idempotently by record identifier and rejects different ciphertext submitted under an existing identifier.
- Limit plaintext metadata persisted per synchronization record to account membership, a random record identifier, history generation, synchronization key generation, a server-assigned synchronization order, and ciphertext size.
- Put usage timestamps, measurement date, time zone, application information, end reason, and source device inside the ciphertext. Cryptographically bind plaintext metadata to the ciphertext so tampering is detectable.
- Clients fetch ciphertext newer than their synchronization-order cursor and filter by date, device, and application after decryption. The server does not provide content-based search APIs for usage data.
- The initial implementation does not attempt to hide communication timing or volume. Send ciphertext when synchronization becomes available, without fixed-size padding, fixed-interval batching, or dummy traffic.
- When switching from the plaintext `usage_history.sqlite` to the encrypted local database before the public release, discard existing development history without migration and create an empty encrypted database. This destructive change applies only to unreleased data.
- After public release, do not use destructive reinitialization as the normal migration strategy for the encrypted database. An incompatible change requires a separate migration design and decision.
- Implement the design incrementally. The first implementation is limited to whole-database encryption for the local SQLite database and device-only key management through operating-system credential stores.
- Exclude E2EE synchronization records, an upload outbox, account authentication, device enrollment, and the synchronization server from the first implementation. Treat them as follow-up tasks governed by this ADR.
- The concrete local database encryption library, synchronization-record cryptographic primitives and serialization, and device-to-device key transfer protocol remain open design questions in this ADR.

## 理由 / Rationale

### 日本語

- サーバー侵害時に利用履歴の内容が漏れる範囲を暗号文へ限定できる。
- 運営者が利用履歴を閲覧できないことを、運用規則だけでなくシステム設計として保証できる。
- 復旧不能という制約を早期に明示し、利便性のために後から復号経路を追加してプライバシー保証を弱めることを防げる。

### English

- A server compromise exposes ciphertext rather than the contents of usage history.
- The inability of the operator to inspect usage history is enforced by system design rather than operational policy alone.
- Making unrecoverability explicit early prevents a later convenience feature from introducing a decryption path that weakens the privacy guarantee.

## 影響 / Consequences

### 日本語

- 現在の平文 SQLite スキーマから、暗号化された保存形式への移行が必要になる。
- サーバーは暗号文の内容を使った集計、検索、重複排除、障害調査を行えない。
- 集計表示には端末側での復号と計算が必要になる。
- 集計仕様を変更した場合でも、同期済みの利用セッションから新しい集計値を再生成できる。
- 利用セッション単位の同期は集計値だけを同期する方式よりレコード数と通信量が増える。
- 全端末の総利用時間は一人の経過時間として解釈でき、同時に複数端末を使っても過大計上しない。
- アプリ別時間は計測事実を失わず、端末間の恣意的な優先順位を必要としない一方、加算可能な内訳ではなくなる。
- 同じ製品の利用時間が OS ごとに分かれる可能性があるが、表示名の一致や不安定な実行ファイル情報による誤統合を避けられる。
- 別端末で同期履歴を表示する場合、同期元端末のアプリアイコンは利用できず、初期実装では汎用アイコンを表示する。
- 同期対象を最小化できる一方、端末内識別子から同期用識別子を安定して解決する対応表が必要になる。
- ローカルDBの検索・集計能力を維持できる一方、ローカルDB用鍵と同期用データ鍵のライフサイクルを個別に管理する必要がある。
- 同じ利用セッションについて、検索可能なローカル行とサーバー送信用暗号文の生成・同期状態を整合させる必要がある。
- 自動起動後も利用者の入力を待たずに暗号化された計測と同期を開始できる。
- OSユーザーとして動作できるプロセスや、解錠済み端末へアクセスできる人物からデータを保護する境界は提供しない。
- 端末追加時には両端末を同時に操作する必要があるが、カメラ、復旧コードまたはTime Wise専用パスフレーズは不要になる。
- サーバーは承認処理を妨害できるが、確認コードが一致する端末間通信へ検知されずに介入して同期用データ鍵を取得できないことをプロトコル要件とする。
- 「全履歴削除」という名称と、同期されたコピーを含めて削除されるという利用者の期待が一致する。
- オフラインまたは到達不能な端末上のコピーは即時に消去できず、その端末が次に同期した時点で削除される。改変されたクライアントや端末外へ複製されたデータの遠隔消去は保証できない。
- 削除端末が保持する過去の平文、暗号文および古い鍵は遠隔消去できないが、別経路で新しい暗号文を取得しても復号できなくなる。
- 残る承認済み端末は、過去の同期レコードを読むための旧鍵と、新しい同期レコードのための現行鍵を管理する必要がある。
- 過去履歴の一括再暗号化を避けられる一方、削除端末は保持済みの旧鍵に対応する過去の暗号文を引き続き復号できる。
- 自動起動などの端末固有設定が他端末の状態を意図せず変更することを避けられ、初期同期のデータモデルを小さく保てる。
- オフライン端末間で同じレコードを更新する競合が発生せず、サーバーは平文を読まずに再送を重複排除できる。
- 同期後の個別セッション訂正は提供できず、将来必要になった場合は訂正イベントなどの追記専用モデルが必要になる。
- サーバー侵害時に利用時刻や送信元端末を直接検索されることを避けられる一方、サーバーは認証された通信元、アップロード時刻、通信量を運用時に観測できる。
- 新しい端末の初回同期や長期間オフラインだった端末は、必要な期間だけをサーバーで選別できないため、多数の暗号文を取得して端末側で処理する必要がある。
- 同期遅延と帯域消費を抑えられる一方、通信パターンからTime Wiseが活動していた時刻や生成データ量を推測される可能性を受け入れる。
- 開発中の既存履歴は失われるが、平文DBから暗号化DBへの移行処理を製品コードへ持ち込まずに済む。
- 暗号化版を公開した後は、利用者データを保持するマイグレーションと復旧テストが必要になる。
- 最初の変更を現在の永続化境界へ限定でき、将来のサーバー設計を待たずに平文ローカル保存を解消できる。
- 認証情報の再設定と暗号化データの復旧は、同じ機能として扱えない。
- 鍵を失った利用者に対し、運営者が履歴を復旧できないことを事前に明示する必要がある。
- 最後の承認済み端末を失った後は、新しい端末を承認できず、サーバー上の過去データも復号できない。
- 暗号文であっても、アカウント、端末、通信時刻、データ量などのメタデータがサーバーから見える可能性は残る。

### English

- The existing plaintext SQLite schema requires migration to an encrypted storage format.
- The server cannot aggregate, search, deduplicate, or troubleshoot data by inspecting ciphertext contents.
- Aggregate views require client-side decryption and computation.
- Aggregate behavior can change without losing the ability to recompute results from synchronized usage sessions.
- Session-level synchronization creates more records and network traffic than synchronizing aggregates alone.
- All-device total usage represents one person's elapsed time and is not inflated by simultaneous device usage.
- Per-application usage preserves every observation and avoids arbitrary precedence between devices, but is no longer an additive breakdown of total usage time.
- Usage for one product may be split by operating system, but this avoids false merges based on matching display names or unstable executable information.
- A destination device cannot display an icon captured on the source device; the initial implementation uses a generic icon for synchronized history.
- Synchronization is data-minimized, but requires a stable mapping from on-device identities to opaque synchronization identifiers.
- Local database search and aggregation remain available, but the local database key and synchronization data key require separate lifecycle management.
- The implementation must keep the queryable local row and the generated server-bound ciphertext and synchronization state consistent for each usage session.
- Encrypted measurement and synchronization can begin after autostart without waiting for user input.
- This design does not protect data from a process acting as the operating-system user or a person with access to an unlocked device.
- Device enrollment requires simultaneous access to both devices, but does not require a camera, recovery code, or dedicated Time Wise passphrase.
- The protocol must allow the server to deny enrollment but not to intervene undetectably in a device channel whose authentication codes match and obtain the synchronization data key.
- The Delete all history label matches the user's expectation that synchronized copies are deleted as well.
- Copies on offline or unreachable devices cannot be erased immediately and are removed when those devices next synchronize. Remote erasure of data copied outside the device or retained by a modified client cannot be guaranteed.
- Plaintext, ciphertext, and old keys already held by a removed device cannot be erased remotely, but that device cannot decrypt new ciphertext obtained through another channel.
- Remaining authorized devices must manage old keys for historical records and a current key for new records.
- Avoiding bulk re-encryption simplifies device removal, but a removed device can continue to decrypt historical ciphertext corresponding to old keys it retains.
- Device-specific settings such as autostart cannot unintentionally change another device, and the initial synchronization model remains small.
- Offline devices do not create conflicting updates to one record, and the server can deduplicate retries without reading plaintext.
- Individual synchronized sessions cannot be edited in place; a future correction feature would require an append-only correction-event model.
- A server compromise does not expose directly searchable usage times or source devices, although the server can observe authenticated connections, upload times, and traffic volume during operation.
- A new device or a device returning after a long offline period must fetch and process many ciphertext records locally because the server cannot select records by usage period.
- Synchronization latency and bandwidth remain low, but traffic patterns may reveal when Time Wise was active and how much data it generated.
- Existing development history is lost, but the product does not need to carry a plaintext-to-encrypted database migration path.
- After the encrypted version is released, migrations that preserve user data and corresponding recovery tests are required.
- The first change remains within the existing persistence boundary and removes plaintext local storage without waiting for the future server design.
- Resetting account authentication and recovering encrypted data cannot be treated as the same operation.
- Users must be warned in advance that the operator cannot restore history after key loss.
- After the last authorized device is lost, no new device can be authorized and existing server-side data cannot be decrypted.
- Encryption does not necessarily hide metadata such as the account, device, transfer time, or ciphertext size from the server.

## 未決事項 / Open questions

### 日本語

- 確認コード付き端末間鍵転送に使用する、標準化され監査済みの具体的な暗号プロトコル。
- 対応OSごとの資格情報ストアAPI、端末限定属性、および利用不能時のエラー処理。
- アカウント認証情報を変更した場合の扱い。
- 将来、端末間で共有する設定を追加する場合の、端末固有設定との分類規則。
- 利用セッションを格納する暗号化レコードの形式と、ローカルでの検索・集計方法。
- ローカルDB全体の具体的な暗号化方式とライブラリ。
- ローカル行とE2EE同期レコードを一貫して生成するトランザクションおよび再試行方式。
- 端末の紛失を申告してから、サーバー認可と同期鍵世代の更新が完了するまでの操作フロー。
- 同期カーソル、取得順序、および大量の未同期レコードを分割送受信する方式。
- 将来、Windows と macOS のアプリを同一製品へ関連付ける場合の識別方法。
- 履歴世代の表現、削除要求の認証、およびサーバーバックアップから暗号文が物理削除される期限。
- サーバーが保持する暗号文とメタデータの保持期間。

### English

- The specific standardized and reviewed cryptographic protocol used for device-to-device key transfer with a short authentication code.
- Credential-store APIs, device-only attributes, and failure behavior for each supported operating system.
- What happens when account credentials change.
- How future settings shared between devices are distinguished from device-specific settings.
- The encrypted record format for usage sessions and how local search and aggregation operate.
- The concrete whole-database encryption construction and library.
- Transaction and retry behavior that consistently creates local rows and E2EE synchronization records.
- The operational flow from reporting a lost device through server revocation and synchronization-key generation advancement.
- Synchronization cursors, retrieval order, and batching large backlogs of unsynchronized records.
- How Windows and macOS applications are associated with one product if cross-platform grouping is added later.
- History-generation representation, deletion-request authentication, and the deadline for physically removing ciphertext from server backups.
- Retention periods for server-side ciphertext and metadata.
