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

    let icon = tray_icon::Icon::from_rgba(make_icon(), 32, 32).expect("icon failed");
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

/// 32×32 RGBA clipboard icon with rounded corners, clip bar, and text lines
fn make_icon() -> Vec<u8> {
    const S: usize = 32;
    let mut px = vec![0u8; S * S * 4];

    let set = |px: &mut Vec<u8>, x: usize, y: usize, r: u8, g: u8, b: u8, a: u8| {
        if x < S && y < S {
            let i = (y * S + x) * 4;
            px[i] = r; px[i+1] = g; px[i+2] = b; px[i+3] = a;
        }
    };

    // Draw filled rounded rectangle (clipboard body): x 3..28, y 5..30, radius 3
    for y in 0..S {
        for x in 0..S {
            let (fx, fy) = (x as i32, y as i32);
            // Body: rounded rect 3..28, 5..30
            let in_body = fx >= 3 && fx <= 28 && fy >= 5 && fy <= 30;
            // Corner cutouts
            let corner = (fx < 6 && fy < 8) || (fx > 25 && fy < 8)
                      || (fx < 6 && fy > 27) || (fx > 25 && fy > 27);
            // Clip bar at top: x 10..21, y 3..9
            let clip_bar = fx >= 10 && fx <= 21 && fy >= 3 && fy <= 9;
            // Clip bar hole: x 13..18, y 3..6
            let clip_hole = fx >= 13 && fx <= 18 && fy >= 3 && fy <= 6;

            if clip_bar && !clip_hole {
                // Clip bar: darker blue-grey
                set(&mut px, x, y, 90, 110, 160, 255);
            } else if in_body && !corner {
                // Body gradient: top is lighter, bottom darker
                let t = fy as f32 / S as f32;
                let r = (100.0 - t * 20.0) as u8;
                let g = (150.0 - t * 30.0) as u8;
                let b = (230.0 - t * 20.0) as u8;
                set(&mut px, x, y, r, g, b, 255);
            }
        }
    }

    // Text lines on clipboard body (white, semi-transparent)
    // Line 1: y=13, x 7..24
    for x in 7..25usize { set(&mut px, x, 13, 255, 255, 255, 200); }
    for x in 7..25usize { set(&mut px, x, 14, 255, 255, 255, 200); }
    // Line 2: y=18, x 7..24
    for x in 7..25usize { set(&mut px, x, 18, 255, 255, 255, 200); }
    for x in 7..25usize { set(&mut px, x, 19, 255, 255, 255, 200); }
    // Line 3: y=23, x 7..18 (shorter)
    for x in 7..19usize { set(&mut px, x, 23, 255, 255, 255, 200); }
    for x in 7..19usize { set(&mut px, x, 24, 255, 255, 255, 200); }

    // Border: 1px outline around body
    for y in 5..=30usize {
        for x in 3..=28usize {
            let (fx, fy) = (x as i32, y as i32);
            let on_edge = fx == 3 || fx == 28 || fy == 5 || fy == 30;
            let corner = (fx < 6 && fy < 8) || (fx > 25 && fy < 8)
                      || (fx < 6 && fy > 27) || (fx > 25 && fy > 27);
            if on_edge && !corner {
                set(&mut px, x, y, 60, 90, 180, 255);
            }
        }
    }

    px
}
