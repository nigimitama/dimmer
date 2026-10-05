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
            MonitorInfo {
                id: m.id,
                name: m.name,
                brightness,
            }
        })
        .collect()
}

fn apply_to(backend: &dyn MonitorBackend, targets: &[MonitorIdent], value: u8) -> ApplyResult {
    let failed = targets
        .iter()
        .filter_map(|m| {
            backend
                .set_brightness(&m.id, value)
                .err()
                .map(|error| ApplyFailure {
                    id: m.id.clone(),
                    name: m.name.clone(),
                    error,
                })
        })
        .collect();
    ApplyResult {
        attempted: targets.len(),
        failed,
    }
}

pub fn apply_all(backend: &dyn MonitorBackend, value: u8) -> ApplyResult {
    apply_to(backend, &backend.list(), value)
}

pub fn apply_one(backend: &dyn MonitorBackend, id: &str, value: u8) -> ApplyResult {
    let targets: Vec<_> = backend.list().into_iter().filter(|m| m.id == id).collect();
    if targets.is_empty() {
        return ApplyResult {
            attempted: 0,
            failed: vec![ApplyFailure {
                id: id.into(),
                name: id.into(),
                error: "モニターが見つかりません".into(),
            }],
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
        result = ApplyResult {
            attempted: all.len(),
            failed: apply_to(backend, &retry, value).failed,
        };
    }
    result
}

#[cfg(test)]
mod tests {
    use super::mock::MockBackend;
    use super::*;
    use std::cell::RefCell;

    #[test]
    fn read_all_reports_unreadable_monitor_as_none() {
        let b = MockBackend::new(&[("a", "Ext", Some(60)), ("lap", "Laptop", None)]);
        let all = read_all(&b);
        assert_eq!(
            all[0],
            MonitorInfo {
                id: "a".into(),
                name: "Ext".into(),
                brightness: Some(60)
            }
        );
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
            vec![
                ("a".to_string(), 40),
                ("b".to_string(), 40),
                ("b".to_string(), 40)
            ]
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
