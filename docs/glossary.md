# Time Wise 用語集 / Glossary

Time Wise の要件、実装、画面表示で使用する用語を定義する。未決の定義は、要件整理の進行に合わせて更新する。

This reference defines terminology used in Time Wise requirements, implementation, and user interfaces. Definitions that remain open will be updated as requirements are refined.

## フォーカスアプリ / Focused application

ユーザーのデスクトップで前面にあり、現在入力を受け取る対象となっているアプリ。

The foreground application that is currently eligible to receive user input.

## 利用時間 / Usage time

アプリがフォーカスアプリとして観測された時間を基礎に算出する時間。バックグラウンドで起動しているだけの時間、画面ロック中、スリープ中は含めない。入力操作がなくても、フォーカスアプリが表示され、画面が利用可能な時間は含める。

Time calculated from periods in which an application was observed as focused. It excludes time when a process was merely running in the background, the screen was locked, or the computer was asleep. It includes time when the focused application was visible and the screen was available, even without input activity.

## 計測サンプル / Measurement sample

ある時点で観測したフォーカスアプリの識別情報と観測時刻。フォーカス変更イベントで取得し、30 秒ごとのチェックポイントでも現在の状態を確認する。

The focused application's identity and observation time at a particular moment. Samples come from foreground-change events, with the current state also checked at 30-second checkpoints.

## 利用セッション / Usage session

同じアプリが継続してフォーカスされていた期間。画面ロックとスリープはセッションを中断する。日付変更や瞬間的なフォーカス喪失を境界とする詳細規則は未決。将来の複数端末同期では、個々の利用セッションを暗号化された同期元データとし、日別・週別・アプリ別の集計値は承認済み端末上で再生成する案を検討する。

A continuous period during which the same application was focused. Screen lock and sleep interrupt a session. Detailed boundary rules for date changes and momentary focus loss remain open. For future multi-device synchronization, Time Wise is considering individual usage sessions as encrypted source records, with authorized devices recomputing daily, weekly, and per-application aggregates.

## アイドル状態 / Idle state

アプリがフォーカスされていても、ユーザー入力が一定時間観測されていない状態。動画視聴、文書の閲覧、オンライン会議など、入力操作を伴わない利用も含み得る。v1 では利用時間から除外しない。

A state in which an application remains focused but no user input has been observed for a period. This can include intentional use without input, such as watching video, reading, or attending an online meeting. v1 does not exclude this state from usage time.

## 画面ロック / Screen lock

OS のユーザーセッションがロックされ、アプリを操作できない状態。この期間は利用時間から除外する。

A state in which the operating-system user session is locked and applications cannot be used. This period is excluded from usage time.

## スリープ / Sleep

コンピューターまたは対象セッションの動作が一時停止している状態。この期間は利用時間から除外する。

A state in which the computer or relevant session is suspended. This period is excluded from usage time.

## 正式対応 OS / Supported operating system

リリース時に計測精度と主要機能を検証し、動作を保証する OS。v1 では Windows と macOS を正式対応 OS および配布対象とする。Linux は正式対応対象外とし、将来アダプターを追加できるアーキテクチャ境界だけを維持する。

An operating system on which measurement accuracy and primary features are validated for release. Windows and macOS are the supported and distributed platforms for v1. Linux is not officially supported and retains only an architectural boundary for a future adapter.

## アプリ / Application

利用時間を集計する単位。対応 OS 上で安定して取得できる識別情報を基に同一性を判定する。ブラウザはブラウザ自体を一つのアプリとして扱い、Web サイト、ドメイン、タブには分けない。同じ製品の複数ウィンドウ、プロセス、ブラウザプロファイルも一つに合算する。

The unit used to aggregate usage time, identified from stable information available on each supported operating system. A browser is treated as one application rather than being divided by website, domain, or tab. Multiple windows, processes, and browser profiles belonging to the same product are combined.

## アプリ識別情報 / Application identity

