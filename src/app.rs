use crate::store::{ClipContent, ClipEntry, Store};
use arboard::Clipboard;
use eframe::egui;
use egui::{Color32, RichText, ScrollArea, TextEdit, TextureHandle, Vec2};
use std::collections::HashMap;
use std::sync::mpsc::Receiver;

pub struct App {
    pub store: Store,
    pub rx: Receiver<ClipContent>,
    visible: bool,
    just_shown: bool,
    query: String,
    query_lc: String,       // cached lowercase, updated only when query changes
    clipboard: Clipboard,
    img_cache: HashMap<u64, TextureHandle>,
    pub hotkey_triggered: std::sync::Arc<std::sync::atomic::AtomicBool>,
}

impl App {
    pub fn new(
        rx: Receiver<ClipContent>,
        hotkey_triggered: std::sync::Arc<std::sync::atomic::AtomicBool>,
    ) -> Self {
        Self {
            store: Store::load(),
            rx,
            visible: true,
            just_shown: true,
            query: String::new(),
            query_lc: String::new(),
            clipboard: Clipboard::new().expect("clipboard init failed"),
            img_cache: HashMap::new(),
            hotkey_triggered,
        }
    }

    fn copy_entry(&mut self, entry: &ClipEntry) {
        match &entry.content {
            ClipContent::Text(t) => { let _ = self.clipboard.set_text(t.clone()); }
            ClipContent::Image { width, height, rgba } => {
                let _ = self.clipboard.set_image(arboard::ImageData {
                    width: *width as usize,
                    height: *height as usize,
                    bytes: rgba.clone().into(),
                });
            }
        }
    }

    fn show(&mut self, ctx: &egui::Context) {
        self.visible = true;
        self.just_shown = true;
        ctx.send_viewport_cmd(egui::ViewportCommand::Visible(true));
        ctx.send_viewport_cmd(egui::ViewportCommand::Focus);
    }

    fn hide(&mut self, ctx: &egui::Context) {
        self.visible = false;
        self.query.clear();
        self.query_lc.clear();
        ctx.send_viewport_cmd(egui::ViewportCommand::Visible(false));
    }
}

impl eframe::App for App {
    // Override close to minimize instead of quit
    fn on_exit(&mut self, _gl: Option<&eframe::glow::Context>) {}

    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // Intercept window close → hide instead of exit
        if ctx.input(|i| i.viewport().close_requested()) {
            ctx.send_viewport_cmd(egui::ViewportCommand::CancelClose);
            self.hide(ctx);
            return;
        }

        // Toggle visibility via hotkey
        if self.hotkey_triggered.swap(false, std::sync::atomic::Ordering::Relaxed) {
            if self.visible { self.hide(ctx); } else { self.show(ctx); }
        }

        // Drain new clipboard entries
        while let Ok(content) = self.rx.try_recv() {
            self.store.push(content);
            if self.visible { ctx.request_repaint(); }
        }

        if !self.visible {
            ctx.request_repaint_after(std::time::Duration::from_millis(200));
            return;
        }

        if ctx.input(|i| i.key_pressed(egui::Key::Escape)) {
            self.hide(ctx);
            return;
        }

        egui::TopBottomPanel::top("search_bar").show(ctx, |ui| {
            ui.add_space(6.0);
            ui.horizontal(|ui| {
                ui.label("🔍");
                let resp = ui.add(
                    TextEdit::singleline(&mut self.query)
                        .hint_text("搜索历史…")
                        .desired_width(f32::INFINITY),
                );
                if self.just_shown { resp.request_focus(); }

                // Update cached lowercase only when query changes
                let new_lc = self.query.to_lowercase();
                if new_lc != self.query_lc { self.query_lc = new_lc; }

                if ui.button("🗑").on_hover_text("清除所有未固定条目").clicked() {
                    self.store.clear_unpinned();
                    self.img_cache.clear();
                }
            });
            ui.add_space(4.0);
        });

        egui::TopBottomPanel::bottom("hint").show(ctx, |ui| {
            ui.label(
                RichText::new(format!(
                    "{} 条记录  •  Ctrl+Shift+V 呼出  •  Esc 隐藏",
                    self.store.entries.len()
                ))
                .small()
                .color(Color32::GRAY),
            );
        });

