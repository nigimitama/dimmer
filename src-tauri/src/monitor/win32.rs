//! Win32 の Monitor Configuration API（DDC/CI）と DisplayConfig API（モニター名）による MonitorBackend 実装

use super::{MonitorBackend, MonitorIdent};
use std::collections::HashMap;
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
            let mut target = DISPLAYCONFIG_TARGET_DEVICE_NAME {
                header: DISPLAYCONFIG_DEVICE_INFO_HEADER {
                    r#type: DISPLAYCONFIG_DEVICE_INFO_GET_TARGET_NAME,
                    size: std::mem::size_of::<DISPLAYCONFIG_TARGET_DEVICE_NAME>() as u32,
                    adapterId: path.targetInfo.adapterId,
                    id: path.targetInfo.id,
                },
                ..Default::default()
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
                // PHYSICAL_MONITOR は packed 構造体なので、参照を取る前に値としてコピーする
                let description = pm.szPhysicalMonitorDescription;
                raw.push(RawMonitor { device_path: paths.get(i).cloned(), description: from_wide(&description) });
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
