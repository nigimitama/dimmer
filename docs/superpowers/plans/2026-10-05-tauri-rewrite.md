# Dimmer Tauri 書き直し 実装計画

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Python/tkinter 製のモニター輝度管理アプリ Dimmer を、Tauri v2 + React + TypeScript で書き直す。スリープ復帰後もスケジュールが正しく効くようにする。

**Architecture:** ロジックと状態はすべて Rust 側に置く。Win32 API による DDC/CI 制御、純粋関数のスケジュール計算、30秒ごとの tick で判定するスケジューラー、JSON の設定ファイル、トレイがそれにあたる。React（Fluent UI v9）は Tauri の command で操作を送り、`monitors-updated` event を受けて描画するだけにする。スケジューラーは「現在のスロット」と「適用済みのスロット」を比べる方式で、スリープ復帰を壁時計の飛びで検知する。

**Tech Stack:** Tauri 2 / Rust（`windows` 0.62、chrono、serde）/ React 19 + TypeScript + Vite / Fluent UI React v9 / Vitest / pnpm

**Spec:** `docs/superpowers/specs/2026-10-05-tauri-rewrite-design.md`（実装者はこの計画と spec の両方を読むこと）

## Global Constraints

- 対象 OS は Windows 10/11 のみ。他の OS でビルドできなくてよい
- パッケージマネージャーは pnpm（corepack 経由）
- アプリのバージョンは `1.0.0`。`package.json`、`src-tauri/Cargo.toml`、`src-tauri/tauri.conf.json` で一致させる
- Tauri の identifier は `io.github.nigimitama.dimmer`。productName は `Dimmer`
- 設定ファイルは `app_config_dir()/settings.json`（`%APPDATA%\io.github.nigimitama.dimmer\settings.json`）
- 設定ファイルの形式：`{ "version": 1, "autostart": bool, "schedule": [{ "time": "HH:MM", "brightness": 0..=100 }] }`
- デフォルトのスケジュール：06:00→70, 07:00→80, 08:00→90, 09:00→100, 17:00→90, 18:00→80, 19:00→60, 20:00→40, 21:00→30, 22:00→10, 23:00→0
- `autostart` のデフォルトは `true`
- スケジューラーの tick 間隔は 30秒、復帰とみなす間隔は 90秒以上（または時刻の巻き戻り）、リトライの間隔は 5秒→15秒、全台の失敗で諦めるまでの tick 数は 10
- 輝度スライダーは 0〜100、10刻み、100ms のデバウンス
- ウィンドウは 520×640
- 自動起動で起動したときの引数は `--autostart`。このときはウィンドウを表示しない
- バンドルは NSIS のみ。コード署名はしない
- UI の文言は日本語

## Review Focus

spec に明記されていないものの、使う人が実際に遭遇しやすい入力と失敗のパターン。それぞれ、担当するタスクにテストを入れてある。

1. **ノート PC の内蔵ディスプレイなど DDC/CI に対応していないモニターが混ざっている**：外部モニターに適用できれば適用済みとし、永遠にリトライし続けない。内蔵ディスプレイは「読み取れません」と表示し、「すべて」を動かしてもエラーのトーストは出さない → Task 6（`record_result`）、Task 4（`read_all`、`apply_all`）、Task 10（`reportApply`）
2. **同じ機種のモニターが2台ある**：ID が別になり、名前は「DELL U2720Q (1)」「DELL U2720Q (2)」のように区別される。個別スライダーで正しい1台だけが変わる → Task 5（`build_idents`）、Task 4（`apply_one`）
3. **スケジュールの数値入力に空文字、`abc`、`150`、`7.5`、` 50 ` が入る**：該当行にエラーを表示し、保存しない（` 50 ` は 50 として受け付ける）。アプリは落ちない → Task 9（`validateDraft`）
4. **OS の時刻を手動で戻した、またはタイムゾーンを変えた**：復帰と同じ扱いで直近の値を再適用する → Task 6（`clock_going_backwards_reapplies`）
5. **スケジュールが空、または1件だけ**：空ならスケジューラーは何もせず、グラフの代わりに「スケジュールがありません」と表示する。1件ならその値が一日中続く平らなグラフになる → Task 2、Task 6（`empty_schedule_does_nothing`）、Task 9（`stepPoints`）、Task 11（`ScheduleChart`）

---

## ファイル構成

```
dimmer/
├─ package.json / pnpm-lock.yaml / vite.config.ts / vitest.config.ts / eslint.config.js / tsconfig*.json / index.html
├─ src/
│  ├─ main.tsx                 FluentProvider（OS テーマ追従）と App のマウント
│  ├─ App.tsx                  画面全体。状態（monitors, settings）とトーストを持つ
│  ├─ api.ts                   command / event の型付きラッパー
│  ├─ hooks/
│  │  ├─ useDebouncedCallback.ts
│  │  ├─ useIsDark.ts
│  │  └─ useNowMinutes.ts
│  ├─ lib/
│  │  ├─ schedule.ts           検証・スロット計算・グラフの頂点計算（純粋関数）
│  │  └─ schedule.test.ts
│  └─ components/
│     ├─ BrightnessPanel.tsx   「すべて」と各モニターのスライダー
│     ├─ SchedulePanel.tsx     見出し・グラフ・リスト・ボタン
│     ├─ ScheduleChart.tsx     24時間の階段グラフ（SVG）
│     ├─ ScheduleList.tsx      編集用のリスト
│     └─ SettingsDialog.tsx    自動起動のトグルとバージョン
└─ src-tauri/
   ├─ Cargo.toml / tauri.conf.json / build.rs / capabilities/default.json / icons/
   ├─ examples/list_monitors.rs   実機でモニターの ID・名前・輝度を確認するための例
   └─ src/
      ├─ main.rs                 lib の run() を呼ぶだけ（scaffold のまま）
      ├─ lib.rs                  Builder の組み立て（プラグイン、setup、window event）
      ├─ state.rs                AppState
      ├─ schedule.rs             ScheduleEntry、スロット計算、検証、デフォルト
      ├─ settings.rs             Settings、読み書き、SettingsStore
      ├─ monitor/mod.rs          MonitorBackend トレイト、read_all / apply_* / apply_with_retry
      ├─ monitor/mock.rs         テスト用の MockBackend（#[cfg(test)]）
      ├─ monitor/win32.rs      WinBackend（Win32 実装）と build_idents
      ├─ scheduler.rs            SchedulerState、run_once、spawn
      ├─ commands.rs             Tauri command
      └─ tray.rs                 トレイとウィンドウの表示切り替え
```

---

### Task 1: 開発環境と Tauri プロジェクトの雛形

**Files:**
- Create: `package.json`、`pnpm-lock.yaml`、`index.html`、`vite.config.ts`、`tsconfig.json`、`tsconfig.node.json`、`src/`（雛形）、`src-tauri/`（雛形一式）
- Modify: `.gitignore`

**Interfaces:**
- Produces: `pnpm tauri dev` で起動する空のアプリ。Rust の lib crate 名は `dimmer_lib`

- [ ] **Step 1: Rust と pnpm を入れる**

PowerShell で実行する。MSVC Build Tools 2022 と WebView2 はインストール済みであることを確認済み。

```powershell
winget install --id Rustlang.Rustup -e --accept-source-agreements --accept-package-agreements
# 新しいシェルを開き直してから
rustup default stable
rustc --version
corepack enable
corepack prepare pnpm@latest --activate
pnpm --version
```

期待する結果：`rustc 1.xx` と pnpm のバージョンが表示される。`corepack enable` が権限エラーになる場合は、管理者の PowerShell で実行するようユーザーに頼む。

- [ ] **Step 2: 雛形を一時ディレクトリに生成する**

リポジトリのルートは空ではないため、サブディレクトリに生成してから移す。

```bash
cd /c/Users/nigim/dimmer
pnpm create tauri-app@latest scaffold --template react-ts --manager pnpm --identifier io.github.nigimitama.dimmer --yes
```

対話プロンプトが出た場合の答え：project name=`dimmer`、identifier=`io.github.nigimitama.dimmer`、language=TypeScript / JavaScript、package manager=pnpm、UI template=React、flavor=TypeScript。

- [ ] **Step 3: 雛形をルートへ移す**

```bash
cd /c/Users/nigim/dimmer
cat scaffold/.gitignore >> .gitignore
rm -f scaffold/.gitignore scaffold/README.md
rm -rf scaffold/.vscode
cp -r scaffold/. .
rm -rf scaffold
git status --short
```

期待する結果：`package.json`、`index.html`、`src/`、`src-tauri/`、`vite.config.ts`、`tsconfig*.json`、`public/` が追加されている。既存の `assets/`、`README.md`、`.vscode/` は変わっていない。

- [ ] **Step 4: 雛形の不要なものを外す**

`src-tauri/Cargo.toml` から `tauri-plugin-opener` の行を消す。`src-tauri/src/lib.rs` から `.plugin(tauri_plugin_opener::init())` を消す。`src-tauri/capabilities/default.json` の `permissions` から `"opener:default"` を消す。`package.json` の dependencies から `@tauri-apps/plugin-opener` を消す。

- [ ] **Step 5: アプリの情報とウィンドウを設定する**

`src-tauri/tauri.conf.json` を次のように編集する（`$schema`、`build` セクションは雛形のまま残す）。

```json
{
  "productName": "Dimmer",
  "version": "1.0.0",
  "identifier": "io.github.nigimitama.dimmer",
  "app": {
    "windows": [
      {
        "label": "main",
        "title": "Dimmer",
        "width": 520,
        "height": 640,
        "minWidth": 440,
        "minHeight": 480,
        "visible": false
      }
    ],
    "security": { "csp": null }
  },
  "bundle": {
    "active": true,
    "targets": ["nsis"],
    "icon": [
      "icons/32x32.png",
      "icons/128x128.png",
      "icons/128x128@2x.png",
      "icons/icon.icns",
      "icons/icon.ico"
    ]
  }
}
```

`visible: false` にしているのは、Task 8 で「自動起動のときは表示しない」を実装するため。それまでの間、開発中はウィンドウが出ないので、この Step では一時的に `true` にして確認し、**コミット前に `false` に戻す**。

`package.json` の `"version"` を `"1.0.0"` に、`"name"` を `"dimmer"` にする。`src-tauri/Cargo.toml` の `[package]` の `version` を `"1.0.0"`、`description` を `"Monitor brightness manager"` にする。

- [ ] **Step 6: アイコンを生成する**

```bash
pnpm tauri icon assets/icon-dark.png
```

期待する結果：`src-tauri/icons/` のファイルが上書きされる。

- [ ] **Step 7: 起動を確認する**

```bash
pnpm install
pnpm tauri dev
```

期待する結果：「Dimmer」というタイトルの 520×640 のウィンドウが開き、雛形の画面が表示される。確認したらウィンドウを閉じ、`tauri.conf.json` の `visible` を `false` に戻す。

- [ ] **Step 8: コミットする**

```bash
git add -A
git status --short   # .superpowers/ や node_modules/ や src-tauri/target/ が含まれていないことを確認する
git commit -m "chore: scaffold Tauri + React TypeScript app"
```

---

### Task 2: スケジュールの計算と検証（`schedule.rs`）

**Files:**
- Create: `src-tauri/src/schedule.rs`
- Modify: `src-tauri/Cargo.toml`、`src-tauri/src/lib.rs`

**Interfaces:**
- Produces:
  - `pub struct ScheduleEntry { pub time: String, pub brightness: u8 }`（`Clone, Debug, PartialEq, Eq, Serialize, Deserialize`）
  - `pub struct Slot { pub id: NaiveDateTime, pub brightness: u8 }`（`Clone, Copy, Debug, PartialEq, Eq`）
  - `pub fn current_slot(entries: &[ScheduleEntry], now: NaiveDateTime) -> Option<Slot>`
  - `pub fn next_slot(entries: &[ScheduleEntry], now: NaiveDateTime) -> Option<Slot>`
  - `pub struct EntryError { pub index: usize, pub message: String }`
  - `pub fn normalize(entries: &[ScheduleEntry]) -> Result<Vec<ScheduleEntry>, Vec<EntryError>>`（検証して時刻順に並べる）
  - `pub fn format_errors(errors: &[EntryError]) -> String`
  - `pub fn default_schedule() -> Vec<ScheduleEntry>`

- [ ] **Step 1: 依存を追加する**

`src-tauri/Cargo.toml` の `[dependencies]` に追加する（`serde` と `serde_json` は雛形にあるはずなので、なければ追加する）。

```toml
chrono = "0.4"
log = "0.4"
serde = { version = "1", features = ["derive"] }
serde_json = "1"
```

`[dev-dependencies]` セクションを追加する。

```toml
[dev-dependencies]
tempfile = "3"
```

`src-tauri/src/lib.rs` の先頭に `pub mod schedule;` を追加する。

- [ ] **Step 2: 失敗するテストを書く**

`src-tauri/src/schedule.rs` を作り、まずテストだけを書く。

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use chrono::NaiveDate;

    fn e(time: &str, brightness: u8) -> ScheduleEntry {
        ScheduleEntry { time: time.into(), brightness }
    }

    fn at(day: u32, h: u32, m: u32, s: u32) -> NaiveDateTime {
        NaiveDate::from_ymd_opt(2026, 10, day).unwrap().and_hms_opt(h, m, s).unwrap()
    }

    fn sample() -> Vec<ScheduleEntry> {
        vec![e("06:00", 70), e("18:00", 80), e("23:00", 0)]
    }

    #[test]
    fn current_slot_is_latest_entry_not_after_now() {
        let slot = current_slot(&sample(), at(5, 19, 30, 0)).unwrap();
        assert_eq!(slot, Slot { id: at(5, 18, 0, 0), brightness: 80 });
    }

    #[test]
    fn current_slot_includes_exact_boundary() {
        let slot = current_slot(&sample(), at(5, 18, 0, 0)).unwrap();
        assert_eq!(slot.id, at(5, 18, 0, 0));
    }

    #[test]
    fn current_slot_before_first_entry_uses_previous_days_last_entry() {
        let slot = current_slot(&sample(), at(5, 2, 0, 0)).unwrap();
        assert_eq!(slot, Slot { id: at(4, 23, 0, 0), brightness: 0 });
    }

    #[test]
    fn current_slot_ignores_input_order() {
        let entries = vec![e("23:00", 0), e("06:00", 70), e("18:00", 80)];
        assert_eq!(current_slot(&entries, at(5, 7, 0, 0)).unwrap().brightness, 70);
    }

    #[test]
    fn single_entry_applies_all_day() {
        let entries = vec![e("12:00", 40)];
        assert_eq!(current_slot(&entries, at(5, 13, 0, 0)).unwrap(), Slot { id: at(5, 12, 0, 0), brightness: 40 });
        assert_eq!(current_slot(&entries, at(5, 11, 0, 0)).unwrap(), Slot { id: at(4, 12, 0, 0), brightness: 40 });
    }

    #[test]
    fn empty_schedule_has_no_slot() {
        assert_eq!(current_slot(&[], at(5, 12, 0, 0)), None);
        assert_eq!(next_slot(&[], at(5, 12, 0, 0)), None);
    }

    #[test]
    fn next_slot_is_first_entry_after_now() {
        assert_eq!(next_slot(&sample(), at(5, 18, 0, 0)).unwrap(), Slot { id: at(5, 23, 0, 0), brightness: 0 });
    }

    #[test]
    fn next_slot_wraps_to_tomorrow() {
        assert_eq!(next_slot(&sample(), at(5, 23, 30, 0)).unwrap(), Slot { id: at(6, 6, 0, 0), brightness: 70 });
    }

    #[test]
    fn normalize_sorts_by_time() {
        let sorted = normalize(&[e("18:00", 80), e("06:00", 70)]).unwrap();
        assert_eq!(sorted, vec![e("06:00", 70), e("18:00", 80)]);
    }

    #[test]
    fn normalize_rejects_duplicate_time() {
        let errors = normalize(&[e("06:00", 70), e("06:00", 80)]).unwrap_err();
        assert_eq!(errors.len(), 1);
        assert_eq!(errors[0].index, 1);
    }

    #[test]
    fn normalize_rejects_bad_time_format() {
        for bad in ["6:00", "24:00", "06:60", "0600", "", "06:00:00"] {
            let errors = normalize(&[e(bad, 50)]).unwrap_err();
            assert_eq!(errors[0].index, 0, "{bad} should be rejected");
        }
    }

    #[test]
    fn normalize_rejects_brightness_over_100() {
        let errors = normalize(&[e("06:00", 101)]).unwrap_err();
        assert_eq!(errors[0].index, 0);
    }

    #[test]
    fn default_schedule_is_valid_and_matches_spec() {
        let d = default_schedule();
        assert_eq!(normalize(&d).unwrap(), d);
        assert_eq!(d.len(), 11);
        assert_eq!(d[0], e("06:00", 70));
        assert_eq!(d[10], e("23:00", 0));
    }

    #[test]
    fn format_errors_uses_one_based_rows() {
        let text = format_errors(&[EntryError { index: 1, message: "x".into() }]);
        assert_eq!(text, "2行目: x");
    }
}
```

- [ ] **Step 3: テストが失敗することを確認する**

```bash
cd src-tauri && cargo test schedule
```

期待する結果：`ScheduleEntry`、`current_slot` などが未定義というコンパイルエラーになる。

- [ ] **Step 4: 実装する**

`schedule.rs` のテストの上に書く。

```rust
//! スケジュール（時刻ごとの目標輝度）の計算と検証。I/O を持たない純粋関数のみ。

