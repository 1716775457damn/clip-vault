mod app;
mod monitor;
mod store;

use app::App;
use global_hotkey::{GlobalHotKeyEvent, GlobalHotKeyManager, hotkey::{Code, HotKey, Modifiers}};
use std::sync::{Arc, atomic::{AtomicBool, Ordering}};
use std::sync::mpsc;

fn main() -> eframe::Result {
    let (tx, rx) = mpsc::channel();
    monitor::start(tx);

    let manager = GlobalHotKeyManager::new().expect("hotkey manager failed");
    let hotkey = HotKey::new(Some(Modifiers::CONTROL | Modifiers::SHIFT), Code::KeyV);
    manager.register(hotkey).expect("hotkey register failed");

    let triggered = Arc::new(AtomicBool::new(false));
    let triggered_clone = triggered.clone();

    std::thread::spawn(move || {
        loop {
            if let Ok(_) = GlobalHotKeyEvent::receiver().recv() {
                triggered_clone.store(true, Ordering::Relaxed);
            }
        }
    });

    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_title("Clip Vault")
            .with_inner_size([360.0, 560.0])
            .with_min_inner_size([300.0, 400.0])
            .with_always_on_top()
            .with_close_button(true),
        ..Default::default()
    };

    eframe::run_native("Clip Vault", options, Box::new(|cc| {
        // Load Microsoft YaHei for CJK support
        let mut fonts = egui::FontDefinitions::default();
        if let Ok(bytes) = std::fs::read("C:\\Windows\\Fonts\\msyh.ttc") {
            fonts.font_data.insert("msyh".to_owned(), egui::FontData::from_owned(bytes).into());
            fonts.families.entry(egui::FontFamily::Proportional).or_default().push("msyh".to_owned());
            fonts.families.entry(egui::FontFamily::Monospace).or_default().push("msyh".to_owned());
        }
        cc.egui_ctx.set_fonts(fonts);
        Ok(Box::new(App::new(rx, triggered)))
    }))
}
