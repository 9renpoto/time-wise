# ADR 0001: フォーカスアプリの利用時間を計測する / Track focused application usage

- 状態 / Status: 検討中 / Proposed
- 日付 / Date: 2026-07-27

## コンテキスト / Context

### 日本語

Time Wise は、既存のスクリーンタイムアプリを参考に、ユーザーがアプリを利用した時間を記録する。

現在の実装は、一定間隔で起動中のプロセスを取得し、プロセスが存在していた時間を利用時間として扱っている。この方法では、バックグラウンドで動作しているアプリも利用中として記録されるため、ユーザーが実際に操作していた時間を表せない。

OS ごとに取得できる情報や必要な権限が異なるため、具体的な取得方法はプラットフォームごとに選択する必要がある。

### English

Time Wise records how long users spend in applications, taking existing screen-time applications as product references.

The current implementation periodically enumerates running processes and treats process lifetime as usage time. This records background applications as active usage and therefore does not represent the applications the user was actually using.

Available information and required permissions vary by operating system, so each platform requires its own collection implementation.

## 決定 / Decision

### 日本語

- 基本的な計測対象を、前面でフォーカスされているアプリとする。単にプロセスが起動している時間は利用時間として扱わない。
- 画面ロック中およびスリープ中の時間を利用時間から除外する。キーボードやマウスの無操作時間だけを根拠としたアイドル除外は行わない。
- v1 の正式対応 OS と配布対象は Windows とする。Linux と macOS は、将来アダプターを追加できる境界だけを維持する。
- 利用時間はアプリ単位で集計する。ブラウザも Chrome、Edge、Firefox などのアプリ単位で扱い、Web サイト、ドメイン、タブ単位には分解しない。
- ウィンドウタイトルは収集および保存しない。アプリの識別と表示に必要な最小限の情報だけを扱う。
- 製品体験は macOS のスクリーンタイムを参考にする。ただし、Windows 向けの独立した実装とし、本 ADR の計測範囲とプライバシー方針を優先する。
- v1 は利用時間の計測と可視化に限定する。今日の合計、アプリ別ランキング、時間帯別グラフ、日別・週別の切り替え、過去データをダッシュボードに表示する。
- 利用制限、通知、カテゴリ分類は v1 に含めない。
- 利用履歴は端末内だけに保存し、自動削除せず無期限に保持する。サーバー同期を将来導入する場合は、サーバー側の保持期間、同意、削除、セキュリティを別途設計する。
- 設定画面から全利用履歴を削除できるようにし、実行前に確認を求める。期間指定、アプリ単位の削除、削除後の復元は提供しない。
- 計測時刻は UTC で保存し、計測時の Windows ローカルタイムゾーンも保存する。日別履歴は計測時点のローカル日付に固定し、週は月曜日から開始する。
- Windows のフォーカス変更イベントでアプリ切り替えを即時記録し、30 秒ごとに現在のセッションをチェックポイント保存する。内部では秒以下の精度を保持し、画面では分単位で表示する。
- Windows ログイン時の自動起動は、初回セットアップで説明し、ユーザーが明示的に有効化した場合だけ設定する。後から設定画面で変更できる。
- Time Wise の起動中はトレイに常駐して常時計測する。一時停止・再開機能は提供せず、アプリを終了すると計測を停止する。
- Time Wise 自身は常に計測対象から除外する。Explorer やタスクマネージャーなど、通常操作できる Windows アプリは原則として計測する。
- ユーザー向けの任意アプリ除外および URL・Web サイト単位の除外は提供しない。原理検証や自動テスト用の切り替えは、開発・テスト専用として実装してよい。
- 同じ製品の複数ウィンドウ、プロセス、ブラウザプロファイルは、一つのアプリとして合算する。画面には製品名と代表アイコンを表示する。
- 履歴と設定は、現在ログインしている Windows ユーザーのローカル領域へ保存し、別のユーザーアカウントとは共有しない。
- アプリ識別子を取得できない時間は「未分類」として総利用時間とアプリ別表示に残す。後から権限が得られても、過去時間を推測で再分類しない。
- 識別子を取得済みで、表示名やアイコンだけが不足している場合は、後からメタデータを補完して過去履歴の表示へ反映してよい。
- Windows 11 実機でフォーカス変更、画面ロック、スリープ、自動起動、トレイ動作を受け入れ確認する。
- GitHub Actions では、Ubuntu で OS 非依存の中核を検証し、Windows でワークスペース全体をビルドおよびテストする。リリース成果物は Windows 向けだけを生成する。

### English

