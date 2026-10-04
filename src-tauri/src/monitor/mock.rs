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
