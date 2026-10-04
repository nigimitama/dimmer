//! 実機でモニターの ID・名前・輝度を確認する: cargo run --example list_monitors
use dimmer_lib::monitor::{read_all, win32::WinBackend};

fn main() {
    for m in read_all(&WinBackend::new()) {
        println!("{:<24} {:>9}  {}", m.name, format!("{:?}", m.brightness), m.id);
    }
}