- Measure the application currently focused in the foreground. Do not treat the lifetime of a running process as usage time.
- Exclude time while the screen is locked or the computer is asleep. Do not exclude time solely because there has been no keyboard or mouse input.
- Support and distribute v1 for Windows. Preserve boundaries that allow Linux and macOS adapters to be added later.
- Aggregate usage by application. Treat Chrome, Edge, Firefox, and other browsers as applications; do not break usage down by website, domain, or tab.
- Do not collect or store window titles. Handle only the minimum information required to identify and display an application.
- Use macOS Screen Time as a product-experience reference, while building an independent Windows implementation governed by the scope and privacy rules in this ADR.
- Limit v1 to measurement and visualization. Show today's total, application rankings, an hourly chart, daily and weekly views, and historical data.
- Exclude usage limits, notifications, and category classification from v1.
- Store history only on the device, retain it indefinitely, and do not synchronize it with a server. Define separate retention, consent, deletion, and security rules before adding server synchronization.
- Allow users to delete all usage history from Settings after confirmation. Do not provide deletion by date range or application, or recovery after deletion.
- Store timestamps in UTC together with the Windows local time zone at measurement time. Assign history permanently to the local date observed at measurement time, and start weeks on Monday.
- Record application switches immediately from Windows foreground-change events. Checkpoint the current session every 30 seconds. Retain sub-second internal precision and display usage in minutes.
- Explain autostart during onboarding and enable it only after explicit user consent. Allow the setting to be changed later.
- Measure continuously while Time Wise is running in the system tray. Do not provide pause and resume states. Stop measurement when the application exits.
- Always exclude Time Wise itself. Measure ordinarily interactive Windows applications such as Explorer and Task Manager by default.
- Do not provide user-facing exclusion rules for applications, URLs, or websites. Development-only switches may be used for proof-of-concept work and automated tests.
- Combine multiple windows, processes, and browser profiles belonging to the same product into one application. Display one product name and representative icon.
- Store history and settings in the current Windows user's local data area and do not share them with other Windows accounts.
- Keep time for which no application identifier could be obtained as Unclassified in both totals and application views. Do not infer a historical classification after permissions change.
- If an identifier was captured but its display name or icon was unavailable, metadata may be added later and reflected in historical views.
- Perform acceptance testing for focus changes, screen lock, sleep, autostart, and tray behavior on physical Windows 11 hardware.
- Use GitHub Actions to test the OS-independent core on Ubuntu and the complete workspace on Windows. Produce release artifacts for Windows only.

## 理由 / Rationale

### 日本語

- 一般的なスクリーンタイムの意味に近い計測結果になる。
- バックグラウンド常駐アプリによる過大計上を避けられる。
- アプリ別時間をユーザーが直感的に理解しやすい。
- 動画視聴、文書の閲覧、オンライン会議など、入力操作を伴わない利用を計測できる。

### English

- The result more closely matches the common meaning of screen time.
- Background applications do not inflate recorded usage.
- Application-level totals are easier for users to understand.
- Passive but intentional activity, including watching video, reading, and attending online meetings, remains measurable.

## 影響 / Consequences

### 日本語

- 現在のプロセス一覧ベースの計測処理を置き換えるか、役割を限定する必要がある。
- Windows 固有の取得処理をインフラストラクチャ層へ閉じ込め、ドメイン層とアプリケーション層から分離する必要がある。
- デスクトップアプリとパッケージアプリを、安定した製品単位へ解決する識別処理が必要になる。
- ブラウザ内の Web サイト別時間、同じ製品のプロファイル別・ウィンドウ別時間は確認できない。
- 文書名、メール件名、閲覧ページ名などがウィンドウタイトルを通じて保存されることを防げる一方、タイトルに依存した識別はできない。
- ロックせずに離席した時間は利用時間に含まれる。将来アイドル時間を推定する場合も、メディア視聴などを誤って除外しない設計が必要になる。
- ローカルデータ量と集計性能を監視する必要がある。同期を追加するまでは複数端末を横断した集計や復元はできない。
- 全履歴削除は取り消せないため、対象と結果を確認画面で明確に伝える必要がある。
- タイムゾーン変更後も過去の日別集計は安定するが、UTC 時刻に加えて計測時のタイムゾーンまたは確定したローカル日付を永続化する必要がある。
- イベント購読の停止を検知する必要がある。異常終了時に失われる直近の計測時間は、原則として最大約 30 秒になる。
- Windows のユーザー切り替え時は、各ユーザーのプロセスと保存領域を独立して扱う必要がある。
- 未分類時間により総利用時間の欠落を防げるが、識別できなかった過去時間は権限取得後も未分類として残る。
- OS イベントを伴う動作は CI だけでは保証できないため、Windows 11 実機での受け入れ確認が必要になる。
- OS 非依存ロジックを Windows 固有コードから分離し、Ubuntu CI でも検証可能にする必要がある。

### English

- The current process-enumeration recorder must be replaced or given a narrower role.
- Windows-specific collection must remain in the infrastructure layer, separate from domain and application logic.
- Desktop and packaged Windows applications require resolution to stable product-level identities.
- The product will not report usage by website, browser profile, or individual window.
- Avoiding window titles prevents storage of document names, email subjects, and page titles, but also prevents title-based identification.
- Time spent away from an unlocked computer remains usage time. Any future idle-time estimation must avoid excluding passive media consumption.
- Local data volume and query performance require monitoring. Cross-device aggregation and recovery are unavailable until synchronization is added.
- Deleting all history is irreversible, so the confirmation must clearly communicate its scope and result.
- Historical daily totals remain stable after time-zone changes, but the system must persist either the measurement-time zone or the finalized local date in addition to UTC timestamps.
- The recorder must detect stopped event subscriptions. A crash should lose no more than approximately 30 seconds of recent usage under normal conditions.
- Each signed-in Windows user requires an independent process context and data location.
- Unclassified time prevents gaps in the total, but historically unidentified time remains unclassified after permissions change.
- CI alone cannot validate operating-system events, so acceptance testing on physical Windows 11 hardware is required.
- OS-independent logic must remain separate from Windows-specific code so it can also be tested in Ubuntu CI.

## 未決事項 / Open questions

### 日本語

- 日付変更や瞬間的なフォーカス喪失を利用セッション境界として扱う詳細規則。
- OS 権限がない場合の表示とフォールバック。
- OS 権限やイベント購読が失敗した場合のユーザー向けエラー表示。

### English

- Detailed session-boundary rules for date changes and momentary focus loss.
- Display and fallback behavior when OS permissions are unavailable.
- User-facing errors for permission failures and failed event subscriptions.