use chrono::{NaiveDateTime, NaiveTime};
use serde::{Deserialize, Serialize};
use std::collections::HashSet;

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ScheduleEntry {
    /// "HH:MM"（24時間表記、ゼロ埋め）
    pub time: String,
    pub brightness: u8,
}

/// スケジュールの1回分の発生。`id` はその回の発生日時で、日付が違えば別のスロットになる
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Slot {
    pub id: NaiveDateTime,
    pub brightness: u8,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct EntryError {
    pub index: usize,
    pub message: String,
}

fn parse_time(s: &str) -> Option<NaiveTime> {
    if s.len() != 5 {
        return None;
    }
    NaiveTime::parse_from_str(s, "%H:%M").ok()
}

fn sorted_times(entries: &[ScheduleEntry]) -> Vec<(NaiveTime, u8)> {
    let mut v: Vec<_> = entries
        .iter()
        .filter_map(|e| parse_time(&e.time).map(|t| (t, e.brightness)))
        .collect();
    v.sort_by_key(|(t, _)| *t);
    v
}

pub fn current_slot(entries: &[ScheduleEntry], now: NaiveDateTime) -> Option<Slot> {
    let times = sorted_times(entries);
    let today = now.date();
    if let Some(&(t, brightness)) = times.iter().rev().find(|(t, _)| *t <= now.time()) {
        return Some(Slot { id: today.and_time(t), brightness });
    }
    let &(t, brightness) = times.last()?;
    Some(Slot { id: today.pred_opt()?.and_time(t), brightness })
}

pub fn next_slot(entries: &[ScheduleEntry], now: NaiveDateTime) -> Option<Slot> {
    let times = sorted_times(entries);
    let today = now.date();
    if let Some(&(t, brightness)) = times.iter().find(|(t, _)| *t > now.time()) {
        return Some(Slot { id: today.and_time(t), brightness });
    }
    let &(t, brightness) = times.first()?;
    Some(Slot { id: today.succ_opt()?.and_time(t), brightness })
}

pub fn normalize(entries: &[ScheduleEntry]) -> Result<Vec<ScheduleEntry>, Vec<EntryError>> {
    let mut errors = Vec::new();
    let mut seen = HashSet::new();
    for (index, entry) in entries.iter().enumerate() {
        if parse_time(&entry.time).is_none() {
            errors.push(EntryError { index, message: format!("時刻の形式が正しくありません: {:?}", entry.time) });
            continue;
        }
        if entry.brightness > 100 {
            errors.push(EntryError { index, message: "輝度は 0〜100 で指定してください".into() });
            continue;
        }
        if !seen.insert(entry.time.clone()) {
            errors.push(EntryError { index, message: format!("時刻 {} が重複しています", entry.time) });
        }
    }
    if !errors.is_empty() {
        return Err(errors);
    }
    let mut sorted = entries.to_vec();
    sorted.sort_by(|a, b| a.time.cmp(&b.time));
    Ok(sorted)
}

pub fn format_errors(errors: &[EntryError]) -> String {
    errors
        .iter()
        .map(|e| format!("{}行目: {}", e.index + 1, e.message))
        .collect::<Vec<_>>()
        .join("\n")
}

pub fn default_schedule() -> Vec<ScheduleEntry> {
    [
        ("06:00", 70),
        ("07:00", 80),
        ("08:00", 90),
        ("09:00", 100),
        ("17:00", 90),
        ("18:00", 80),
        ("19:00", 60),
        ("20:00", 40),
        ("21:00", 30),
        ("22:00", 10),
        ("23:00", 0),
    ]
    .into_iter()
    .map(|(time, brightness)| ScheduleEntry { time: time.into(), brightness })
    .collect()
}
```

注意：`"6:00"` は長さが4なので `parse_time` の長さチェックで弾かれる。`"06:00:00"` は長さ8なので同じく弾かれる。

- [ ] **Step 5: テストが通ることを確認する**

```bash
cd src-tauri && cargo test schedule
```

期待する結果：14件すべて PASS。

- [ ] **Step 6: コミットする**

```bash
git add src-tauri/Cargo.toml src-tauri/Cargo.lock src-tauri/src/lib.rs src-tauri/src/schedule.rs
git commit -m "feat: add schedule slot calculation and validation"
```

---

### Task 3: 設定ファイル（`settings.rs`）

**Files:**
- Create: `src-tauri/src/settings.rs`
- Modify: `src-tauri/src/lib.rs`

**Interfaces:**
- Consumes: `schedule::{ScheduleEntry, normalize, default_schedule}`
- Produces:
  - `pub const CURRENT_VERSION: u32 = 1;`
  - `pub struct Settings { pub version: u32, pub autostart: bool, pub schedule: Vec<ScheduleEntry> }`（`Clone, Debug, PartialEq, Eq, Serialize, Deserialize, Default` の手動実装）
  - `pub enum LoadOutcome { Loaded, Created, RecoveredFromCorrupt }`
  - `pub fn load_or_init(path: &Path) -> io::Result<(Settings, LoadOutcome)>`
  - `pub fn save(path: &Path, settings: &Settings) -> io::Result<()>`
  - `pub fn backup_path(path: &Path) -> PathBuf`
  - `pub struct SettingsStore`：`open(path: PathBuf) -> io::Result<(SettingsStore, LoadOutcome)>`、`get(&self) -> Settings`、`update(&self, f: impl FnOnce(&mut Settings)) -> io::Result<Settings>`

- [ ] **Step 1: 失敗するテストを書く**

`src-tauri/src/lib.rs` に `pub mod settings;` を追加し、`src-tauri/src/settings.rs` にテストを書く。

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use crate::schedule::ScheduleEntry;
    use tempfile::tempdir;

    #[test]
    fn missing_file_creates_defaults() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("sub").join("settings.json");
        let (settings, outcome) = load_or_init(&path).unwrap();
        assert_eq!(outcome, LoadOutcome::Created);
        assert_eq!(settings, Settings::default());
        assert!(settings.autostart);
        assert!(path.exists());
    }

    #[test]
    fn save_then_load_round_trips() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("settings.json");
        let mut s = Settings::default();
        s.autostart = false;
        s.schedule = vec![ScheduleEntry { time: "12:00".into(), brightness: 40 }];
        save(&path, &s).unwrap();
        let (loaded, outcome) = load_or_init(&path).unwrap();
        assert_eq!(outcome, LoadOutcome::Loaded);
        assert_eq!(loaded, s);
        assert!(!dir.path().join("settings.json.tmp").exists());
    }

    #[test]
    fn unsorted_file_is_loaded_sorted() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("settings.json");
        std::fs::write(
            &path,
            r#"{"version":1,"autostart":true,"schedule":[{"time":"18:00","brightness":80},{"time":"06:00","brightness":70}]}"#,
        )
        .unwrap();
        let (loaded, _) = load_or_init(&path).unwrap();
        assert_eq!(loaded.schedule[0].time, "06:00");
    }

    fn assert_recovers(content: &str) {
        let dir = tempdir().unwrap();
        let path = dir.path().join("settings.json");
        std::fs::write(&path, content).unwrap();
        let (settings, outcome) = load_or_init(&path).unwrap();
        assert_eq!(outcome, LoadOutcome::RecoveredFromCorrupt);
        assert_eq!(settings, Settings::default());
        assert_eq!(std::fs::read_to_string(backup_path(&path)).unwrap(), content);
        let (reloaded, outcome) = load_or_init(&path).unwrap();
        assert_eq!(outcome, LoadOutcome::Loaded);
        assert_eq!(reloaded, Settings::default());
    }

    #[test]
    fn broken_json_is_backed_up_and_replaced() {
        assert_recovers("{ not json");
    }

    #[test]
    fn duplicate_times_are_treated_as_corrupt() {
        assert_recovers(
            r#"{"version":1,"autostart":true,"schedule":[{"time":"06:00","brightness":70},{"time":"06:00","brightness":80}]}"#,
        );
    }

    #[test]
    fn unknown_version_is_treated_as_corrupt() {
        assert_recovers(r#"{"version":2,"autostart":true,"schedule":[]}"#);
    }

    #[test]
    fn store_update_persists() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("settings.json");
        let (store, _) = SettingsStore::open(path.clone()).unwrap();
        let updated = store.update(|s| s.autostart = false).unwrap();
        assert!(!updated.autostart);
        assert!(!store.get().autostart);
        let (reloaded, _) = load_or_init(&path).unwrap();
        assert!(!reloaded.autostart);
    }
}
```

- [ ] **Step 2: テストが失敗することを確認する**

```bash
cd src-tauri && cargo test settings
```

期待する結果：`load_or_init` などが未定義というコンパイルエラーになる。

- [ ] **Step 3: 実装する**

```rust
//! 設定ファイル（settings.json）の読み書き

use crate::schedule::{default_schedule, normalize, ScheduleEntry};
use serde::{Deserialize, Serialize};
use std::ffi::OsString;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::sync::Mutex;

pub const CURRENT_VERSION: u32 = 1;

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Settings {
    pub version: u32,
    pub autostart: bool,
    pub schedule: Vec<ScheduleEntry>,
}

impl Default for Settings {
    fn default() -> Self {
        Self { version: CURRENT_VERSION, autostart: true, schedule: default_schedule() }
    }
}

#[derive(Debug, PartialEq, Eq)]
pub enum LoadOutcome {
    Loaded,
    Created,
    RecoveredFromCorrupt,
}

fn with_suffix(path: &Path, suffix: &str) -> PathBuf {
    let mut s: OsString = path.as_os_str().to_owned();
    s.push(suffix);
    PathBuf::from(s)
}

pub fn backup_path(path: &Path) -> PathBuf {
    with_suffix(path, ".bak")
}

fn parse(text: &str) -> Option<Settings> {
    let settings: Settings = serde_json::from_str(text).ok()?;
    if settings.version != CURRENT_VERSION {
        return None;
    }
    let schedule = normalize(&settings.schedule).ok()?;
    Some(Settings { schedule, ..settings })
}

pub fn load_or_init(path: &Path) -> io::Result<(Settings, LoadOutcome)> {
    match fs::read_to_string(path) {
        Ok(text) => match parse(&text) {
            Some(settings) => Ok((settings, LoadOutcome::Loaded)),
            None => {
                fs::rename(path, backup_path(path))?;
                let settings = Settings::default();
                save(path, &settings)?;
                Ok((settings, LoadOutcome::RecoveredFromCorrupt))
            }
        },
        Err(e) if e.kind() == io::ErrorKind::NotFound => {
            let settings = Settings::default();
            save(path, &settings)?;
            Ok((settings, LoadOutcome::Created))
        }
        Err(e) => Err(e),
    }
}

/// 一時ファイルに書いてから置き換える。書き込み中に落ちても元のファイルは壊れない
pub fn save(path: &Path, settings: &Settings) -> io::Result<()> {
    if let Some(dir) = path.parent() {
        fs::create_dir_all(dir)?;
    }
    let tmp = with_suffix(path, ".tmp");
    let json = serde_json::to_string_pretty(settings).map_err(io::Error::other)?;
    fs::write(&tmp, json)?;
    fs::rename(&tmp, path)
}

pub struct SettingsStore {
    path: PathBuf,
    current: Mutex<Settings>,
}

impl SettingsStore {
    pub fn open(path: PathBuf) -> io::Result<(Self, LoadOutcome)> {
        let (settings, outcome) = load_or_init(&path)?;
        Ok((Self { path, current: Mutex::new(settings) }, outcome))
    }

    pub fn get(&self) -> Settings {
        self.current.lock().unwrap().clone()
    }

    /// 変更を適用して保存する。保存に失敗した場合はメモリ上の値も変えない
    pub fn update(&self, f: impl FnOnce(&mut Settings)) -> io::Result<Settings> {
        let mut current = self.current.lock().unwrap();
        let mut next = current.clone();
        f(&mut next);
        save(&self.path, &next)?;
        *current = next.clone();
        Ok(next)
    }
}
```

- [ ] **Step 4: テストが通ることを確認する**

```bash
cd src-tauri && cargo test settings
```

期待する結果：7件すべて PASS。

- [ ] **Step 5: コミットする**

```bash
git add src-tauri/src/lib.rs src-tauri/src/settings.rs
git commit -m "feat: add settings file persistence with corruption recovery"
```

---

### Task 4: モニター操作の共通処理とテスト用のモック（`monitor/mod.rs`、`monitor/mock.rs`）

**Files:**
- Create: `src-tauri/src/monitor/mod.rs`、`src-tauri/src/monitor/mock.rs`
- Modify: `src-tauri/src/lib.rs`

**Interfaces:**
- Produces:
  - `pub struct MonitorIdent { pub id: String, pub name: String }`（`Clone, Debug, PartialEq, Eq`）
  - `pub struct MonitorInfo { pub id: String, pub name: String, pub brightness: Option<u8> }`（`Serialize` 付き）
  - `pub struct ApplyFailure { pub id: String, pub name: String, pub error: String }`（`Serialize` 付き）
  - `pub struct ApplyResult { pub attempted: usize, pub failed: Vec<ApplyFailure> }`（`Serialize, Default` 付き）、`fn any_succeeded(&self) -> bool`
  - `pub trait MonitorBackend: Send + Sync { fn list(&self) -> Vec<MonitorIdent>; fn get_brightness(&self, id: &str) -> Result<u8, String>; fn set_brightness(&self, id: &str, value: u8) -> Result<(), String>; }`
  - `pub fn read_all(backend: &dyn MonitorBackend) -> Vec<MonitorInfo>`
  - `pub fn apply_all(backend: &dyn MonitorBackend, value: u8) -> ApplyResult`
  - `pub fn apply_one(backend: &dyn MonitorBackend, id: &str, value: u8) -> ApplyResult`
  - `pub fn apply_with_retry(backend: &dyn MonitorBackend, value: u8, delays: &[Duration], sleep: &dyn Fn(Duration)) -> ApplyResult`
  - `pub const RETRY_DELAYS: [Duration; 2]`（5秒、15秒）
  - テスト用（`#[cfg(test)] pub mod mock`）：`MockBackend::new(&[(&str, &str, Option<u8>)])`、`fail_sets(&self, id: &str, times: usize)`、`set_calls(&self) -> Vec<(String, u8)>`、`replace_monitors(&self, &[(&str, &str, Option<u8>)])`。輝度が `None` のモニターは get も set も常に失敗する（DDC/CI 非対応のモニターを表す）

- [ ] **Step 1: モックを書く**

`src-tauri/src/lib.rs` に `pub mod monitor;` を追加する。`src-tauri/src/monitor/mock.rs`：

```rust
//! テスト用の MonitorBackend

use super::{MonitorBackend, MonitorIdent};
use std::collections::HashMap;
use std::sync::Mutex;

pub struct MockBackend {
    monitors: Mutex<Vec<(MonitorIdent, Option<u8>)>>,
    remaining_failures: Mutex<HashMap<String, usize>>,
    set_calls: Mutex<Vec<(String, u8)>>,
}

fn build(specs: &[(&str, &str, Option<u8>)]) -> Vec<(MonitorIdent, Option<u8>)> {
    specs
        .iter()
        .map(|(id, name, b)| (MonitorIdent { id: id.to_string(), name: name.to_string() }, *b))
        .collect()
}

impl MockBackend {
    /// (id, name, brightness)。brightness が None のモニターは DDC/CI 非対応として常に失敗する
    pub fn new(specs: &[(&str, &str, Option<u8>)]) -> Self {
        Self {
            monitors: Mutex::new(build(specs)),
            remaining_failures: Mutex::new(HashMap::new()),
            set_calls: Mutex::new(Vec::new()),
        }
    }

    /// 次の `times` 回の set_brightness を失敗させる
    pub fn fail_sets(&self, id: &str, times: usize) {
        self.remaining_failures.lock().unwrap().insert(id.to_string(), times);
    }

    pub fn set_calls(&self) -> Vec<(String, u8)> {
        self.set_calls.lock().unwrap().clone()
    }

    pub fn replace_monitors(&self, specs: &[(&str, &str, Option<u8>)]) {
        *self.monitors.lock().unwrap() = build(specs);
    }
}

impl MonitorBackend for MockBackend {
    fn list(&self) -> Vec<MonitorIdent> {
        self.monitors.lock().unwrap().iter().map(|(m, _)| m.clone()).collect()
    }

    fn get_brightness(&self, id: &str) -> Result<u8, String> {
        let monitors = self.monitors.lock().unwrap();
        match monitors.iter().find(|(m, _)| m.id == id) {
            Some((_, Some(b))) => Ok(*b),
            Some((_, None)) => Err("DDC/CI not supported".into()),
            None => Err("not found".into()),
        }
    }

    fn set_brightness(&self, id: &str, value: u8) -> Result<(), String> {
        self.set_calls.lock().unwrap().push((id.to_string(), value));
        if let Some(n) = self.remaining_failures.lock().unwrap().get_mut(id) {
            if *n > 0 {
                *n -= 1;
                return Err("transient failure".into());
            }
        }
        let mut monitors = self.monitors.lock().unwrap();
        match monitors.iter_mut().find(|(m, _)| m.id == id) {
            Some((_, b @ Some(_))) => {
                *b = Some(value);
                Ok(())
            }
            Some((_, None)) => Err("DDC/CI not supported".into()),
            None => Err("not found".into()),
        }
    }
}
```

- [ ] **Step 2: 失敗するテストを書く**

`src-tauri/src/monitor/mod.rs` の末尾にテストを書く（このファイルの冒頭には `#[cfg(test)] pub mod mock;` を置く）。

```rust
#[cfg(test)]
mod tests {
    use super::mock::MockBackend;
    use super::*;
    use std::cell::RefCell;

    #[test]
    fn read_all_reports_unreadable_monitor_as_none() {
        let b = MockBackend::new(&[("a", "Ext", Some(60)), ("lap", "Laptop", None)]);
        let all = read_all(&b);
        assert_eq!(all[0], MonitorInfo { id: "a".into(), name: "Ext".into(), brightness: Some(60) });
        assert_eq!(all[1].brightness, None);
    }

    #[test]
    fn apply_all_continues_after_a_failure() {
        let b = MockBackend::new(&[("lap", "Laptop", None), ("a", "Ext", Some(60))]);
        let r = apply_all(&b, 30);
        assert_eq!(r.attempted, 2);
        assert_eq!(r.failed.len(), 1);
        assert_eq!(r.failed[0].id, "lap");
        assert_eq!(r.failed[0].name, "Laptop");
        assert!(r.any_succeeded());
        assert_eq!(b.get_brightness("a"), Ok(30));
    }

    #[test]
    fn apply_one_only_touches_target() {
        let b = MockBackend::new(&[("a", "A", Some(60)), ("b", "B", Some(60))]);
        let r = apply_one(&b, "b", 20);
        assert!(r.failed.is_empty());
        assert_eq!(b.set_calls(), vec![("b".to_string(), 20)]);
    }

    #[test]
    fn apply_one_unknown_id_fails() {
        let b = MockBackend::new(&[("a", "A", Some(60))]);
        let r = apply_one(&b, "zzz", 20);
        assert_eq!(r.attempted, 0);
        assert_eq!(r.failed.len(), 1);
        assert!(b.set_calls().is_empty());
    }

    #[test]
    fn retry_only_retries_failed_monitors_and_sleeps_given_delays() {
        let b = MockBackend::new(&[("a", "A", Some(60)), ("b", "B", Some(60))]);
        b.fail_sets("b", 1);
        let slept = RefCell::new(Vec::new());
        let r = apply_with_retry(&b, 40, &RETRY_DELAYS, &|d| slept.borrow_mut().push(d));
        assert!(r.failed.is_empty());
        assert_eq!(r.attempted, 2);
        assert_eq!(slept.into_inner(), vec![Duration::from_secs(5)]);
        assert_eq!(
            b.set_calls(),
            vec![("a".to_string(), 40), ("b".to_string(), 40), ("b".to_string(), 40)]
        );
    }

    #[test]
    fn retry_gives_up_after_all_delays() {
        let b = MockBackend::new(&[("lap", "Laptop", None)]);
        let slept = RefCell::new(Vec::new());
        let r = apply_with_retry(&b, 40, &RETRY_DELAYS, &|d| slept.borrow_mut().push(d));
        assert_eq!(r.failed.len(), 1);
        assert!(!r.any_succeeded());
        assert_eq!(slept.into_inner(), RETRY_DELAYS.to_vec());
        assert_eq!(b.set_calls().len(), 3);
    }

    #[test]
    fn no_monitors_is_not_a_success_but_not_a_failure() {
        let b = MockBackend::new(&[]);
        let r = apply_with_retry(&b, 40, &RETRY_DELAYS, &|_| panic!("must not sleep"));
        assert_eq!(r, ApplyResult::default());
        assert!(!r.any_succeeded());
    }
}
```

- [ ] **Step 3: テストが失敗することを確認する**

```bash
cd src-tauri && cargo test monitor
```

期待する結果：`MonitorBackend` などが未定義というコンパイルエラーになる。

- [ ] **Step 4: 実装する**

`src-tauri/src/monitor/mod.rs` のテストの上に書く。

```rust
//! モニターの列挙と輝度の読み書き。OS 依存の部分は MonitorBackend の実装に閉じ込める

#[cfg(test)]
pub mod mock;
pub mod win32;

use serde::Serialize;
use std::time::Duration;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MonitorIdent {
    pub id: String,
    pub name: String,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct MonitorInfo {
    pub id: String,
    pub name: String,
    /// 読み取れなければ None（DDC/CI 非対応のモニターなど）
    pub brightness: Option<u8>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct ApplyFailure {
    pub id: String,
    pub name: String,
    pub error: String,
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize)]
pub struct ApplyResult {
    pub attempted: usize,
    pub failed: Vec<ApplyFailure>,
}

impl ApplyResult {
    pub fn any_succeeded(&self) -> bool {
        self.failed.len() < self.attempted
    }
}

pub trait MonitorBackend: Send + Sync {
    /// 接続中のモニターを列挙する。DDC 通信は行わない
    fn list(&self) -> Vec<MonitorIdent>;
    fn get_brightness(&self, id: &str) -> Result<u8, String>;
    fn set_brightness(&self, id: &str, value: u8) -> Result<(), String>;
}

pub const RETRY_DELAYS: [Duration; 2] = [Duration::from_secs(5), Duration::from_secs(15)];

pub fn read_all(backend: &dyn MonitorBackend) -> Vec<MonitorInfo> {
    backend
        .list()
        .into_iter()
        .map(|m| {
            let brightness = backend.get_brightness(&m.id).ok();
            MonitorInfo { id: m.id, name: m.name, brightness }
        })
        .collect()
}

fn apply_to(backend: &dyn MonitorBackend, targets: &[MonitorIdent], value: u8) -> ApplyResult {
    let failed = targets
        .iter()
        .filter_map(|m| {
            backend.set_brightness(&m.id, value).err().map(|error| ApplyFailure {
                id: m.id.clone(),
                name: m.name.clone(),
                error,
            })
        })
        .collect();
    ApplyResult { attempted: targets.len(), failed }
}

pub fn apply_all(backend: &dyn MonitorBackend, value: u8) -> ApplyResult {
    apply_to(backend, &backend.list(), value)
}

pub fn apply_one(backend: &dyn MonitorBackend, id: &str, value: u8) -> ApplyResult {
    let targets: Vec<_> = backend.list().into_iter().filter(|m| m.id == id).collect();
    if targets.is_empty() {
        return ApplyResult {
            attempted: 0,
            failed: vec![ApplyFailure { id: id.into(), name: id.into(), error: "モニターが見つかりません".into() }],
        };
    }
    apply_to(backend, &targets, value)
}

/// 全モニターに適用し、失敗したモニターだけを delays の間隔でリトライする
pub fn apply_with_retry(
    backend: &dyn MonitorBackend,
    value: u8,
    delays: &[Duration],
    sleep: &dyn Fn(Duration),
) -> ApplyResult {
    let all = backend.list();
    let mut result = apply_to(backend, &all, value);
    for delay in delays {
        if result.failed.is_empty() {
            break;
        }
        sleep(*delay);
        let retry: Vec<_> = all
            .iter()
            .filter(|m| result.failed.iter().any(|f| f.id == m.id))
            .cloned()
            .collect();
        result = ApplyResult { attempted: all.len(), failed: apply_to(backend, &retry, value).failed };
    }
    result
}
```

`pub mod win32;` は Task 5 で作るので、この時点では空の `src-tauri/src/monitor/win32.rs` を作っておく（中身は `//! Win32 実装（Task 5）` の1行だけ）。

- [ ] **Step 5: テストが通ることを確認する**

```bash
cd src-tauri && cargo test monitor
```

期待する結果：7件すべて PASS。

- [ ] **Step 6: コミットする**

```bash
git add src-tauri/src/lib.rs src-tauri/src/monitor
git commit -m "feat: add monitor backend trait with retry and mock"
```

---

### Task 5: Win32 によるモニター制御（`monitor/win32.rs`）

**Files:**
- Modify: `src-tauri/src/monitor/win32.rs`、`src-tauri/Cargo.toml`
- Create: `src-tauri/examples/list_monitors.rs`

**Interfaces:**
- Consumes: `monitor::{MonitorBackend, MonitorIdent, read_all}`
- Produces:
  - `pub struct WinBackend`、`WinBackend::new() -> Self`（`MonitorBackend` を実装する。内部の `Mutex<()>` で DDC へのアクセスを直列化する）
  - `pub(crate) struct RawMonitor { pub device_path: Option<String>, pub description: String }`
  - `pub(crate) fn build_idents(raw: &[RawMonitor], names: &HashMap<String, String>) -> Vec<MonitorIdent>`（`names` のキーは小文字のデバイスパス）

- [ ] **Step 1: 依存を追加する**

`src-tauri/Cargo.toml` の `[dependencies]` に追加する。

```toml
windows = { version = "0.62", features = [
  "Win32_Foundation",
  "Win32_Graphics_Gdi",
  "Win32_Devices_Display",
] }
```

- [ ] **Step 2: `build_idents` の失敗するテストを書く**

`src-tauri/src/monitor/win32.rs` の中身を置き換え、まずテストを書く。

```rust
#[cfg(test)]
mod tests {
    use super::*;

    fn raw(path: Option<&str>, desc: &str) -> RawMonitor {
        RawMonitor { device_path: path.map(String::from), description: desc.into() }
    }

    #[test]
    fn uses_device_path_as_id_and_friendly_name_case_insensitively() {
        let names = HashMap::from([(r"\\?\display#dela0f3#1".to_string(), "DELL U2720Q".to_string())]);
        let ids = build_idents(&[raw(Some(r"\\?\DISPLAY#DELA0F3#1"), "Generic PnP Monitor")], &names);
        assert_eq!(ids, vec![MonitorIdent { id: r"\\?\DISPLAY#DELA0F3#1".into(), name: "DELL U2720Q".into() }]);
    }

    #[test]
    fn falls_back_to_numbered_name() {
        let ids = build_idents(&[raw(Some("p1"), "Generic"), raw(Some("p2"), "Generic")], &HashMap::new());
        assert_eq!(ids[0].name, "Monitor 1");
        assert_eq!(ids[1].name, "Monitor 2");
    }

    #[test]
    fn same_model_monitors_get_distinct_ids_and_numbered_names() {
        let names = HashMap::from([
            ("p1".to_string(), "DELL U2720Q".to_string()),
            ("p2".to_string(), "DELL U2720Q".to_string()),
        ]);
        let ids = build_idents(&[raw(Some("p1"), "G"), raw(Some("p2"), "G")], &names);
        assert_ne!(ids[0].id, ids[1].id);
        assert_eq!(ids[0].name, "DELL U2720Q (1)");
        assert_eq!(ids[1].name, "DELL U2720Q (2)");
    }

    #[test]
    fn missing_device_paths_still_produce_unique_ids() {
        let ids = build_idents(&[raw(None, "Generic"), raw(None, "Generic")], &HashMap::new());
        assert_ne!(ids[0].id, ids[1].id);
    }

    #[test]
    fn duplicate_device_paths_are_disambiguated() {
        let ids = build_idents(&[raw(Some("p"), "G"), raw(Some("p"), "G")], &HashMap::new());
        assert_eq!(ids[0].id, "p");
        assert_eq!(ids[1].id, "p#2");
    }
}
```

- [ ] **Step 3: テストが失敗することを確認する**

```bash
cd src-tauri && cargo test monitor::win32
```

期待する結果：`RawMonitor`、`build_idents` が未定義というコンパイルエラーになる。

- [ ] **Step 4: `build_idents` を実装する**

```rust
//! Win32 の Monitor Configuration API（DDC/CI）と DisplayConfig API（モニター名）による MonitorBackend 実装

use super::{MonitorBackend, MonitorIdent};
use std::collections::HashMap;

pub(crate) struct RawMonitor {
    pub device_path: Option<String>,
    pub description: String,
}

/// 列挙結果から一意な ID と表示名を作る。names のキーは小文字のデバイスパス
pub(crate) fn build_idents(raw: &[RawMonitor], names: &HashMap<String, String>) -> Vec<MonitorIdent> {
    let mut id_counts: HashMap<String, usize> = HashMap::new();
    let mut idents: Vec<MonitorIdent> = raw
        .iter()
        .enumerate()
        .map(|(i, r)| {
            let base = r.device_path.clone().unwrap_or_else(|| format!("{}#{}", r.description, i + 1));
            let count = id_counts.entry(base.clone()).or_insert(0);
            *count += 1;
            let id = if *count == 1 { base } else { format!("{base}#{count}") };
            let name = r
                .device_path
                .as_ref()
                .and_then(|p| names.get(&p.to_lowercase()))
                .cloned()
                .unwrap_or_else(|| format!("Monitor {}", i + 1));
            MonitorIdent { id, name }
        })
        .collect();

    let mut name_totals: HashMap<String, usize> = HashMap::new();
    for m in &idents {
        *name_totals.entry(m.name.clone()).or_insert(0) += 1;
    }
    let mut name_seen: HashMap<String, usize> = HashMap::new();
    for m in &mut idents {
        if name_totals[&m.name] > 1 {
            let n = name_seen.entry(m.name.clone()).or_insert(0);
            *n += 1;
            m.name = format!("{} ({})", m.name, n);
        }
    }
    idents
}
```

- [ ] **Step 5: テストが通ることを確認する**

```bash
cd src-tauri && cargo test monitor::win32
```

期待する結果：5件すべて PASS。

- [ ] **Step 6: Win32 の呼び出しを実装する**

`build_idents` の下（テストの上）に追加する。`use` は `win32.rs` の先頭に移す。

```rust
use std::sync::Mutex;
use windows::core::{BOOL, PCWSTR};
use windows::Win32::Devices::Display::{
    DestroyPhysicalMonitor, DisplayConfigGetDeviceInfo, GetDisplayConfigBufferSizes,
    GetNumberOfPhysicalMonitorsFromHMONITOR, GetPhysicalMonitorsFromHMONITOR,
    GetVCPFeatureAndVCPFeatureReply, QueryDisplayConfig, SetVCPFeature,
    DISPLAYCONFIG_DEVICE_INFO_GET_TARGET_NAME, DISPLAYCONFIG_DEVICE_INFO_HEADER,
    DISPLAYCONFIG_MODE_INFO, DISPLAYCONFIG_PATH_INFO, DISPLAYCONFIG_TARGET_DEVICE_NAME,
    PHYSICAL_MONITOR, QDC_ONLY_ACTIVE_PATHS,
};
use windows::Win32::Foundation::{ERROR_SUCCESS, HANDLE, LPARAM, RECT};
use windows::Win32::Graphics::Gdi::{
    EnumDisplayDevicesW, EnumDisplayMonitors, GetMonitorInfoW, DISPLAY_DEVICEW, HDC, HMONITOR,
    MONITORINFO, MONITORINFOEXW,
};

const VCP_BRIGHTNESS: u8 = 0x10;
const EDD_GET_DEVICE_INTERFACE_NAME: u32 = 0x1;
const DISPLAY_DEVICE_ACTIVE: u32 = 0x1;

fn from_wide(buf: &[u16]) -> String {
    let len = buf.iter().position(|&c| c == 0).unwrap_or(buf.len());
    String::from_utf16_lossy(&buf[..len])
}

fn hmonitors() -> Vec<HMONITOR> {
    unsafe extern "system" fn collect(h: HMONITOR, _: HDC, _: *mut RECT, data: LPARAM) -> BOOL {
        let list = unsafe { &mut *(data.0 as *mut Vec<HMONITOR>) };
        list.push(h);
        true.into()
    }
    let mut list: Vec<HMONITOR> = Vec::new();
    unsafe {
        let _ = EnumDisplayMonitors(None, None, Some(collect), LPARAM(&mut list as *mut _ as isize));
    }
    list
}

/// HMONITOR に属するアクティブなモニターのデバイスインターフェースパス
fn device_paths(hmon: HMONITOR) -> Vec<String> {
    let mut info = MONITORINFOEXW::default();
    info.monitorInfo.cbSize = std::mem::size_of::<MONITORINFOEXW>() as u32;
    if !unsafe { GetMonitorInfoW(hmon, &mut info as *mut _ as *mut MONITORINFO) }.as_bool() {
        return Vec::new();
    }
    let mut paths = Vec::new();
    for i in 0.. {
        let mut dd = DISPLAY_DEVICEW { cb: std::mem::size_of::<DISPLAY_DEVICEW>() as u32, ..Default::default() };
        let found = unsafe {
            EnumDisplayDevicesW(PCWSTR(info.szDevice.as_ptr()), i, &mut dd, EDD_GET_DEVICE_INTERFACE_NAME)
        };
        if !found.as_bool() {
            break;
        }
        if dd.StateFlags.0 & DISPLAY_DEVICE_ACTIVE != 0 {
            paths.push(from_wide(&dd.DeviceID));
        }
    }
    paths
}

fn physical_monitors(hmon: HMONITOR) -> Vec<PHYSICAL_MONITOR> {
    let mut n = 0u32;
    if unsafe { GetNumberOfPhysicalMonitorsFromHMONITOR(hmon, &mut n) }.is_err() || n == 0 {
        return Vec::new();
    }
    let mut monitors = vec![PHYSICAL_MONITOR::default(); n as usize];
    if unsafe { GetPhysicalMonitorsFromHMONITOR(hmon, &mut monitors) }.is_err() {
        return Vec::new();
    }
    monitors
}

/// 小文字のデバイスパス → EDID 由来の製品名
fn friendly_names() -> HashMap<String, String> {
    let mut names = HashMap::new();
    let (mut n_paths, mut n_modes) = (0u32, 0u32);
    unsafe {
        if GetDisplayConfigBufferSizes(QDC_ONLY_ACTIVE_PATHS, &mut n_paths, &mut n_modes) != ERROR_SUCCESS {
            return names;
        }
        let mut paths = vec![DISPLAYCONFIG_PATH_INFO::default(); n_paths as usize];
        let mut modes = vec![DISPLAYCONFIG_MODE_INFO::default(); n_modes as usize];
        if QueryDisplayConfig(
            QDC_ONLY_ACTIVE_PATHS,
            &mut n_paths,
            paths.as_mut_ptr(),
            &mut n_modes,
            modes.as_mut_ptr(),
            None,
        ) != ERROR_SUCCESS
        {
            return names;
        }
        for path in &paths[..n_paths as usize] {
            let mut target = DISPLAYCONFIG_TARGET_DEVICE_NAME::default();
            target.header = DISPLAYCONFIG_DEVICE_INFO_HEADER {
                r#type: DISPLAYCONFIG_DEVICE_INFO_GET_TARGET_NAME,
                size: std::mem::size_of::<DISPLAYCONFIG_TARGET_DEVICE_NAME>() as u32,
                adapterId: path.targetInfo.adapterId,
                id: path.targetInfo.id,
            };
            if DisplayConfigGetDeviceInfo(&mut target.header) != 0 {
                continue;
            }
            let name = from_wide(&target.monitorFriendlyDeviceName);
            if !name.is_empty() {
                names.insert(from_wide(&target.monitorDevicePath).to_lowercase(), name);
            }
        }
    }
    names
}

/// 列挙した物理モニターのハンドル。drop で解放する
struct Snapshot(Vec<(MonitorIdent, HANDLE)>);

impl Snapshot {
    fn take() -> Self {
        let mut raw = Vec::new();
        let mut handles = Vec::new();
        for hmon in hmonitors() {
            let paths = device_paths(hmon);
            for (i, pm) in physical_monitors(hmon).into_iter().enumerate() {
                raw.push(RawMonitor {
                    device_path: paths.get(i).cloned(),
                    description: from_wide(&pm.szPhysicalMonitorDescription),
                });
                handles.push(pm.hPhysicalMonitor);
            }
        }
        let idents = build_idents(&raw, &friendly_names());
        Snapshot(idents.into_iter().zip(handles).collect())
    }

    fn handle(&self, id: &str) -> Result<HANDLE, String> {
        self.0
            .iter()
            .find(|(m, _)| m.id == id)
            .map(|(_, h)| *h)
            .ok_or_else(|| "モニターが見つかりません".to_string())
    }
}

impl Drop for Snapshot {
    fn drop(&mut self) {
        for (_, handle) in &self.0 {
            unsafe {
                let _ = DestroyPhysicalMonitor(*handle);
            }
        }
    }
}

pub struct WinBackend {
    /// スケジューラーと手動操作が同時に DDC と通信しないようにする
    lock: Mutex<()>,
}

impl WinBackend {
    pub fn new() -> Self {
        Self { lock: Mutex::new(()) }
    }
}

impl Default for WinBackend {
    fn default() -> Self {
        Self::new()
    }
}

impl MonitorBackend for WinBackend {
    fn list(&self) -> Vec<MonitorIdent> {
        let _guard = self.lock.lock().unwrap();
        Snapshot::take().0.iter().map(|(m, _)| m.clone()).collect()
    }

    fn get_brightness(&self, id: &str) -> Result<u8, String> {
        let _guard = self.lock.lock().unwrap();
        let snapshot = Snapshot::take();
        let handle = snapshot.handle(id)?;
        let mut current = 0u32;
        let ok = unsafe { GetVCPFeatureAndVCPFeatureReply(handle, VCP_BRIGHTNESS, None, &mut current, None) };
        if ok == 0 {
            return Err(format!("輝度を読み取れません: {}", std::io::Error::last_os_error()));
        }
        Ok(current.min(100) as u8)
    }

    fn set_brightness(&self, id: &str, value: u8) -> Result<(), String> {
        let _guard = self.lock.lock().unwrap();
        let snapshot = Snapshot::take();
        let handle = snapshot.handle(id)?;
        let ok = unsafe { SetVCPFeature(handle, VCP_BRIGHTNESS, value.min(100) as u32) };
        if ok == 0 {
            return Err(format!("輝度を変更できません: {}", std::io::Error::last_os_error()));
        }
        Ok(())
    }
}
```

コンパイルエラーへの対処：ここに書いたシグネチャは `windows` 0.62.2 のドキュメントで確認している。それでも型名が見つからない場合は、https://microsoft.github.io/windows-docs-rs/ でその名前を検索し、モジュールのパスを直す。よくあるケースとして、`BOOL` が `windows::core` にない場合は `windows::Win32::Foundation::BOOL` を、`dd.StateFlags.0` が型エラーになる場合は `dd.StateFlags` を（`u32` として）使う。処理の中身は変えないこと。

- [ ] **Step 7: 実機確認用の example を作る**

`src-tauri/src/lib.rs` で `pub mod monitor;` が `pub` になっていることを確認する。`src-tauri/examples/list_monitors.rs`：

```rust
//! 実機でモニターの ID・名前・輝度を確認する: cargo run --example list_monitors
use dimmer_lib::monitor::{read_all, win32::WinBackend};

fn main() {
    for m in read_all(&WinBackend::new()) {
        println!("{:<24} {:>5}  {}", m.name, format!("{:?}", m.brightness), m.id);
    }
}
```

- [ ] **Step 8: 実機で確認する**

```bash
cd src-tauri && cargo run --example list_monitors
```

期待する結果：接続中のモニターが1行ずつ表示される。外部モニターには製品名（例：`DELL U2720Q`）と `Some(数値)` が表示され、ID は `\\?\DISPLAY#...` の形になる。ノート PC の内蔵ディスプレイは `None` になってよい。結果をユーザーに見せ、名前が「Monitor N」になってしまう場合はその旨を報告する（その場合も先に進んでよい）。

- [ ] **Step 9: すべてのテストを流してコミットする**

```bash
cd src-tauri && cargo test && cd ..
git add src-tauri/Cargo.toml src-tauri/Cargo.lock src-tauri/src/monitor/win32.rs src-tauri/examples/list_monitors.rs
git commit -m "feat: control monitor brightness via Win32 DDC/CI API"
```

---

### Task 6: スケジューラー（`scheduler.rs`）

**Files:**
- Create: `src-tauri/src/scheduler.rs`
- Modify: `src-tauri/src/lib.rs`

**Interfaces:**
- Consumes: `schedule::{current_slot, ScheduleEntry, Slot}`、`monitor::{apply_with_retry, ApplyResult, MonitorBackend, RETRY_DELAYS}`
- Produces:
  - `pub const TICK_INTERVAL: Duration`（30秒）、`pub const RESUME_GAP_SECS: i64 = 90;`、`pub const MAX_FAILED_TICKS: u32 = 10;`
  - `pub struct SchedulerState`（`Default`）
  - `pub struct Decision { pub apply: Option<Slot>, pub resumed: bool, pub monitors_changed: bool, pub first: bool }`
  - `SchedulerState::tick(&mut self, now: NaiveDateTime, monitor_ids: Vec<String>, entries: &[ScheduleEntry]) -> Decision`
  - `SchedulerState::record_result(&mut self, slot: Slot, result: &ApplyResult) -> bool`（適用済みにしたら true）
  - `SchedulerState::mark_current_applied(&mut self, entries: &[ScheduleEntry], now: NaiveDateTime)`
  - `pub fn run_once(backend: &dyn MonitorBackend, scheduler: &Mutex<SchedulerState>, entries: &[ScheduleEntry], now: NaiveDateTime, sleep: &dyn Fn(Duration)) -> bool`（`monitors-updated` を送るべきなら true）
  - スレッドの起動（`spawn`）は AppState が必要なので Task 7 で追加する

- [ ] **Step 1: 失敗するテストを書く**

`src-tauri/src/lib.rs` に `pub mod scheduler;` を追加し、`src-tauri/src/scheduler.rs` にテストを書く。

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use crate::monitor::mock::MockBackend;
    use crate::monitor::{ApplyFailure, ApplyResult};
    use crate::schedule::ScheduleEntry;
    use chrono::{Duration as ChronoDuration, NaiveDate};

    fn entries() -> Vec<ScheduleEntry> {
        vec![
            ScheduleEntry { time: "06:00".into(), brightness: 70 },
            ScheduleEntry { time: "18:00".into(), brightness: 80 },
        ]
    }

    fn at(h: u32, m: u32, s: u32) -> NaiveDateTime {
        NaiveDate::from_ymd_opt(2026, 10, 5).unwrap().and_hms_opt(h, m, s).unwrap()
    }

    fn ids() -> Vec<String> {
        vec!["a".into(), "b".into()]
    }

    fn ok(n: usize) -> ApplyResult {
        ApplyResult { attempted: n, failed: vec![] }
    }

    fn all_failed() -> ApplyResult {
        ApplyResult {
            attempted: 1,
            failed: vec![ApplyFailure { id: "a".into(), name: "A".into(), error: "x".into() }],
        }
    }

    /// 1回目の tick で適用し、成功を記録した状態を作る
    fn applied_at(now: NaiveDateTime) -> SchedulerState {
        let mut s = SchedulerState::default();
        let d = s.tick(now, ids(), &entries());
        s.record_result(d.apply.unwrap(), &ok(2));
        s
    }

    #[test]
    fn first_tick_applies_current_slot() {
        let mut s = SchedulerState::default();
        let d = s.tick(at(19, 0, 0), ids(), &entries());
        assert!(d.first);
        assert_eq!(d.apply.unwrap().brightness, 80);
    }

    #[test]
    fn same_slot_is_not_reapplied_so_manual_changes_survive() {
        let mut s = applied_at(at(19, 0, 0));
        let d = s.tick(at(19, 0, 30), ids(), &entries());
        assert_eq!(d.apply, None);
    }

    #[test]
    fn new_slot_is_applied() {
        let mut s = applied_at(at(17, 59, 40));
        let d = s.tick(at(18, 0, 10), ids(), &entries());
        assert_eq!(d.apply.unwrap().brightness, 80);
        assert!(!d.resumed);
    }

    #[test]
    fn long_gap_is_treated_as_resume_and_reapplies_same_slot() {
        let mut s = applied_at(at(19, 0, 0));
        let d = s.tick(at(19, 0, 0) + ChronoDuration::seconds(RESUME_GAP_SECS), ids(), &entries());
        assert!(d.resumed);
        assert_eq!(d.apply.unwrap().brightness, 80);
    }

    #[test]
    fn clock_going_backwards_reapplies() {
        let mut s = applied_at(at(19, 0, 0));
        let d = s.tick(at(18, 30, 0), ids(), &entries());
        assert!(d.resumed);
        assert_eq!(d.apply.unwrap().brightness, 80);
    }

    #[test]
    fn monitor_change_reapplies() {
        let mut s = applied_at(at(19, 0, 0));
        let d = s.tick(at(19, 0, 30), vec!["a".into()], &entries());
        assert!(d.monitors_changed);
        assert!(d.apply.is_some());
    }

    #[test]
    fn monitor_order_does_not_count_as_change() {
        let mut s = applied_at(at(19, 0, 0));
        let d = s.tick(at(19, 0, 30), vec!["b".into(), "a".into()], &entries());
        assert!(!d.monitors_changed);
        assert_eq!(d.apply, None);
    }

    #[test]
    fn all_failed_is_retried_next_tick_even_after_resume() {
        let mut s = applied_at(at(19, 0, 0));
        let d = s.tick(at(21, 0, 0), ids(), &entries());
        assert!(!s.record_result(d.apply.unwrap(), &all_failed()));
        let d = s.tick(at(21, 0, 30), ids(), &entries());
        assert!(d.apply.is_some());
    }

    #[test]
    fn partial_success_marks_applied() {
        let mut s = SchedulerState::default();
        let d = s.tick(at(19, 0, 0), ids(), &entries());
        let partial = ApplyResult { attempted: 2, ..all_failed() };
        assert!(s.record_result(d.apply.unwrap(), &partial));
        assert_eq!(s.tick(at(19, 0, 30), ids(), &entries()).apply, None);
    }

    #[test]
    fn zero_monitors_marks_applied() {
        let mut s = SchedulerState::default();
        let d = s.tick(at(19, 0, 0), vec![], &entries());
        assert!(s.record_result(d.apply.unwrap(), &ApplyResult::default()));
    }

    #[test]
    fn gives_up_after_max_failed_ticks() {
        let mut s = SchedulerState::default();
        let mut now = at(19, 0, 0);
        for i in 1..=MAX_FAILED_TICKS {
            let d = s.tick(now, ids(), &entries());
            let marked = s.record_result(d.apply.expect("should keep retrying"), &all_failed());
            assert_eq!(marked, i == MAX_FAILED_TICKS);
            now += ChronoDuration::seconds(30);
        }
        assert_eq!(s.tick(now, ids(), &entries()).apply, None);
    }

    #[test]
    fn saving_schedule_marks_current_slot_applied() {
        // 19:00 に 18:00 のスロットを適用済み。そこへ 18:30 のエントリを追加して保存する
        let mut s = applied_at(at(19, 0, 0));
        let mut edited = entries();
        edited.push(ScheduleEntry { time: "18:30".into(), brightness: 50 });
        s.mark_current_applied(&edited, at(19, 0, 5));
        // mark しなければ 18:30 のスロットが新しく適用されるが、mark したので適用されない
        assert_eq!(s.tick(at(19, 0, 30), ids(), &edited).apply, None);
    }

    #[test]
    fn empty_schedule_does_nothing() {
        let mut s = SchedulerState::default();
        assert_eq!(s.tick(at(19, 0, 0), ids(), &[]).apply, None);
    }

    #[test]
    fn run_once_recovers_from_failure_right_after_resume() {
        let backend = MockBackend::new(&[("a", "A", Some(50))]);
        let scheduler = Mutex::new(applied_at(at(17, 0, 0)));
        backend.fail_sets("a", 1);
        let updated = run_once(&backend, &scheduler, &entries(), at(19, 0, 0), &|_| {});
        assert!(updated);
        assert_eq!(backend.set_calls(), vec![("a".to_string(), 80), ("a".to_string(), 80)]);
        let again = run_once(&backend, &scheduler, &entries(), at(19, 0, 30), &|_| {});
        assert!(!again);
        assert_eq!(backend.set_calls().len(), 2);
    }

    #[test]
    fn run_once_reports_monitor_change_without_slot() {
        let backend = MockBackend::new(&[("a", "A", Some(50))]);
        let scheduler = Mutex::new(SchedulerState::default());
        run_once(&backend, &scheduler, &[], at(19, 0, 0), &|_| {});
        backend.replace_monitors(&[("a", "A", Some(50)), ("b", "B", Some(50))]);
        assert!(run_once(&backend, &scheduler, &[], at(19, 0, 30), &|_| {}));
    }
}
```

- [ ] **Step 2: テストが失敗することを確認する**

```bash
cd src-tauri && cargo test scheduler
```

期待する結果：`SchedulerState` などが未定義というコンパイルエラーになる。

- [ ] **Step 3: 実装する**

```rust
//! 30秒ごとの tick で「今のスロット」と「適用済みのスロット」を比べ、必要なら輝度を適用する。
//! 壁時計が想定より大きく進んだ（または戻った）tick は、スリープ復帰とみなして再適用する。