同じアプリの計測サンプルをまとめ、アプリ名やアイコンを表示するための最小限の情報。実行ファイルの識別子、表示名、アイコン取得用の参照などを想定する。ウィンドウタイトル、文書名、メール件名、閲覧 URL、Web サイト名は含めない。表示情報だけが不足している場合は後から補完できる。初期の複数端末同期では OS 固有の識別子を維持し、異なる OS 上の同じ製品を推測で統合しない。

The minimum information needed to group samples from the same application and display its name and icon. It may include an executable identifier, display name, and an icon reference. It excludes window titles, document names, email subjects, browsing URLs, and website names. Missing display metadata may be added later. Initial multi-device synchronization preserves platform-specific identities and does not speculatively merge the same product across operating systems.

## 同期用アプリ識別子 / Synchronization application identifier

同期された利用セッションから同じアプリの記録をまとめるための不透明な安定識別子。実行ファイルパスやアイコン取得元などの端末固有情報を値に含めない。端末内のアプリ識別情報との対応は同期元端末が管理する。

An opaque stable identifier used to group records for the same application in synchronized usage sessions. Its value does not contain device-specific information such as executable paths or icon sources. The source device manages its association with the on-device application identity.

## 未分類 / Unclassified

フォーカスアプリの識別子を取得できず、特定のアプリへ割り当てられない利用時間。総利用時間へ含め、アプリ別表示では独立した項目として扱う。後から権限が得られても過去分を推測で再分類せず、新しいセッションから通常の分類を始める。

Usage time for which no focused-application identifier could be obtained. It is included in total usage and shown as a separate application-level item. Historical time is not reclassified by inference after permissions change; normal classification begins with new sessions.

## 受け入れ確認 / Acceptance testing

Windows 11 実機と macOS 実機でフォーカス変更、画面ロック、スリープ、自動起動、トレイ動作を確認し、v1 の要件を満たすか判断する手動検証。自動ビルドと自動テストは GitHub Actions で実行する。

Manual validation on both physical Windows 11 and physical macOS hardware to determine whether focus changes, screen lock, sleep, autostart, and tray behavior satisfy v1 requirements. Automated builds and tests run in GitHub Actions.

## ポータブル中核 / Portable core

特定 OS の API に依存しないドメインモデル、利用セッション処理、SQLite 永続化、日別・週別集計。Windows CI と macOS CI に加え、Ubuntu CI でも自動テストする。

Domain models, usage-session processing, SQLite persistence, and daily and weekly aggregation that do not depend on a specific operating-system API. The portable core is tested in Windows, macOS, and Ubuntu CI.

## 計測と可視化 / Measurement and visualization

v1 の中核機能。フォーカスアプリの利用時間を記録し、今日の合計、アプリ別ランキング、時間帯別グラフ、日別・週別表示、過去データとして確認できるようにする。利用制限、通知、カテゴリ分類は含まない。

The core v1 capability. It records focused-application usage and presents today's total, application rankings, an hourly chart, daily and weekly views, and historical data. It excludes usage limits, notifications, and category classification.

## ローカル履歴 / Local history

OS のユーザーアカウントごとの端末内領域に保存された利用履歴。v1 ではサーバーへ送信せず、自動削除も行わず、無期限に保持する。将来サーバー同期を導入する場合は、別の保持規則を定める。

Usage history stored on the device for an individual operating-system user account. v1 does not send it to a server, delete it automatically, or share it with other accounts, and retains it indefinitely. Server synchronization will require separate retention rules.

## エンドツーエンド暗号化 / End-to-end encryption

利用データを送信元の承認済み端末で暗号化し、受信先の承認済み端末でだけ復号する保護方式。同期サーバーは暗号文を保存・配信するが、利用データの平文または復号可能な鍵を保持しない。アカウント、端末、通信時刻、データ量などのメタデータまで隠すことは意味しない。初期版は暗号文のパディングやダミー通信を行わない。

