use crate::monitor::MonitorBackend;
use crate::scheduler::SchedulerState;
use crate::settings::SettingsStore;
use std::sync::{Arc, Mutex};

pub struct AppState {
    pub backend: Arc<dyn MonitorBackend>,
    pub settings: SettingsStore,
    pub scheduler: Mutex<SchedulerState>,
}