use crate::monitor::{apply_with_retry, ApplyResult, MonitorBackend, RETRY_DELAYS};
use crate::schedule::{current_slot, ScheduleEntry, Slot};
use chrono::NaiveDateTime;
use std::sync::Mutex;
use std::time::Duration;

pub const TICK_INTERVAL: Duration = Duration::from_secs(30);
pub const RESUME_GAP_SECS: i64 = 90;
pub const MAX_FAILED_TICKS: u32 = 10;

#[derive(Debug, Default)]
pub struct SchedulerState {
    last_applied: Option<NaiveDateTime>,
    last_tick: Option<NaiveDateTime>,
    last_monitor_ids: Vec<String>,
    failed_ticks: u32,
}

#[derive(Debug, PartialEq, Eq)]
pub struct Decision {
    pub apply: Option<Slot>,
    pub resumed: bool,
    pub monitors_changed: bool,
    pub first: bool,
}

impl SchedulerState {
    pub fn tick(&mut self, now: NaiveDateTime, mut monitor_ids: Vec<String>, entries: &[ScheduleEntry]) -> Decision {
        monitor_ids.sort();
        let first = self.last_tick.is_none();
        let resumed = self.last_tick.is_some_and(|prev| {
            let gap = (now - prev).num_seconds();
            gap < 0 || gap >= RESUME_GAP_SECS
        });
        let monitors_changed = !first && monitor_ids != self.last_monitor_ids;
        self.last_tick = Some(now);
        self.last_monitor_ids = monitor_ids;

        if first || resumed || monitors_changed {
            // 再適用に失敗しても、次の tick でまた試すことになる
            self.last_applied = None;
        }
        let apply = current_slot(entries, now).filter(|slot| self.last_applied != Some(slot.id));
        Decision { apply, resumed, monitors_changed, first }
    }

