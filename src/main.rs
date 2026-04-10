mod app;
mod monitor;
mod store;

use app::{App, TrayMsg};
use global_hotkey::{GlobalHotKeyEvent, GlobalHotKeyManager, hotkey::{Code, HotKey, Modifiers}};
use std::sync::{Arc, atomic::{AtomicBool, Ordering}};
use std::sync::mpsc;
use tray_icon::{TrayIconBuilder, menu::{Menu, MenuItem, MenuEvent}};

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
            if GlobalHotKeyEvent::receiver().recv().is_ok() {
                triggered_clone.store(true, Ordering::Relaxed);
            }
        }
    });

    // System tray
    let tray_menu = Menu::new();
    let show_item = MenuItem::new("显示窗口", true, None);
    let quit_item = MenuItem::new("退出", true, None);
    let show_id = show_item.id().clone();
    let quit_id  = quit_item.id().clone();
    tray_menu.append(&show_item).ok();
    tray_menu.append(&quit_item).ok();

    let icon = tray_icon::Icon::from_rgba(make_icon(), 16, 16).expect("icon failed");
    let _tray = TrayIconBuilder::new()
        .with_menu(Box::new(tray_menu))
        .with_tooltip("Clip Vault")
        .with_icon(icon)
        .build()
        .expect("tray failed");

    let (tray_tx, tray_rx) = mpsc::channel::<TrayMsg>();
    std::thread::spawn(move || {
        loop {
            if let Ok(event) = MenuEvent::receiver().recv() {
                if event.id == show_id {
                    let _ = tray_tx.send(TrayMsg::Show);
                } else if event.id == quit_id {
                    let _ = tray_tx.send(TrayMsg::Quit);
                }
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
        let mut fonts = egui::FontDefinitions::default();
        if let Ok(bytes) = std::fs::read("C:\\Windows\\Fonts\\msyh.ttc") {
            fonts.font_data.insert("msyh".to_owned(), egui::FontData::from_owned(bytes).into());
            fonts.families.entry(egui::FontFamily::Proportional).or_default().push("msyh".to_owned());
            fonts.families.entry(egui::FontFamily::Monospace).or_default().push("msyh".to_owned());
        }
        cc.egui_ctx.set_fonts(fonts);
        Ok(Box::new(App::new(rx, triggered, tray_rx)))
    }))
}

fn make_icon() -> Vec<u8> {
    let mut px = vec![0u8; 16 * 16 * 4];
    for y in 0..16usize {
        for x in 0..16usize {
            let i = (y * 16 + x) * 4;
            let on = (x >= 1 && x <= 14 && y >= 1 && y <= 14)
                && !(x >= 3 && x <= 12 && y >= 3 && y <= 12);
            if on || (x >= 4 && x <= 11 && y >= 5 && y <= 11) {
                px[i]   = 80;
                px[i+1] = 140;
                px[i+2] = 220;
                px[i+3] = 255;
            }
        }
    }
    px
}
