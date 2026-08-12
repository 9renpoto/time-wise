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

同じアプリが継続してフォーカスされていた期間。画面ロックとスリープはセッションを中断する。日付変更や瞬間的なフォーカス喪失を境界とする詳細規則は未決。

A continuous period during which the same application was focused. Screen lock and sleep interrupt a session. Detailed boundary rules for date changes and momentary focus loss remain open.

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

同じアプリの計測サンプルをまとめ、アプリ名やアイコンを表示するための最小限の情報。実行ファイルの識別子、表示名、アイコン取得用の参照などを想定する。ウィンドウタイトル、文書名、メール件名、閲覧 URL、Web サイト名は含めない。表示情報だけが不足している場合は後から補完できる。

The minimum information needed to group samples from the same application and display its name and icon. It may include an executable identifier, display name, and an icon reference. It excludes window titles, document names, email subjects, browsing URLs, and website names. Missing display metadata may be added later.

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

## 全履歴削除 / Delete all history

端末内のすべての利用履歴を削除する操作。v1 では実行前に確認を求め、削除後の復元、期間指定、アプリ単位の削除は提供しない。

An operation that deletes all usage history from the device. v1 requires confirmation and does not provide recovery, date-range deletion, or application-specific deletion.

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