    pub fn record_result(&mut self, slot: Slot, result: &ApplyResult) -> bool {
        let done = result.attempted == 0 || result.any_succeeded() || {
            self.failed_ticks += 1;
            self.failed_ticks >= MAX_FAILED_TICKS
        };
        if done {
            self.last_applied = Some(slot.id);
            self.failed_ticks = 0;
        }
        done
    }

    /// スケジュールの保存時に呼ぶ。編集した瞬間には輝度を変えず、次のスロットから効かせる
    pub fn mark_current_applied(&mut self, entries: &[ScheduleEntry], now: NaiveDateTime) {
        self.last_applied = current_slot(entries, now).map(|s| s.id);
        self.failed_ticks = 0;
    }
}

/// tick を1回処理する。monitors-updated を送るべきなら true を返す
pub fn run_once(
    backend: &dyn MonitorBackend,
    scheduler: &Mutex<SchedulerState>,
    entries: &[ScheduleEntry],
    now: NaiveDateTime,
    sleep: &dyn Fn(Duration),
) -> bool {
    let ids = backend.list().into_iter().map(|m| m.id).collect();
    let decision = scheduler.lock().unwrap().tick(now, ids, entries);
    if decision.resumed {
        log::info!("detected resume or clock change at {now}");
    }
    if decision.monitors_changed {
        log::info!("monitor configuration changed");
    }
    let Some(slot) = decision.apply else {
        return decision.monitors_changed;
    };

    log::info!("applying slot {} -> {}%", slot.id, slot.brightness);
    let result = apply_with_retry(backend, slot.brightness, &RETRY_DELAYS, sleep);
    for f in &result.failed {
        log::warn!("failed to apply to {} ({}): {}", f.name, f.id, f.error);
    }
    if !scheduler.lock().unwrap().record_result(slot, &result) {
        log::warn!("slot {} not applied to any monitor; will retry next tick", slot.id);
    }
    true
}
```

- [ ] **Step 4: テストが通ることを確認する**

```bash
cd src-tauri && cargo test scheduler
```

期待する結果：15件すべて PASS。

- [ ] **Step 5: コミットする**

```bash
git add src-tauri/src/lib.rs src-tauri/src/scheduler.rs
git commit -m "feat: add resume-aware brightness scheduler"
```

---

### Task 7: AppState・command・起動処理（`state.rs`、`commands.rs`、`lib.rs`）

**Files:**
- Create: `src-tauri/src/state.rs`、`src-tauri/src/commands.rs`
- Modify: `src-tauri/src/lib.rs`、`src-tauri/src/scheduler.rs`、`src-tauri/Cargo.toml`

**Interfaces:**
- Consumes: Task 2〜6 のすべて
- Produces:
  - `pub struct AppState { pub backend: Arc<dyn MonitorBackend>, pub settings: SettingsStore, pub scheduler: Mutex<SchedulerState> }`
  - command（JS からは snake_case の名前で呼ぶ。引数のキーはそのまま）：
    - `get_monitors() -> Vec<MonitorInfo>`
    - `set_brightness_all({ value }) -> ApplyResult`
    - `set_brightness({ id, value }) -> ApplyResult`
    - `get_settings() -> Settings`
    - `save_schedule({ entries }) -> Settings`（検証エラーは `"2行目: ..."` 形式の文字列で reject）
    - `reset_schedule() -> Settings`
    - `set_autostart({ enabled }) -> Settings`
  - event：`monitors-updated`（payload は `MonitorInfo[]`）
  - `scheduler::spawn(app: AppHandle)`
  - `pub const AUTOSTART_FLAG: &str = "--autostart";`（`lib.rs`）

この Task では、Rust 側の command が JS から呼べるところまでを作る。トレイとウィンドウの表示制御は Task 8 で行う。

- [ ] **Step 1: プラグインの依存を追加する**

`src-tauri/Cargo.toml` の `[dependencies]` に追加する。

```toml
tauri-plugin-log = "2"
tauri-plugin-autostart = "2"
tauri-plugin-single-instance = "2"
```

`tauri` の行の features に `"tray-icon"` と `"image-png"` を加える（例：`tauri = { version = "2", features = ["tray-icon", "image-png"] }`）。`image-png` は Task 8 でトレイアイコンを PNG から読むために使う。

- [ ] **Step 2: `state.rs` を書く**

```rust
use crate::monitor::MonitorBackend;
use crate::scheduler::SchedulerState;
use crate::settings::SettingsStore;
use std::sync::{Arc, Mutex};

