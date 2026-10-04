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