        egui::CentralPanel::default().show(ctx, |ui| {
            // Single-pass partition: pinned + recent, filtered
            let mut pinned: Vec<&ClipEntry> = Vec::new();
            let mut recent: Vec<&ClipEntry> = Vec::new();
            for e in &self.store.entries {
                if !self.query_lc.is_empty() {
                    match &e.content {
                        ClipContent::Text(_) => {
                            if !e.text_lc.contains(&self.query_lc) { continue; }
                        }
                        ClipContent::Image { .. } => continue,
                    }
                }
                if e.pinned { pinned.push(e); } else { recent.push(e); }
            }

            let mut action: Option<Action> = None;

            ScrollArea::vertical().auto_shrink(false).show(ui, |ui| {
                if !pinned.is_empty() {
                    ui.label(RichText::new("📌 已固定").small().color(Color32::GOLD));
                    for e in &pinned {
                        if let Some(a) = render_entry(ui, e, ctx, &mut self.img_cache) {
                            action = Some(a);
                        }
                    }
                    ui.separator();
                }

                // Time-grouped recent entries
                let now = chrono::Local::now();
                let today = now.date_naive();
                let yesterday = today.pred_opt().unwrap_or(today);

                let mut last_group = "";
                for e in &recent {
                    let date = e.time.date_naive();
                    let group = if date == today { "今天" }
                        else if date == yesterday { "昨天" }
                        else { "更早" };
                    if group != last_group {
                        ui.add_space(4.0);
                        ui.label(RichText::new(group).small().color(Color32::DARK_GRAY));
                        last_group = group;
                    }
                    if let Some(a) = render_entry(ui, e, ctx, &mut self.img_cache) {
                        action = Some(a);
                    }
                }

                if pinned.is_empty() && recent.is_empty() {
                    ui.centered_and_justified(|ui| {
                        ui.label(RichText::new(
                            if self.query_lc.is_empty() { "复制任意内容即可记录" } else { "未找到匹配项" }
                        ).color(Color32::GRAY));
                    });
                }
            });

            if let Some(a) = action {
                match a {
                    Action::Copy(id) => {
                        if let Some(entry) = self.store.entries.iter().find(|e| e.id == id).cloned() {
                            self.copy_entry(&entry);
                            self.store.push(entry.content);
                            self.hide(ctx);
                        }
                    }
                    Action::Pin(id) => { self.store.toggle_pin(id); }
                    Action::Delete(id) => {
                        self.img_cache.remove(&id);
                        self.store.remove(id);
                    }
                }
            }
        });

        self.just_shown = false;
    }
}

enum Action { Copy(u64), Pin(u64), Delete(u64) }

fn render_entry(
    ui: &mut egui::Ui,
    entry: &ClipEntry,
    ctx: &egui::Context,
    img_cache: &mut HashMap<u64, TextureHandle>,
) -> Option<Action> {
    let mut action = None;

    let bg = if entry.pinned { Color32::from_rgb(40, 40, 20) } else { Color32::from_rgb(28, 28, 28) };
    egui::Frame::new()
        .fill(bg)
        .corner_radius(6.0)
        .inner_margin(egui::Margin::same(8))
        .show(ui, |ui| {
            ui.horizontal(|ui| {
                // Time
                ui.label(
                    RichText::new(entry.time.format("%H:%M").to_string())
                        .small().color(Color32::DARK_GRAY),
                );
                // Stats (char count / image size)
                ui.label(
                    RichText::new(&entry.stats).small().color(Color32::DARK_GRAY),
                );
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if ui.small_button(RichText::new("✕").color(Color32::DARK_GRAY)).clicked() {
                        action = Some(Action::Delete(entry.id));
                    }
                    let pin_col = if entry.pinned { Color32::GOLD } else { Color32::DARK_GRAY };
                    if ui.small_button(RichText::new("📌").color(pin_col)).clicked() {
                        action = Some(Action::Pin(entry.id));
                    }
                });
            });

            match &entry.content {
                ClipContent::Text(text) => {
                    let resp = ui.add(
                        egui::Label::new(RichText::new(&entry.preview).monospace().size(13.0))
                            .sense(egui::Sense::click())
                            .truncate(),
                    );
                    if resp.clicked() { action = Some(Action::Copy(entry.id)); }
                    let resp = resp.on_hover_ui(|ui| {
                        ui.set_max_width(420.0);
                        let preview: String = text.chars().take(2000).collect();
                        let suffix = if text.chars().count() > 2000 { "\n…" } else { "" };
                        ui.label(RichText::new(format!("{}{}", preview.trim(), suffix)).monospace().size(12.0));
                    });
                    resp.context_menu(|ui| {
                        if ui.button("📋  复制").clicked() {
                            action = Some(Action::Copy(entry.id));
                            ui.close_menu();
                        }
                        if ui.button("📌  固定 / 取消固定").clicked() {
                            action = Some(Action::Pin(entry.id));
                            ui.close_menu();
                        }
                        ui.separator();
                        if ui.button("🗑  删除").clicked() {
                            action = Some(Action::Delete(entry.id));
                            ui.close_menu();
                        }
                    });
                }
                ClipContent::Image { width, height, rgba } => {
                    let tex = img_cache.entry(entry.id).or_insert_with(|| {
                        let img = egui::ColorImage::from_rgba_unmultiplied(
                            [*width as usize, *height as usize], rgba,
                        );
                        ctx.load_texture(format!("img_{}", entry.id), img, Default::default())
                    });
                    let scale = (260.0_f32 / *width as f32).min(1.0);
                    let size = Vec2::new(*width as f32 * scale, *height as f32 * scale);
                    let resp = ui.add(egui::Image::new(&*tex).fit_to_exact_size(size).sense(egui::Sense::click()));
                    if resp.clicked() { action = Some(Action::Copy(entry.id)); }
                }
            }
        });

    ui.add_space(4.0);
    action
}