pub struct AppState {
    pub backend: Arc<dyn MonitorBackend>,
    pub settings: SettingsStore,
    pub scheduler: Mutex<SchedulerState>,
}
```

- [ ] **Step 3: `commands.rs` を書く**

DDC 通信は 1台あたり数十ms かかり、同期 command は UI スレッドで動くため、DDC を伴う command は `spawn_blocking` で別スレッドに逃がす。

```rust
//! React から呼ぶ Tauri command

use crate::monitor::{apply_all, apply_one, read_all, ApplyResult, MonitorInfo};
use crate::schedule::{default_schedule, format_errors, normalize, ScheduleEntry};
use crate::settings::Settings;
use crate::state::AppState;
use chrono::Local;
use tauri::{AppHandle, Manager, State};
use tauri_plugin_autostart::ManagerExt;

async fn blocking<T, F>(app: AppHandle, f: F) -> Result<T, String>
where
    T: Send + 'static,
    F: FnOnce(&AppState) -> T + Send + 'static,
{
    tauri::async_runtime::spawn_blocking(move || f(&app.state::<AppState>()))
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn get_monitors(app: AppHandle) -> Result<Vec<MonitorInfo>, String> {
    blocking(app, |s| read_all(s.backend.as_ref())).await
}

#[tauri::command]
pub async fn set_brightness_all(app: AppHandle, value: u8) -> Result<ApplyResult, String> {
    blocking(app, move |s| apply_all(s.backend.as_ref(), value.min(100))).await
}

#[tauri::command]
pub async fn set_brightness(app: AppHandle, id: String, value: u8) -> Result<ApplyResult, String> {
    blocking(app, move |s| apply_one(s.backend.as_ref(), &id, value.min(100))).await
}

#[tauri::command]
pub fn get_settings(state: State<'_, AppState>) -> Settings {
    state.settings.get()
}

fn store_schedule(state: &AppState, entries: Vec<ScheduleEntry>) -> Result<Settings, String> {
    let schedule = normalize(&entries).map_err(|errors| format_errors(&errors))?;
    let settings = state
        .settings
        .update(|s| s.schedule = schedule)
        .map_err(|e| format!("設定を保存できません: {e}"))?;
    state
        .scheduler
        .lock()
        .unwrap()
        .mark_current_applied(&settings.schedule, Local::now().naive_local());
    Ok(settings)
}

#[tauri::command]
pub fn save_schedule(state: State<'_, AppState>, entries: Vec<ScheduleEntry>) -> Result<Settings, String> {
    store_schedule(&state, entries)
}

#[tauri::command]
pub fn reset_schedule(state: State<'_, AppState>) -> Result<Settings, String> {
    store_schedule(&state, default_schedule())
}

#[tauri::command]
pub fn set_autostart(app: AppHandle, state: State<'_, AppState>, enabled: bool) -> Result<Settings, String> {
    let launcher = app.autolaunch();
    let result = if enabled { launcher.enable() } else { launcher.disable() };
    result.map_err(|e| format!("自動起動の設定を変更できません: {e}"))?;
    state
        .settings
        .update(|s| s.autostart = enabled)
        .map_err(|e| format!("設定を保存できません: {e}"))
}
```

`save_schedule` と `reset_schedule` を同期 command にしているのは、DDC を伴わず、UI スレッドで順番に処理されるので保存の順序が入れ替わらないため。

- [ ] **Step 4: スケジューラーのスレッドを追加する**

`src-tauri/src/scheduler.rs` の `run_once` の下に追加する（`use` は先頭へ）。

```rust
use crate::monitor::read_all;
use crate::state::AppState;
use chrono::Local;
use tauri::{AppHandle, Emitter, Manager};

pub fn spawn(app: AppHandle) {
    std::thread::Builder::new()
        .name("scheduler".into())
        .spawn(move || loop {
            let state = app.state::<AppState>();
            let entries = state.settings.get().schedule;
            let now = Local::now().naive_local();
            if run_once(state.backend.as_ref(), &state.scheduler, &entries, now, &std::thread::sleep) {
                let _ = app.emit("monitors-updated", read_all(state.backend.as_ref()));
            }
            std::thread::sleep(TICK_INTERVAL);
        })
        .expect("failed to spawn scheduler thread");
}
```

- [ ] **Step 5: `lib.rs` を組み立てる**

`src-tauri/src/lib.rs` の全体を次のようにする（雛形の `greet` は消す）。`tray` は Task 8 で追加するので、まだ書かない。

```rust
pub mod commands;
pub mod monitor;
pub mod schedule;
pub mod scheduler;
pub mod settings;
pub mod state;

use monitor::win32::WinBackend;
use scheduler::SchedulerState;
use settings::SettingsStore;
use state::AppState;
use std::sync::{Arc, Mutex};
use tauri::{AppHandle, Manager};
use tauri_plugin_autostart::{MacosLauncher, ManagerExt};

pub const AUTOSTART_FLAG: &str = "--autostart";

/// 設定と OS の自動起動の登録を一致させる
fn sync_autostart(app: &AppHandle, enabled: bool) {
    let launcher = app.autolaunch();
    let registered = launcher.is_enabled().unwrap_or(false);
    let result = match (enabled, registered) {
        (true, false) => launcher.enable(),
        (false, true) => launcher.disable(),
        _ => Ok(()),
    };
    if let Err(e) = result {
        log::warn!("failed to sync autostart: {e}");
    }
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        // single-instance は最初に登録する必要がある
        .plugin(tauri_plugin_single_instance::init(|app, _args, _cwd| {
            if let Some(window) = app.get_webview_window("main") {
                let _ = window.show();
                let _ = window.set_focus();
            }
        }))
        .plugin(tauri_plugin_log::Builder::new().level(log::LevelFilter::Info).build())
        .plugin(tauri_plugin_autostart::init(MacosLauncher::LaunchAgent, Some(vec![AUTOSTART_FLAG])))
        .invoke_handler(tauri::generate_handler![
            commands::get_monitors,
            commands::set_brightness_all,
            commands::set_brightness,
            commands::get_settings,
            commands::save_schedule,
            commands::reset_schedule,
            commands::set_autostart,
        ])
        .setup(|app| {
            let path = app.path().app_config_dir()?.join("settings.json");
            let (settings, outcome) = SettingsStore::open(path.clone())?;
            log::info!("settings {:?}: {}", outcome, path.display());
            sync_autostart(app.handle(), settings.get().autostart);
            app.manage(AppState {
                backend: Arc::new(WinBackend::new()),
                settings,
                scheduler: Mutex::new(SchedulerState::default()),
            });
            scheduler::spawn(app.handle().clone());
            if let Some(window) = app.get_webview_window("main") {
                window.show()?;
            }
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
```

`window.show()` は暫定。Task 8 で自動起動時に表示しない処理に置き換える。

- [ ] **Step 6: ビルドとテストを確認する**

```bash
pnpm build
cd src-tauri && cargo test && cargo clippy --all-targets -- -D warnings
```

期待する結果：テストがすべて PASS し、clippy の警告がない（`pnpm build` は `generate_context!` が `dist/` を必要とするため先に実行する）。

- [ ] **Step 7: 実機で command を確認する**

`pnpm tauri dev` で起動し、ウィンドウで DevTools（右クリック →「検証」、または Ctrl+Shift+I）の Console を開いて実行する。

```js
const { invoke } = window.__TAURI_INTERNALS__;
await invoke("get_monitors");
await invoke("set_brightness_all", { value: 30 });
await invoke("get_settings");
await invoke("save_schedule", { entries: [{ time: "06:00", brightness: 70 }, { time: "06:00", brightness: 80 }] }).catch(e => e);
```

期待する結果：1つ目はモニター一覧を返す。2つ目で外部モニターが暗くなり、`{ attempted, failed }` を返す。3つ目は11件のデフォルトのスケジュールを返す。4つ目は `"2行目: 時刻 06:00 が重複しています"` で reject される。ターミナルのログに `applying slot ...` が出る（起動直後の tick）。最後に `set_brightness_all` で元の明るさに戻しておく。

- [ ] **Step 8: コミットする**

```bash
git add src-tauri
git commit -m "feat: expose monitor and settings commands and start scheduler"
```

---

### Task 8: トレイ・ウィンドウの表示制御・自動起動時の非表示（`tray.rs`）

**Files:**
- Create: `src-tauri/src/tray.rs`
- Modify: `src-tauri/src/lib.rs`

**Interfaces:**
- Consumes: `AUTOSTART_FLAG`
- Produces:
  - `pub fn create(app: &AppHandle) -> tauri::Result<()>`
  - `pub fn show_main_window(app: &AppHandle)`
  - `pub fn set_theme(app: &AppHandle, theme: tauri::Theme)`

- [ ] **Step 1: `tray.rs` を書く**

アイコンは既存の `assets/icon-dark.png`（OS がダークのとき）と `assets/icon-light.png`（ライトのとき）を埋め込む。現行の Python 版と同じ対応関係。

```rust
//! トレイアイコンとメインウィンドウの表示切り替え

use tauri::image::Image;
use tauri::menu::{Menu, MenuItem};
use tauri::tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent};
use tauri::{AppHandle, Manager, Theme};

const TRAY_ID: &str = "main-tray";
const ICON_DARK: &[u8] = include_bytes!("../../assets/icon-dark.png");
const ICON_LIGHT: &[u8] = include_bytes!("../../assets/icon-light.png");

fn icon_for(theme: Theme) -> Image<'static> {
    let bytes = match theme {
        Theme::Dark => ICON_DARK,
        _ => ICON_LIGHT,
    };
    Image::from_bytes(bytes).expect("tray icon must be a valid PNG")
}

pub fn show_main_window(app: &AppHandle) {
    if let Some(window) = app.get_webview_window("main") {
        let _ = window.unminimize();
        let _ = window.show();
        let _ = window.set_focus();
    }
}

fn toggle_main_window(app: &AppHandle) {
    if let Some(window) = app.get_webview_window("main") {
        if window.is_visible().unwrap_or(false) {
            let _ = window.hide();
        } else {
            show_main_window(app);
        }
    }
}

pub fn create(app: &AppHandle) -> tauri::Result<()> {
    let toggle = MenuItem::with_id(app, "toggle", "表示/非表示", true, None::<&str>)?;
    let quit = MenuItem::with_id(app, "quit", "終了", true, None::<&str>)?;
    let menu = Menu::with_items(app, &[&toggle, &quit])?;
    let theme = app
        .get_webview_window("main")
        .and_then(|w| w.theme().ok())
        .unwrap_or(Theme::Light);

    TrayIconBuilder::with_id(TRAY_ID)
        .icon(icon_for(theme))
        .tooltip("Dimmer")
        .menu(&menu)
        .show_menu_on_left_click(false)
        .on_menu_event(|app, event| match event.id.as_ref() {
            "toggle" => toggle_main_window(app),
            "quit" => app.exit(0),
            _ => {}
        })
        .on_tray_icon_event(|tray, event| {
            if let TrayIconEvent::Click { button: MouseButton::Left, button_state: MouseButtonState::Up, .. } = event {
                toggle_main_window(tray.app_handle());
            }
        })
        .build(app)?;
    Ok(())
}

pub fn set_theme(app: &AppHandle, theme: Theme) {
    if let Some(tray) = app.tray_by_id(TRAY_ID) {
        let _ = tray.set_icon(Some(icon_for(theme)));
    }
}
```

- [ ] **Step 2: `lib.rs` に組み込む**

`pub mod tray;` を追加する。single-instance のコールバックを次のようにする。

```rust
        .plugin(tauri_plugin_single_instance::init(|app, _args, _cwd| tray::show_main_window(app)))
```

`setup` の末尾の暫定の `window.show()` のブロックを、次のように置き換える。

```rust
            tray::create(app.handle())?;
            if !std::env::args().any(|arg| arg == AUTOSTART_FLAG) {
                tray::show_main_window(app.handle());
            }
            Ok(())
```

`.setup(...)` と `.run(...)` の間に追加する。

```rust
        .on_window_event(|window, event| match event {
            tauri::WindowEvent::CloseRequested { api, .. } => {
                // × ボタンでは終了せず、トレイに隠す
                api.prevent_close();
                let _ = window.hide();
            }
            tauri::WindowEvent::ThemeChanged(theme) => tray::set_theme(window.app_handle(), *theme),
            _ => {}
        })
```

- [ ] **Step 3: ビルドを確認する**

```bash
cd src-tauri && cargo clippy --all-targets -- -D warnings && cargo test
```

期待する結果：警告がなく、テストがすべて PASS。

- [ ] **Step 4: 実機で確認する**

`pnpm tauri dev` で起動して次を確認する。

1. ウィンドウが表示され、トレイに Dimmer のアイコンが出る
2. × ボタンでウィンドウが消え、アプリは終了しない（トレイのアイコンが残る）
3. トレイアイコンの左クリックで、表示と非表示が切り替わる
4. トレイの右クリックメニューの「終了」でアプリが終了する
5. Windows の設定で「アプリモード」をダーク/ライトに切り替えると、トレイアイコンが変わる

自動起動時の非表示は、ビルド済みの exe で確認する。

```bash
cd src-tauri && cargo build && ./target/debug/dimmer.exe --autostart
```

期待する結果：ウィンドウは開かず、トレイにアイコンだけが出る。もう一度 `./target/debug/dimmer.exe` を起動すると、2つ目は起動せず、既存のウィンドウが表示される。確認したらトレイから終了する。

- [ ] **Step 5: コミットする**

```bash
git add src-tauri/src/tray.rs src-tauri/src/lib.rs
git commit -m "feat: add tray icon, hide-on-close, and silent autostart"
```

---

### Task 9: フロントエンドの土台（依存、API 層、テーマ、スケジュールの純粋関数）

**Files:**
- Modify: `package.json`、`src/main.tsx`、`index.html`
- Create: `eslint.config.js`、`vitest.config.ts`、`src/api.ts`、`src/hooks/useIsDark.ts`、`src/hooks/useDebouncedCallback.ts`、`src/hooks/useNowMinutes.ts`、`src/lib/schedule.ts`、`src/lib/schedule.test.ts`
- Delete: `src/App.css`、`src/assets/react.svg`、`public/tauri.svg`、`public/vite.svg`（雛形のもの。存在するものだけ）

**Interfaces:**
- Produces（`src/api.ts`）：
  - 型 `Monitor`、`ApplyFailure`、`ApplyResult`、`ScheduleEntry`、`Settings`
  - `api.getMonitors()`、`api.setBrightnessAll(value)`、`api.setBrightness(id, value)`、`api.getSettings()`、`api.saveSchedule(entries)`、`api.resetSchedule()`、`api.setAutostart(enabled)`、`api.onMonitorsUpdated(cb): Promise<UnlistenFn>`
- Produces（`src/lib/schedule.ts`）：
  - `type DraftEntry = { time: string; brightness: string }`、`type RowErrors = Record<number, string>`
  - `validateDraft(rows: DraftEntry[]): { entries: ScheduleEntry[] | null; errors: RowErrors }`
  - `toDraft(entries: ScheduleEntry[]): DraftEntry[]`
  - `minutesOf(time: string): number`
  - `currentIndex(entries: ScheduleEntry[], nowMinutes: number): number | null`（entries は時刻順）
  - `nextIndex(entries: ScheduleEntry[], nowMinutes: number): number | null`
  - `type Point = { minute: number; brightness: number }`、`stepPoints(entries: ScheduleEntry[]): Point[]`
- Produces（hooks）：`useIsDark(): boolean`、`useDebouncedCallback(fn, delayMs)`、`useNowMinutes(): number`

- [ ] **Step 1: 依存を追加する**

```bash
pnpm add @fluentui/react-components @fluentui/react-icons
pnpm add -D vitest eslint @eslint/js typescript-eslint eslint-plugin-react-hooks globals
corepack use pnpm@latest
```

`corepack use` で `package.json` に `packageManager` が入る（CI の `pnpm/action-setup` がこれを読む）。

`package.json` の `scripts` を次のようにする（`dev`、`build`、`preview`、`tauri` は雛形のまま残す）。

```json
"lint": "eslint .",
"typecheck": "tsc --noEmit",
"test": "vitest run"
```

- [ ] **Step 2: ESLint と Vitest の設定を書く**

`eslint.config.js`：

```js
import js from "@eslint/js";
import globals from "globals";
import reactHooks from "eslint-plugin-react-hooks";
import tseslint from "typescript-eslint";

export default tseslint.config(
  { ignores: ["dist", "src-tauri", ".superpowers"] },
  {
    files: ["**/*.{ts,tsx}"],
    extends: [js.configs.recommended, ...tseslint.configs.recommended],
    languageOptions: { globals: globals.browser },
    plugins: { "react-hooks": reactHooks },
    rules: {
      "react-hooks/rules-of-hooks": "error",
      "react-hooks/exhaustive-deps": "warn",
    },
  },
);
```

`vitest.config.ts`：

```ts
import { defineConfig } from "vitest/config";

export default defineConfig({
  test: { include: ["src/**/*.test.ts"] },
});
```

- [ ] **Step 3: スケジュールの純粋関数の失敗するテストを書く**

`src/lib/schedule.test.ts`：

```ts
import { describe, expect, it } from "vitest";
import { currentIndex, nextIndex, stepPoints, toDraft, validateDraft } from "./schedule";

const entries = [
  { time: "06:00", brightness: 70 },
  { time: "18:00", brightness: 80 },
  { time: "23:00", brightness: 0 },
];
const min = (h: number, m = 0) => h * 60 + m;

describe("validateDraft", () => {
  it("valid rows are converted to entries", () => {
    const r = validateDraft([{ time: "06:00", brightness: " 50 " }]);
    expect(r.errors).toEqual({});
    expect(r.entries).toEqual([{ time: "06:00", brightness: 50 }]);
  });

  it.each(["", "abc", "150", "7.5", "-1", "1e2"])("rejects brightness %j", (b) => {
    const r = validateDraft([{ time: "06:00", brightness: b }]);
    expect(r.entries).toBeNull();
    expect(r.errors[0]).toBeDefined();
  });

  it("rejects empty time", () => {
    expect(validateDraft([{ time: "", brightness: "50" }]).errors[0]).toBeDefined();
  });

  it("marks the later duplicate row", () => {
    const r = validateDraft([
      { time: "06:00", brightness: "50" },
      { time: "06:00", brightness: "60" },
    ]);
    expect(Object.keys(r.errors)).toEqual(["1"]);
    expect(r.entries).toBeNull();
  });

  it("accepts an empty schedule", () => {
    expect(validateDraft([])).toEqual({ entries: [], errors: {} });
  });

  it("toDraft round-trips", () => {
    expect(validateDraft(toDraft(entries)).entries).toEqual(entries);
  });
});

describe("currentIndex / nextIndex", () => {
  it("picks the latest entry not after now", () => {
    expect(currentIndex(entries, min(19))).toBe(1);
    expect(currentIndex(entries, min(18))).toBe(1);
  });

  it("before the first entry uses the previous day's last entry", () => {
    expect(currentIndex(entries, min(2))).toBe(2);
  });

  it("next wraps to the first entry", () => {
    expect(nextIndex(entries, min(19))).toBe(2);
    expect(nextIndex(entries, min(23, 30))).toBe(0);
  });

  it("empty schedule has no current or next", () => {
    expect(currentIndex([], min(12))).toBeNull();
    expect(nextIndex([], min(12))).toBeNull();
  });
});

describe("stepPoints", () => {
  it("draws a step line from 0:00 to 24:00 starting at the previous day's level", () => {
    expect(stepPoints(entries)).toEqual([
      { minute: 0, brightness: 0 },
      { minute: 360, brightness: 0 },
      { minute: 360, brightness: 70 },
      { minute: 1080, brightness: 70 },
      { minute: 1080, brightness: 80 },
      { minute: 1380, brightness: 80 },
      { minute: 1380, brightness: 0 },
      { minute: 1440, brightness: 0 },
    ]);
  });

  it("single entry is a flat line all day", () => {
    const pts = stepPoints([{ time: "12:00", brightness: 40 }]);
    expect(pts[0]).toEqual({ minute: 0, brightness: 40 });
    expect(pts[pts.length - 1]).toEqual({ minute: 1440, brightness: 40 });
    expect(pts.every((p) => p.brightness === 40)).toBe(true);
  });

  it("empty schedule has no points", () => {
    expect(stepPoints([])).toEqual([]);
  });
});
```

- [ ] **Step 4: テストが失敗することを確認する**

```bash
pnpm test
```

期待する結果：`./schedule` が見つからずに FAIL する。

- [ ] **Step 5: `src/api.ts` と `src/lib/schedule.ts` を実装する**

`src/api.ts`：

```ts
import { invoke } from "@tauri-apps/api/core";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";

export type Monitor = { id: string; name: string; brightness: number | null };
export type ApplyFailure = { id: string; name: string; error: string };
export type ApplyResult = { attempted: number; failed: ApplyFailure[] };
export type ScheduleEntry = { time: string; brightness: number };
export type Settings = { version: number; autostart: boolean; schedule: ScheduleEntry[] };

export const api = {
  getMonitors: () => invoke<Monitor[]>("get_monitors"),
  setBrightnessAll: (value: number) => invoke<ApplyResult>("set_brightness_all", { value }),
  setBrightness: (id: string, value: number) => invoke<ApplyResult>("set_brightness", { id, value }),
  getSettings: () => invoke<Settings>("get_settings"),
  saveSchedule: (entries: ScheduleEntry[]) => invoke<Settings>("save_schedule", { entries }),
  resetSchedule: () => invoke<Settings>("reset_schedule"),
  setAutostart: (enabled: boolean) => invoke<Settings>("set_autostart", { enabled }),
  onMonitorsUpdated: (cb: (monitors: Monitor[]) => void): Promise<UnlistenFn> =>
    listen<Monitor[]>("monitors-updated", (event) => cb(event.payload)),
};
```

`src/lib/schedule.ts`：

```ts
import type { ScheduleEntry } from "../api";

/** 編集中の行。入力途中の値を保持するため、輝度も文字列で持つ */
export type DraftEntry = { time: string; brightness: string };
export type RowErrors = Record<number, string>;
export type Point = { minute: number; brightness: number };

const TIME_RE = /^([01]\d|2[0-3]):[0-5]\d$/;
const INT_RE = /^\d{1,3}$/;

export function validateDraft(rows: DraftEntry[]): { entries: ScheduleEntry[] | null; errors: RowErrors } {
  const errors: RowErrors = {};
  const seen = new Set<string>();
  rows.forEach((row, i) => {
    const brightness = row.brightness.trim();
    if (!TIME_RE.test(row.time)) {
      errors[i] = "時刻を入力してください";
    } else if (!INT_RE.test(brightness) || Number(brightness) > 100) {
      errors[i] = "輝度は 0〜100 の整数で入力してください";
    } else if (seen.has(row.time)) {
      errors[i] = `${row.time} が重複しています`;
    } else {
      seen.add(row.time);
    }
  });
  if (Object.keys(errors).length > 0) return { entries: null, errors };
  return { entries: rows.map((r) => ({ time: r.time, brightness: Number(r.brightness.trim()) })), errors };
}

export const toDraft = (entries: ScheduleEntry[]): DraftEntry[] =>
  entries.map((e) => ({ time: e.time, brightness: String(e.brightness) }));

export const minutesOf = (time: string): number => Number(time.slice(0, 2)) * 60 + Number(time.slice(3, 5));

/** 現在のスロットのエントリの index。今日のエントリがまだなければ前日の最後のエントリ */
export function currentIndex(entries: ScheduleEntry[], nowMinutes: number): number | null {
  if (entries.length === 0) return null;
  let index = entries.length - 1;
  entries.forEach((e, i) => {
    if (minutesOf(e.time) <= nowMinutes) index = i;
  });
  return index;
}

export function nextIndex(entries: ScheduleEntry[], nowMinutes: number): number | null {
  if (entries.length === 0) return null;
  const i = entries.findIndex((e) => minutesOf(e.time) > nowMinutes);
  return i === -1 ? 0 : i;
}

/** 0:00〜24:00 の階段グラフの頂点。0:00 時点は前日の最後のエントリの値 */
export function stepPoints(entries: ScheduleEntry[]): Point[] {
  if (entries.length === 0) return [];
  let level = entries[entries.length - 1].brightness;
  const points: Point[] = [{ minute: 0, brightness: level }];
  for (const e of entries) {
    const minute = minutesOf(e.time);
    points.push({ minute, brightness: level }, { minute, brightness: e.brightness });
    level = e.brightness;
  }
  points.push({ minute: 1440, brightness: level });
  return points;
}
```

- [ ] **Step 6: テストが通ることを確認する**

```bash
pnpm test
```

期待する結果：すべて PASS。

- [ ] **Step 7: hooks を書く**

`src/hooks/useIsDark.ts`：

```ts
import { useSyncExternalStore } from "react";

const query = window.matchMedia("(prefers-color-scheme: dark)");

const subscribe = (onChange: () => void) => {
  query.addEventListener("change", onChange);
  return () => query.removeEventListener("change", onChange);
};

/** OS のダークモード設定に追従する */
export const useIsDark = (): boolean => useSyncExternalStore(subscribe, () => query.matches);
```

`src/hooks/useDebouncedCallback.ts`：

```ts
import { useCallback, useEffect, useRef } from "react";

export function useDebouncedCallback<A extends unknown[]>(fn: (...args: A) => void, delayMs: number) {
  const fnRef = useRef(fn);
  const timer = useRef<ReturnType<typeof setTimeout> | undefined>(undefined);

  useEffect(() => {
    fnRef.current = fn;
  });
  useEffect(() => () => clearTimeout(timer.current), []);

  return useCallback(
    (...args: A) => {
      clearTimeout(timer.current);
      timer.current = setTimeout(() => fnRef.current(...args), delayMs);
    },
    [delayMs],
  );
}
```

`src/hooks/useNowMinutes.ts`：

```ts
import { useEffect, useState } from "react";

const nowMinutes = () => {
  const d = new Date();
  return d.getHours() * 60 + d.getMinutes();
};

/** 0:00 からの経過分。30秒ごとに更新する */
export function useNowMinutes(): number {
  const [minutes, setMinutes] = useState(nowMinutes);
  useEffect(() => {
    const timer = setInterval(() => setMinutes(nowMinutes()), 30_000);
    return () => clearInterval(timer);
  }, []);
  return minutes;
}
```

- [ ] **Step 8: `main.tsx` をテーマ対応にし、雛形の不要物を消す**

`src/main.tsx`：

```tsx
import React from "react";
import ReactDOM from "react-dom/client";
import { FluentProvider, webDarkTheme, webLightTheme } from "@fluentui/react-components";
import App from "./App";
import { useIsDark } from "./hooks/useIsDark";

function Root() {
  const isDark = useIsDark();
  return (
    <FluentProvider theme={isDark ? webDarkTheme : webLightTheme} style={{ minHeight: "100vh" }}>
      <App />
    </FluentProvider>
  );
}

ReactDOM.createRoot(document.getElementById("root") as HTMLElement).render(
  <React.StrictMode>
    <Root />
  </React.StrictMode>,
);
```

`src/App.tsx` を、次の Task で置き換えるまでの仮の中身にする。

```tsx
export default function App() {
  return <div>Dimmer</div>;
}
```

雛形の `src/App.css`、`src/assets/react.svg`、`public/tauri.svg`、`public/vite.svg` を削除する（存在するものだけ）。`index.html` の `<title>` を `Dimmer` にし、`<link rel="icon" ...>` の行を消す。`<body>` に `style="margin:0"` を付ける。

- [ ] **Step 9: lint・型・テストを確認してコミットする**

```bash
pnpm lint && pnpm typecheck && pnpm test
git add -A
git commit -m "feat: add frontend foundation with API layer and schedule helpers"
```

期待する結果：3つともエラーなく終わる。

---

### Task 10: 輝度パネルと画面全体（`App.tsx`、`BrightnessPanel.tsx`）

**Files:**
- Modify: `src/App.tsx`
- Create: `src/components/BrightnessPanel.tsx`

**Interfaces:**
- Consumes: `api`、`Monitor`、`ApplyResult`、`useDebouncedCallback`
- Produces:
  - `BrightnessPanel` props：`{ monitors: Monitor[] | null; onMonitorsChange(monitors: Monitor[]): void; onApplied(result: ApplyResult): void; onRescan(): void; onError(title: string, detail?: string): void }`
  - `App` は `notifyError(title, detail?)` を子に渡す。`SchedulePanel` と `SettingsDialog` は Task 11・12 で差し込む場所をコメントで示す

- [ ] **Step 1: `BrightnessPanel.tsx` を書く**

```tsx
import { Button, Card, Divider, makeStyles, Slider, Spinner, Text, tokens } from "@fluentui/react-components";
import { api, type ApplyResult, type Monitor } from "../api";
import { useDebouncedCallback } from "../hooks/useDebouncedCallback";

const useStyles = makeStyles({
  card: { display: "flex", flexDirection: "column", gap: tokens.spacingVerticalS },
  row: { display: "grid", gridTemplateColumns: "140px 1fr 48px", alignItems: "center", gap: tokens.spacingHorizontalM },
  label: { overflow: "hidden", textOverflow: "ellipsis", whiteSpace: "nowrap" },
  value: { textAlign: "right" },
  muted: { color: tokens.colorNeutralForeground3 },
  empty: { display: "flex", flexDirection: "column", alignItems: "flex-start", gap: tokens.spacingVerticalS },
});

type RowProps = {
  label: string;
  value: number | null;
  strong?: boolean;
  onChange(value: number): void;
  onCommit(value: number): void;
};

function BrightnessRow({ label, value, strong, onChange, onCommit }: RowProps) {
  const styles = useStyles();
  const commit = useDebouncedCallback(onCommit, 100);
  return (
    <div className={styles.row}>
      <Text className={styles.label} weight={strong ? "semibold" : "regular"} title={label}>
        {label}
      </Text>
      {value === null ? (
        <Text className={styles.muted}>読み取れません</Text>
      ) : (
        <Slider
          min={0}
          max={100}
          step={10}
          value={value}
          aria-label={label}
          onChange={(_, data) => {
            onChange(data.value);
            commit(data.value);
          }}
        />
      )}
      <Text className={styles.value} weight="semibold">
        {value === null ? "—" : `${value}%`}
      </Text>
    </div>
  );
}

type Props = {
  monitors: Monitor[] | null;
  onMonitorsChange(monitors: Monitor[]): void;
  onApplied(result: ApplyResult): void;
  onRescan(): void;
  onError(title: string, detail?: string): void;
};

export function BrightnessPanel({ monitors, onMonitorsChange, onApplied, onRescan, onError }: Props) {
  const styles = useStyles();

  if (monitors === null) return <Spinner label="モニターを検出しています…" />;
  if (monitors.length === 0) {
    return (
      <Card className={styles.empty}>
        <Text>DDC/CI 対応のモニターが見つかりません</Text>
        <Button onClick={onRescan}>再検出</Button>
      </Card>
    );
  }

  const readable = monitors.flatMap((m) => (m.brightness === null ? [] : [m.brightness]));
  const average = readable.length ? Math.round(readable.reduce((a, b) => a + b, 0) / readable.length) : null;
  const fail = (e: unknown) => onError("輝度を変更できませんでした", String(e));

  return (
    <Card className={styles.card}>
      <BrightnessRow
        label="すべて"
        value={average}
        strong
        onChange={(v) => onMonitorsChange(monitors.map((m) => (m.brightness === null ? m : { ...m, brightness: v })))}
        onCommit={(v) => api.setBrightnessAll(v).then(onApplied).catch(fail)}
      />
      <Divider />
      {monitors.map((m) => (
        <BrightnessRow
          key={m.id}
          label={m.name}
          value={m.brightness}
          onChange={(v) => onMonitorsChange(monitors.map((x) => (x.id === m.id ? { ...x, brightness: v } : x)))}
          onCommit={(v) => api.setBrightness(m.id, v).then(onApplied).catch(fail)}
        />
      ))}
    </Card>
  );
}
```

- [ ] **Step 2: `App.tsx` を書く**

```tsx
import { useCallback, useEffect, useState } from "react";
import {
  makeStyles,
  Title3,
  Toast,
  ToastBody,
  Toaster,
  ToastTitle,
  tokens,
  useId,
  useToastController,
} from "@fluentui/react-components";
import { api, type ApplyResult, type Monitor, type Settings } from "./api";
import { BrightnessPanel } from "./components/BrightnessPanel";

const useStyles = makeStyles({
  root: {
    display: "flex",
    flexDirection: "column",
    gap: tokens.spacingVerticalL,
    padding: tokens.spacingHorizontalXL,
    boxSizing: "border-box",
  },
  header: { display: "flex", justifyContent: "space-between", alignItems: "center" },
  detail: { whiteSpace: "pre-line" },
});

export default function App() {
  const styles = useStyles();
  const [monitors, setMonitors] = useState<Monitor[] | null>(null);
  const [settings, setSettings] = useState<Settings | null>(null);
  const toasterId = useId("toaster");
  const { dispatchToast } = useToastController(toasterId);

  const notifyError = useCallback(
    (title: string, detail?: string) =>
      dispatchToast(
        <Toast>
          <ToastTitle>{title}</ToastTitle>
          {detail && <ToastBody className={styles.detail}>{detail}</ToastBody>}
        </Toast>,
        { intent: "error" },
      ),
    [dispatchToast, styles.detail],
  );

  const refreshMonitors = useCallback(() => {
    api
      .getMonitors()
      .then(setMonitors)
      .catch((e) => notifyError("モニターを取得できませんでした", String(e)));
  }, [notifyError]);

  useEffect(() => {
    refreshMonitors();
    api
      .getSettings()
      .then(setSettings)
      .catch((e) => notifyError("設定を読み込めませんでした", String(e)));
    const unlisten = api.onMonitorsUpdated(setMonitors);
    return () => {
      unlisten.then((f) => f());
    };
  }, [refreshMonitors, notifyError]);

  const reportApply = useCallback(
    (result: ApplyResult) => {
      // 輝度を読み取れないモニター（ノート PC の内蔵ディスプレイなど）への失敗は想定どおりなので知らせない
      const unreadable = new Set((monitors ?? []).filter((m) => m.brightness === null).map((m) => m.id));
      const failed = result.failed.filter((f) => !unreadable.has(f.id));
      if (failed.length > 0) {
        notifyError("一部のモニターに適用できませんでした", failed.map((f) => `${f.name}: ${f.error}`).join("\n"));
      }
    },
    [monitors, notifyError],
  );

  return (
    <div className={styles.root}>
      <header className={styles.header}>
        <Title3>Dimmer</Title3>
        {/* Task 12: SettingsDialog */}
      </header>
      <BrightnessPanel
        monitors={monitors}
        onMonitorsChange={setMonitors}
        onApplied={reportApply}
        onRescan={refreshMonitors}
        onError={notifyError}
      />
      {/* Task 11: SchedulePanel */}
      {settings && null}
      <Toaster toasterId={toasterId} position="bottom" />
    </div>
  );
}
```

`{settings && null}` は、`settings` が未使用で lint エラーになるのを避けるための一時的な行で、Task 11 で置き換える。

- [ ] **Step 3: lint と型を確認する**

```bash
pnpm lint && pnpm typecheck
```

期待する結果：エラーなし。

- [ ] **Step 4: 実機で確認する**

`pnpm tauri dev` で起動して次を確認する。

1. 「すべて」と、各モニターのスライダーが製品名付きで並ぶ
2. 「すべて」を動かすと、指を離して約100ms後に全外部モニターの明るさが変わり、各モニターの値も追従する
3. 個別のスライダーを動かすと、そのモニターだけが変わる
4. ノート PC の内蔵ディスプレイなど読めないモニターは「読み取れません」と表示され、「すべて」を動かしてもエラーのトーストは出ない。アプリの起動中に外部モニターの DDC/CI を OSD でオフにしてから「すべて」を動かすと、そのモニターのエラーのトーストが出る（画面上はまだ読み取れるモニターとして扱われているため）
5. OS をダーク/ライトに切り替えると、画面の配色が追従する

- [ ] **Step 5: コミットする**

```bash
git add src/App.tsx src/components/BrightnessPanel.tsx
git commit -m "feat: add brightness panel with per-monitor sliders"
```

---

### Task 11: スケジュールのパネル（`SchedulePanel.tsx`、`ScheduleChart.tsx`、`ScheduleList.tsx`）

**Files:**
- Create: `src/components/SchedulePanel.tsx`、`src/components/ScheduleChart.tsx`、`src/components/ScheduleList.tsx`
- Modify: `src/App.tsx`

**Interfaces:**
- Consumes: `api`、`ScheduleEntry`、`Settings`、`lib/schedule` の関数すべて、`useNowMinutes`
- Produces:
  - `SchedulePanel` props：`{ schedule: ScheduleEntry[]; onSaved(settings: Settings): void; onError(title: string, detail?: string): void }`（`schedule` は保存済みで時刻順）
  - `ScheduleChart` props：`{ entries: ScheduleEntry[]; nowMinutes: number }`
  - `ScheduleList` props：`{ rows: DraftEntry[]; errors: RowErrors; currentTime: string | null; onChange(rows: DraftEntry[]): void }`

- [ ] **Step 1: `ScheduleChart.tsx` を書く**

```tsx
import { Text, tokens } from "@fluentui/react-components";
import type { ScheduleEntry } from "../api";
import { stepPoints } from "../lib/schedule";

const W = 480;
const H = 120;
const PAD = { left: 28, right: 8, top: 8, bottom: 20 };
const x = (minute: number) => PAD.left + (minute / 1440) * (W - PAD.left - PAD.right);
const y = (brightness: number) => PAD.top + (1 - brightness / 100) * (H - PAD.top - PAD.bottom);
const labelStyle = { fill: tokens.colorNeutralForeground3, fontSize: 9 };

type Props = { entries: ScheduleEntry[]; nowMinutes: number };

/** 一日の輝度の推移（表示専用） */
export function ScheduleChart({ entries, nowMinutes }: Props) {
  const points = stepPoints(entries);
  if (points.length === 0) return <Text>スケジュールがありません</Text>;

  return (
    <svg viewBox={`0 0 ${W} ${H}`} width="100%" role="img" aria-label="一日の輝度の推移">
      {[0, 50, 100].map((b) => (
        <g key={b}>
          <line x1={PAD.left} x2={W - PAD.right} y1={y(b)} y2={y(b)} style={{ stroke: tokens.colorNeutralStroke2 }} />
          <text x={PAD.left - 4} y={y(b) + 3} textAnchor="end" style={labelStyle}>
            {b}
          </text>
        </g>
      ))}
      {[0, 6, 12, 18, 24].map((h) => (
        <text key={h} x={x(h * 60)} y={H - 6} textAnchor="middle" style={labelStyle}>
          {h}
        </text>
      ))}
      <polyline
        points={points.map((p) => `${x(p.minute)},${y(p.brightness)}`).join(" ")}
        style={{ fill: "none", stroke: tokens.colorBrandForeground1, strokeWidth: 2 }}
      />
      <line
        x1={x(nowMinutes)}
        x2={x(nowMinutes)}
        y1={PAD.top}
        y2={H - PAD.bottom}
        style={{ stroke: tokens.colorPaletteMarigoldForeground1, strokeDasharray: "3 3" }}
      />
    </svg>
  );
}
```

SVG の属性に CSS 変数（`tokens.*` は `var(--...)` の文字列）を直接書いても解決されないことがあるため、色は `style` で指定する。

- [ ] **Step 2: `ScheduleList.tsx` を書く**

```tsx
import { Button, Input, makeStyles, mergeClasses, Text, tokens } from "@fluentui/react-components";
import { DismissRegular } from "@fluentui/react-icons";
import type { DraftEntry, RowErrors } from "../lib/schedule";

const useStyles = makeStyles({
  list: { display: "flex", flexDirection: "column", gap: tokens.spacingVerticalXS },
  row: {
    display: "grid",
    gridTemplateColumns: "auto 96px auto",
    alignItems: "center",
    columnGap: tokens.spacingHorizontalS,
    padding: `${tokens.spacingVerticalXS} ${tokens.spacingHorizontalS}`,
    borderRadius: tokens.borderRadiusMedium,
    border: `1px solid transparent`,
  },
  current: { border: `1px solid ${tokens.colorBrandStroke1}` },
  error: { gridColumn: "1 / -1", color: tokens.colorPaletteRedForeground1 },
});

type Props = {
  rows: DraftEntry[];
  errors: RowErrors;
  currentTime: string | null;
  onChange(rows: DraftEntry[]): void;
};

export function ScheduleList({ rows, errors, currentTime, onChange }: Props) {
  const styles = useStyles();
  const patch = (index: number, change: Partial<DraftEntry>) =>
    onChange(rows.map((row, i) => (i === index ? { ...row, ...change } : row)));

  return (
    <div className={styles.list}>
      {rows.map((row, i) => (
        <div key={i} className={mergeClasses(styles.row, row.time === currentTime && styles.current)}>
          <Input type="time" value={row.time} aria-label="時刻" onChange={(_, d) => patch(i, { time: d.value })} />
          <Input
            value={row.brightness}
            inputMode="numeric"
            contentAfter="%"
            aria-label="輝度"
            onChange={(_, d) => patch(i, { brightness: d.value })}
          />
          <Button
            appearance="subtle"
            icon={<DismissRegular />}
            aria-label="削除"
            onClick={() => onChange(rows.filter((_, j) => j !== i))}
          />
          {errors[i] && <Text className={styles.error}>{errors[i]}</Text>}
        </div>
      ))}
    </div>
  );
}
```

- [ ] **Step 3: `SchedulePanel.tsx` を書く**

```tsx
import { useState } from "react";
import { Button, Caption1, Card, makeStyles, Subtitle2, tokens } from "@fluentui/react-components";
import { AddRegular } from "@fluentui/react-icons";
import { api, type ScheduleEntry, type Settings } from "../api";
import { useNowMinutes } from "../hooks/useNowMinutes";
import { currentIndex, nextIndex, toDraft, validateDraft, type DraftEntry } from "../lib/schedule";
import { ScheduleChart } from "./ScheduleChart";
import { ScheduleList } from "./ScheduleList";

const useStyles = makeStyles({
  card: { display: "flex", flexDirection: "column", gap: tokens.spacingVerticalS },
  heading: { display: "flex", justifyContent: "space-between", alignItems: "baseline" },
  next: { color: tokens.colorBrandForeground1 },
  actions: { display: "flex", gap: tokens.spacingHorizontalS },
});

type Props = {
  schedule: ScheduleEntry[];
  onSaved(settings: Settings): void;
  onError(title: string, detail?: string): void;
};

export function SchedulePanel({ schedule, onSaved, onError }: Props) {
  const styles = useStyles();
  const [rows, setRows] = useState<DraftEntry[]>(() => toDraft(schedule));
  const nowMinutes = useNowMinutes();
  const { errors } = validateDraft(rows);
  const fail = (e: unknown) => onError("スケジュールを保存できませんでした", String(e));

  // 編集のたびに自動保存する。エラーがある間は保存しない
  const update = (next: DraftEntry[]) => {
    setRows(next);
    const { entries } = validateDraft(next);
    if (entries) api.saveSchedule(entries).then(onSaved).catch(fail);
  };

  const reset = () =>
    api
      .resetSchedule()
      .then((settings) => {
        setRows(toDraft(settings.schedule));
        onSaved(settings);
      })
      .catch(fail);

  const current = currentIndex(schedule, nowMinutes);
  const next = nextIndex(schedule, nowMinutes);

  return (
    <Card className={styles.card}>
      <div className={styles.heading}>
        <Subtitle2>スケジュール</Subtitle2>
        {next !== null && (
          <Caption1 className={styles.next}>
            次 {schedule[next].time} → {schedule[next].brightness}%
          </Caption1>
        )}
      </div>
      <ScheduleChart entries={schedule} nowMinutes={nowMinutes} />
      <ScheduleList
        rows={rows}
        errors={errors}
        currentTime={current === null ? null : schedule[current].time}
        onChange={update}
      />
      <div className={styles.actions}>
        <Button icon={<AddRegular />} onClick={() => update([...rows, { time: "00:00", brightness: "50" }])}>
          追加
        </Button>
        <Button onClick={reset}>デフォルトに戻す</Button>
      </div>
    </Card>
  );
}
```

- [ ] **Step 4: `App.tsx` に差し込む**

`import { SchedulePanel } from "./components/SchedulePanel";` を追加し、`{/* Task 11: SchedulePanel */}` と `{settings && null}` の2行を次に置き換える。

```tsx
      {settings && <SchedulePanel schedule={settings.schedule} onSaved={setSettings} onError={notifyError} />}
```

- [ ] **Step 5: lint・型・テストを確認する**

```bash
pnpm lint && pnpm typecheck && pnpm test
```

期待する結果：エラーなし。

- [ ] **Step 6: 実機で確認する**

`pnpm tauri dev` で起動して次を確認する。

1. グラフが階段状に描かれ、現在時刻に破線が引かれる。見出しに「次 HH:MM → N%」が出る
2. 現在のスロットの行が枠で強調される
3. 輝度を `150`、`abc`、空にすると、その行に赤字のエラーが出る。正しい値に戻すとエラーが消え、グラフが更新される
4. 「追加」で 00:00 / 50 の行が増える（00:00 が既にあれば重複エラーが出る）。✕ で行が消える
5. 編集してもモニターの明るさは変わらない
6. ウィンドウを閉じて（トレイに隠して）再び開いても、編集内容が残っている。アプリを終了して再起動しても残っている
7. 「デフォルトに戻す」で11件のデフォルトに戻る

- [ ] **Step 7: コミットする**

```bash
git add src/App.tsx src/components/SchedulePanel.tsx src/components/ScheduleChart.tsx src/components/ScheduleList.tsx
git commit -m "feat: add schedule panel with step chart and editable list"
```

---

### Task 12: 設定ダイアログ（`SettingsDialog.tsx`）

**Files:**
- Create: `src/components/SettingsDialog.tsx`
- Modify: `src/App.tsx`

**Interfaces:**
- Consumes: `api.setAutostart`、`Settings`、`@tauri-apps/api/app` の `getVersion`
- Produces：`SettingsDialog` props：`{ settings: Settings | null; onChange(settings: Settings): void; onError(title: string, detail?: string): void }`

- [ ] **Step 1: `SettingsDialog.tsx` を書く**

```tsx
import { useEffect, useState } from "react";
import {
  Button,
  Caption1,
  Dialog,
  DialogActions,
  DialogBody,
  DialogContent,
  DialogSurface,
  DialogTitle,
  DialogTrigger,
  makeStyles,
  Switch,
  tokens,
} from "@fluentui/react-components";
import { SettingsRegular } from "@fluentui/react-icons";
import { getVersion } from "@tauri-apps/api/app";
import { api, type Settings } from "../api";

const useStyles = makeStyles({
  content: { display: "flex", flexDirection: "column", gap: tokens.spacingVerticalM },
});

type Props = {
  settings: Settings | null;
  onChange(settings: Settings): void;
  onError(title: string, detail?: string): void;
};

export function SettingsDialog({ settings, onChange, onError }: Props) {
  const styles = useStyles();
  const [version, setVersion] = useState("");

  useEffect(() => {
    getVersion().then(setVersion);
  }, []);

  return (
    <Dialog>
      <DialogTrigger disableButtonEnhancement>
        <Button appearance="subtle" icon={<SettingsRegular />} aria-label="設定" />
      </DialogTrigger>
      <DialogSurface>
        <DialogBody>
          <DialogTitle>設定</DialogTitle>
          <DialogContent className={styles.content}>
            <Switch
              label="ログイン時に起動"
              checked={settings?.autostart ?? false}
              disabled={settings === null}
              onChange={(_, data) =>
                api
                  .setAutostart(data.checked)
                  .then(onChange)
                  .catch((e) => onError("自動起動の設定を変更できませんでした", String(e)))
              }
            />
            <Caption1>バージョン {version}</Caption1>
          </DialogContent>
          <DialogActions>
            <DialogTrigger disableButtonEnhancement>
              <Button appearance="secondary">閉じる</Button>
            </DialogTrigger>
          </DialogActions>
        </DialogBody>
      </DialogSurface>
    </Dialog>
  );
}
```

- [ ] **Step 2: `App.tsx` に差し込む**

`import { SettingsDialog } from "./components/SettingsDialog";` を追加し、`{/* Task 12: SettingsDialog */}` を次に置き換える。

```tsx
        <SettingsDialog settings={settings} onChange={setSettings} onError={notifyError} />
```

- [ ] **Step 3: lint・型を確認する**

```bash
pnpm lint && pnpm typecheck
```

期待する結果：エラーなし。

- [ ] **Step 4: 実機で確認する**

`pnpm tauri dev` で起動して次を確認する。

1. 右上の歯車でダイアログが開き、「ログイン時に起動」がオン、「バージョン 1.0.0」と表示される
2. トグルをオフにして PowerShell で `Get-ItemProperty HKCU:\Software\Microsoft\Windows\CurrentVersion\Run` を実行すると、Dimmer のエントリが消えている。オンに戻すとエントリが戻る（dev ビルドではパスが `target\debug\dimmer.exe` になる）
3. ダイアログを閉じて開き直しても、トグルの状態が保たれている

- [ ] **Step 5: コミットする**

```bash
git add src/App.tsx src/components/SettingsDialog.tsx
git commit -m "feat: add settings dialog with autostart toggle"
```

---

### Task 13: CI・リリース・pre-commit

**Files:**
- Create: `.github/workflows/ci.yml`
- Modify: `.github/workflows/release.yml`、`.pre-commit-config.yaml`
- Delete: `.github/workflows/pre-commit.yml`

**Interfaces:**
- Produces: PR と main への push で lint・型・テスト・clippy・cargo test が走る。`v*` の tag を push すると NSIS インストーラー付きの下書きの Release ができる

- [ ] **Step 1: `ci.yml` を書き、`pre-commit.yml` を消す**

```yaml
name: ci

on:
  pull_request:
  push:
    branches: [main]

jobs:
  check:
    runs-on: windows-latest
    steps:
      - uses: actions/checkout@v4
      - uses: pnpm/action-setup@v4
      - uses: actions/setup-node@v4
        with:
          node-version: 22
          cache: pnpm
      - uses: dtolnay/rust-toolchain@stable
        with:
          components: rustfmt, clippy
      - uses: Swatinem/rust-cache@v2
        with:
          workspaces: src-tauri
      - run: pnpm install --frozen-lockfile
      - run: pnpm lint
      - run: pnpm typecheck
      - run: pnpm test
      # src-tauri の generate_context! が dist/ を必要とするため、cargo の前にビルドする
      - run: pnpm build
      - run: cargo fmt --check
        working-directory: src-tauri
      - run: cargo clippy --all-targets -- -D warnings
        working-directory: src-tauri
      - run: cargo test
        working-directory: src-tauri
```

```bash
git rm .github/workflows/pre-commit.yml
```

- [ ] **Step 2: `release.yml` を置き換える**

```yaml
name: Build and Release

on:
  push:
    # Release when a new tag is pushed (e.g. `git push origin v1.2.3`)
    tags:
      - "v*"

jobs:
  release:
    runs-on: windows-latest
    permissions:
      contents: write
    steps:
      - uses: actions/checkout@v4
      - uses: pnpm/action-setup@v4
      - uses: actions/setup-node@v4
        with:
          node-version: 22
          cache: pnpm
      - uses: dtolnay/rust-toolchain@stable
      - uses: Swatinem/rust-cache@v2
        with:
          workspaces: src-tauri
      - run: pnpm install --frozen-lockfile
      - uses: tauri-apps/tauri-action@v0
        env:
          GITHUB_TOKEN: ${{ secrets.GITHUB_TOKEN }}
        with:
          tagName: ${{ github.ref_name }}
          releaseName: Release ${{ github.ref_name }}
          releaseDraft: true
          args: --bundles nsis
```

- [ ] **Step 3: `.pre-commit-config.yaml` を置き換える**

```yaml
repos:
  - repo: local
    hooks:
      - id: eslint
        name: eslint
        entry: pnpm lint
        language: system
        pass_filenames: false
        files: \.(ts|tsx|js)$
      - id: tsc
        name: tsc
        entry: pnpm typecheck
        language: system
        pass_filenames: false
        files: \.(ts|tsx)$
      - id: cargo-fmt
        name: cargo fmt
        entry: cargo fmt --manifest-path src-tauri/Cargo.toml --check
        language: system
        pass_filenames: false
        files: \.rs$
```

- [ ] **Step 4: CI と同じコマンドを手元で流す**

```bash
pnpm install --frozen-lockfile && pnpm lint && pnpm typecheck && pnpm test && pnpm build
cd src-tauri && cargo fmt --check && cargo clippy --all-targets -- -D warnings && cargo test
```

期待する結果：すべて成功する。`cargo fmt --check` が差分を出した場合は `cargo fmt` を実行し、その結果もこのコミットに含める。

- [ ] **Step 5: インストーラーのビルドを手元で確認する**

```bash
pnpm tauri build --bundles nsis
ls src-tauri/target/release/bundle/nsis/
```

期待する結果：`Dimmer_1.0.0_x64-setup.exe` ができる。サイズを記録し、ユーザーに報告する（Python 版の exe と比べるため）。

- [ ] **Step 6: コミットする**

```bash
git add -A .github .pre-commit-config.yaml src-tauri
git commit -m "ci: replace Python workflows with Tauri lint, test, and release"
```

---

### Task 14: 実機での確認、Python コードの削除、README

**Files:**
- Delete: `main.py`、`components/`、`modules/`、`pyproject.toml`、`uv.lock`、`build.ps1`、`.python-version`
- Modify: `README.md`、`.gitignore`、`assets/app-window.png`

**Interfaces:**
- Consumes: Task 13 で作ったインストーラー

- [ ] **Step 1: インストーラーで入れて、実機での確認項目を一緒に確認する**

Task 13 の `Dimmer_1.0.0_x64-setup.exe` をインストールし、spec の 10.2 の確認項目を**ユーザーと一緒に**確認する。スリープ、モニターの抜き差し、ログインはユーザーの操作が必要なので、各項目の手順を伝えて結果を聞く。

- [ ] スケジュール時刻をまたいでスリープし、復帰すると直近の値が適用される（手順：現在時刻の2〜3分後にエントリを追加し、スリープしてその時刻を過ぎてから復帰する。復帰後30秒以内に変わればよい）
- [ ] 復帰直後に DDC が失敗しても、リトライで適用される（ログ `%LOCALAPPDATA%\io.github.nigimitama.dimmer\logs\` に `failed to apply` と、その後の成功が残っていれば確認できる。失敗が起きなかった場合は「未発生」と記録する）
- [ ] モニターを抜き差しすると、一覧が更新され、直近の値が適用される
- [ ] 手動で変えた値が、次のスケジュール時刻まで維持される
- [ ] 自動起動を有効にすると、ログイン後にトレイに常駐し、ウィンドウは開かない
- [ ] 二重起動すると、既存のウィンドウが前面に出る
- [ ] OS のダーク/ライトを切り替えると、UI とトレイアイコンが追従する
- [ ] モニター名が表示される（取得できない場合は「Monitor N」になる）

不合格の項目があれば、ここで止めてユーザーに報告し、修正方針を相談する。

- [ ] **Step 2: Python 関連のファイルを削除する**

```bash
git rm -r main.py components modules pyproject.toml uv.lock build.ps1 .python-version
```

`.gitignore` から Python 用の行（`__pycache__`、`*.spec`、`dimmer.json`、`dist/dimmer.json`、`generated*.ico`）を消す。

- [ ] **Step 3: スクリーンショットを差し替える**

新しいアプリのウィンドウのスクリーンショットを撮って `assets/app-window.png` を上書きするよう、ユーザーに頼む（Win+Shift+S でウィンドウを選択して保存）。ユーザーが後回しにしたい場合は、古い画像のまま進め、その旨を最終報告に書く。

- [ ] **Step 4: README を更新する**

`README.md` を次の内容にする。

```markdown
![dimmer-logo](assets/logo/dimmer-logo.png)

# Dimmer

GUI application to control luminance of monitors.

![app-window](assets/app-window.png)

## ✨️ Features

- Manage the brightness of external monitors collectively or individually (DDC/CI)
- The schedule function automatically adjusts brightness at specified times
- Catches up after sleep, startup, or reconnecting a monitor: the brightness scheduled for the current time is applied automatically
- Runs in the system tray and can start automatically at login

## 📩 Download

Download the latest installer (`Dimmer_x.y.z_x64-setup.exe`) from the [releases](https://github.com/nigimitama/dimmer/releases) page.

## 💻️ Supported Platform

- Windows 11
- Windows 10

## 🛠️ Development

Requirements: Rust (stable), Node.js 22+, pnpm (via `corepack enable`), Visual Studio Build Tools with the C++ workload.

```sh
pnpm install
pnpm tauri dev                     # run the app
pnpm lint && pnpm typecheck && pnpm test
cd src-tauri && cargo test         # run `pnpm build` first so that dist/ exists
pnpm tauri build --bundles nsis    # build the installer
```

Settings are stored in `%APPDATA%\io.github.nigimitama.dimmer\settings.json` and logs in `%LOCALAPPDATA%\io.github.nigimitama.dimmer\logs\`.

To release, push a tag such as `v1.0.0`; GitHub Actions builds the installer and creates a draft release.
```

- [ ] **Step 5: 最終確認とコミット**

```bash
pnpm lint && pnpm typecheck && pnpm test && pnpm build
cd src-tauri && cargo clippy --all-targets -- -D warnings && cargo test && cd ..
git add -A
git status --short
git commit -m "chore: remove Python implementation and update README for 1.0.0"
```

期待する結果：すべて成功し、`git status` に意図しないファイル（`.superpowers/`、`node_modules/`、`target/`）が含まれていない。
