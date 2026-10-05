//! 30秒ごとの tick で「今のスロット」と「適用済みのスロット」を比べ、必要なら輝度を適用する。
//! 壁時計が想定より大きく進んだ（または戻った）tick は、スリープ復帰とみなして再適用する。

use crate::monitor::{apply_with_retry, read_all, ApplyResult, MonitorBackend, RETRY_DELAYS};
use crate::schedule::{current_slot, ScheduleEntry, Slot};
use crate::state::AppState;
use chrono::{Local, NaiveDateTime};
use std::sync::Mutex;
use std::time::Duration;
use tauri::{AppHandle, Emitter, Manager};

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
    pub fn tick(
        &mut self,
        now: NaiveDateTime,
        mut monitor_ids: Vec<String>,
        entries: &[ScheduleEntry],
    ) -> Decision {
        monitor_ids.sort();
        let first = self.last_tick.is_none();
        let resumed = self.last_tick.is_some_and(|prev| {
            let gap = (now - prev).num_seconds();
            !(0..RESUME_GAP_SECS).contains(&gap)
        });
        let monitors_changed = !first && monitor_ids != self.last_monitor_ids;
        self.last_tick = Some(now);
        self.last_monitor_ids = monitor_ids;

        if first || resumed || monitors_changed {
            // 再適用に失敗しても、次の tick でまた試すことになる。
            // 失敗回数も数え直し、復帰前の失敗が続いていてもすぐには諦めないようにする
            self.last_applied = None;
            self.failed_ticks = 0;
        }
        let apply = current_slot(entries, now).filter(|slot| self.last_applied != Some(slot.id));
        Decision {
            apply,
            resumed,
            monitors_changed,
            first,
        }
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
        log::warn!(
            "slot {} not applied to any monitor; will retry next tick",
            slot.id
        );
    }
    true
}

pub fn spawn(app: AppHandle) {
    std::thread::Builder::new()
        .name("scheduler".into())
        .spawn(move || loop {
            let state = app.state::<AppState>();
            let entries = state.settings.get().schedule;
            let now = Local::now().naive_local();
            if run_once(
                state.backend.as_ref(),
                &state.scheduler,
                &entries,
                now,
                &std::thread::sleep,
            ) {
                let _ = app.emit("monitors-updated", read_all(state.backend.as_ref()));
            }
            std::thread::sleep(TICK_INTERVAL);
        })
        .expect("failed to spawn scheduler thread");
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::monitor::mock::MockBackend;
    use crate::monitor::{ApplyFailure, ApplyResult};
    use crate::schedule::ScheduleEntry;
    use chrono::{Duration as ChronoDuration, NaiveDate};

    fn entries() -> Vec<ScheduleEntry> {
        vec![
            ScheduleEntry {
                time: "06:00".into(),
                brightness: 70,
            },
            ScheduleEntry {
                time: "18:00".into(),
                brightness: 80,
            },
        ]
    }

    fn at(h: u32, m: u32, s: u32) -> NaiveDateTime {
        NaiveDate::from_ymd_opt(2026, 10, 5)
            .unwrap()
            .and_hms_opt(h, m, s)
            .unwrap()
    }

    fn ids() -> Vec<String> {
        vec!["a".into(), "b".into()]
    }

    fn ok(n: usize) -> ApplyResult {
        ApplyResult {
            attempted: n,
            failed: vec![],
        }
    }

    fn all_failed() -> ApplyResult {
        ApplyResult {
            attempted: 1,
            failed: vec![ApplyFailure {
                id: "a".into(),
                name: "A".into(),
                error: "x".into(),
            }],
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
        let d = s.tick(
            at(19, 0, 0) + ChronoDuration::seconds(RESUME_GAP_SECS),
            ids(),
            &entries(),
        );
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
        let partial = ApplyResult {
            attempted: 2,
            ..all_failed()
        };
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
    fn failure_streak_does_not_carry_over_a_resume() {
        let mut s = SchedulerState::default();
        let mut now = at(19, 0, 0);
        for _ in 1..MAX_FAILED_TICKS {
            let d = s.tick(now, ids(), &entries());
            assert!(!s.record_result(d.apply.unwrap(), &all_failed()));
            now += ChronoDuration::seconds(30);
        }
        // スリープから復帰した直後の1回目の失敗では、まだ諦めない
        let d = s.tick(now + ChronoDuration::hours(1), ids(), &entries());
        assert!(d.resumed);
        assert!(!s.record_result(d.apply.unwrap(), &all_failed()));
    }

    #[test]
    fn saving_schedule_marks_current_slot_applied() {
        // 19:00 に 18:00 のスロットを適用済み。そこへ 18:30 のエントリを追加して保存する
        let mut s = applied_at(at(19, 0, 0));
        let mut edited = entries();
        edited.push(ScheduleEntry {
            time: "18:30".into(),
            brightness: 50,
        });
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
        assert_eq!(
            backend.set_calls(),
            vec![("a".to_string(), 80), ("a".to_string(), 80)]
        );
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