A protection model in which an authorized source device encrypts usage data and only authorized destination devices decrypt it. The synchronization server stores and distributes ciphertext but does not possess usage-data plaintext or a key capable of decrypting it. It does not imply that metadata such as the account, device, transfer time, or data size is hidden. The initial implementation does not pad ciphertext or generate dummy traffic.

## 暗号化ローカルDB / Encrypted local database

端末内の利用履歴を検索・集計可能なリレーショナル形式で保持しながら、保存ファイル全体をローカルDB用鍵で暗号化したSQLiteデータベース。同期用暗号文とは別の保存表現であり、DBファイルやその鍵をサーバーへ送信しない。

An on-device SQLite database whose entire storage file is encrypted with a local database key while preserving a relational form suitable for search and aggregation. It is distinct from synchronization ciphertext; neither the database file nor its key is sent to the server.

## E2EE同期レコード / E2EE synchronization record

一つの利用セッションと同期に必要な最小限のアプリ情報から生成し、同期用データ鍵で暗号化する不変の追記専用レコード。送信元端末が推測困難な一意IDを付与し、サーバーはそのIDで再送を冪等に処理する。端末内のSQLiteデータベース自体は同期しない。

An immutable, append-only encrypted record generated from one usage session and the minimum application information required for synchronization. The source device assigns an unpredictable unique identifier that the server uses to process retries idempotently. The on-device SQLite database itself is not synchronized.

## サーバー可視メタデータ / Server-visible metadata

同期サーバーが復号せずに保存または観測できる情報。同期レコードではアカウントへの所属、無作為なレコードID、履歴世代、同期鍵世代、同期順序および暗号文サイズに限定する。利用日時、アプリおよび送信元端末は含めない。ただし、認証された通信元、アップロード時刻および通信量は運用時に観測できる。

Information that the synchronization server can store or observe without decryption. For synchronization records it is limited to account membership, a random record identifier, history generation, synchronization key generation, synchronization order, and ciphertext size. It excludes usage time, application, and source device, although the server can observe authenticated connections, upload time, and traffic volume during operation.

## 承認済み端末 / Authorized device

利用者の暗号化データを復号するための鍵情報を正当に取得した端末。新しい端末は既存の承認済み端末による明示的な承認を受ける必要があり、アカウントへのログインだけでは承認済み端末にならない。削除、失効および鍵共有の具体的な方式は未決。

A device that has legitimately obtained the key material required to decrypt a user's encrypted data. A new device requires explicit approval from an existing authorized device; signing in to the account alone does not authorize it. The mechanisms for removal, revocation, and key sharing remain undecided.

## 端末鍵 / Device key

承認済み端末を暗号学的に識別し、その端末がローカルDB用鍵と同期用データ鍵を利用できるようにする端末固有の鍵情報。OSの資格情報保護機構へ他端末に同期されない形で保存し、OSユーザーへのログイン後にTime Wiseが自動解錠する。

Device-specific key material that cryptographically identifies an authorized device and enables it to use the local database key and synchronization data key. It is stored in the operating system's credential protection facility without cross-device synchronization and is unlocked automatically by Time Wise after operating-system user login.

## 同期鍵世代 / Synchronization key generation

同期用データ鍵のローテーション境界を表す値。承認済み端末を削除すると新しい世代へ進み、以後の同期レコードは残る承認済み端末だけが取得できる新しい鍵で暗号化する。過去レコードは再暗号化せず、残る承認済み端末が旧鍵を保持する。

A value identifying a rotation boundary for the synchronization data key. Removing an authorized device advances the generation, and subsequent synchronization records are encrypted with a new key available only to the remaining authorized devices. Historical records are not re-encrypted; remaining authorized devices retain the old keys.

## 端末別表示 / Per-device view

同期済み履歴のうち、選択した一台の端末で計測した利用セッションだけを表示・集計する表示範囲。他端末との同時利用による重複排除は行わない。

A view that displays and aggregates only synchronized usage sessions measured by one selected device. It does not deduplicate time that overlaps with usage on another device.

## 全端末表示 / All-device view

すべての承認済み端末で計測した同期済み履歴を表示・集計する表示範囲。総利用時間では、複数端末の利用セッションが重なる時間区間を一度だけ数える。アプリ別時間は各端末の観測をすべて計上するため、その合計が総利用時間を超える場合がある。

A view that displays and aggregates synchronized history measured by every authorized device. Its total usage counts intervals that overlap across devices only once. Per-application usage includes every device's observations, so its sum may exceed total usage time.

## 復旧不能 / Unrecoverable

復号に必要な認証情報と鍵を保持する承認済み端末をすべて失ったため、暗号化された利用データを誰も復号できない状態。Time Wise の運営者は復旧用のマスターキーを保持せず、データ復旧を提供しない。

A state in which nobody can decrypt the encrypted usage data because the credentials required for decryption and every authorized device holding key material have been lost. The Time Wise operator holds no recovery master key and cannot provide data recovery.

## 全履歴削除 / Delete all history

すべての利用履歴を削除する操作。v1 では端末内だけを対象とする。将来、同期を有効にした場合はアカウント全体へ適用し、サーバーとすべての承認済み端末にある同期対象の履歴を削除する。オフライン端末上のコピーは、その端末が次に同期した時点で削除する。実行前に確認を求め、削除後の復元、期間指定、アプリ単位の削除は提供しない。

An operation that deletes all usage history. In v1 it applies only to the device. After synchronization is introduced, it becomes account-wide and removes synchronized history from the server and every authorized device. A copy on an offline device is deleted when that device next synchronizes. The operation requires confirmation and does not provide recovery, date-range deletion, or application-specific deletion.

## 履歴世代 / History generation

全履歴削除の境界を表す単調に進む値。サーバーは現在より古い履歴世代の同期レコードを拒否し、オフライン端末が削除済み履歴を再送信して復活させることを防ぐ。

A monotonically advancing value that marks a Delete all history boundary. The server rejects synchronization records from older generations so an offline device cannot upload and restore deleted history.

## 計測日 / Measurement date

利用時間を日別に集計する日付。計測時点の OS ローカル日付を使用し、後から端末のタイムゾーンが変わっても過去の計測日は変更しない。

The date used for daily usage aggregation. It is the operating-system local date at measurement time and does not change retrospectively when the device's time zone changes.

## 週 / Week

月曜日から日曜日までの計測日のまとまり。週別表示の集計単位として使用する。

A group of measurement dates from Monday through Sunday, used as the aggregation unit for weekly views.

## チェックポイント / Checkpoint

進行中の利用セッションを定期的に永続化する処理。v1 では 30 秒ごとに保存し、異常終了時の未保存時間を制限する。

Periodic persistence of an in-progress usage session. v1 saves a checkpoint every 30 seconds to limit uncommitted time after an abnormal exit.

## 自動起動 / Autostart

OS へのログイン時に Time Wise を起動し、トレイ常駐で計測を始める設定。初回セットアップで説明し、ユーザーが明示的に有効化した場合だけ使用する。後から設定画面で変更できる。

A setting that starts Time Wise at operating-system sign-in and begins measurement in the system tray. It is explained during onboarding, enabled only with explicit user consent, and can be changed later in Settings.

## 計測継続状態 / Continuous measurement

Time Wise が起動し、フォーカスアプリの利用時間を記録している状態。v1 では一時停止状態を設けず、アプリが終了するまで計測を継続する。

The state in which Time Wise is running and recording focused-application usage. v1 has no paused state and continues measurement until the application exits.

## 常時除外アプリ / Always-excluded application

フォーカスされても利用時間へ記録しないアプリ。v1 では Time Wise 自身だけを常時除外する。ユーザーが任意のアプリや URL を追加する除外設定は提供しない。検証用の切り替えは製品機能に含めない。

An application whose focused time is never recorded. In v1, only Time Wise itself is always excluded. Users cannot add applications or URLs to an exclusion list, and validation switches are not product features.
