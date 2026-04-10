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
    hide_next_frame: bool,  // delay hide by one frame so clipboard write completes
    query: String,
    query_lc: String,
    last_query_for_lc: String,  // tracks last query used to build query_lc
    clipboard: Clipboard,
    img_cache: HashMap<u64, TextureHandle>,
    pub hotkey_triggered: std::sync::Arc<std::sync::atomic::AtomicBool>,
    paused: bool,
    last_paused: bool,      // previous paused state for change detection
    status_str: String,
    status_count: usize,
    selected_idx: Option<usize>, // keyboard navigation index into filtered list
}

impl App {
    pub fn new(
        rx: Receiver<ClipContent>,
        hotkey_triggered: std::sync::Arc<std::sync::atomic::AtomicBool>,
    ) -> Self {
        let store = Store::load();
        let count = store.entries.len();
        Self {
            store,
            rx,
            visible: true,
            just_shown: true,
            hide_next_frame: false,
            query: String::new(),
            query_lc: String::new(),
            last_query_for_lc: String::new(),
            clipboard: Clipboard::new().expect("clipboard init failed"),
            img_cache: HashMap::new(),
            hotkey_triggered,
            paused: false,
            last_paused: false,
            status_str: Self::make_status(count, false),
            status_count: count,
            selected_idx: None,
        }
    }

    fn make_status(count: usize, paused: bool) -> String {
        if paused {
            format!("{} 条记录  •  已暂停  •  Ctrl+Shift+V 呼出  •  Esc 隐藏", count)
        } else {
            format!("{} 条记录  •  Ctrl+Shift+V 呼出  •  Esc 隐藏", count)
        }
    }

    fn refresh_status(&mut self) {
        let count = self.store.entries.len();
        // Compare fields directly — no string search
        if count != self.status_count || self.paused != self.last_paused {
            self.status_count = count;
            self.last_paused = self.paused;
            self.status_str = Self::make_status(count, self.paused);
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
        self.last_query_for_lc.clear();
        ctx.send_viewport_cmd(egui::ViewportCommand::Visible(false));
    }
}

impl eframe::App for App {
    fn on_exit(&mut self, _gl: Option<&eframe::glow::Context>) {
        // Flush any pending writes before process exits
        self.store.flush_now();
    }

    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // Delayed hide — execute one frame after clipboard write
        if self.hide_next_frame {
            self.hide_next_frame = false;
            self.hide(ctx);
            return;
        }
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

        // Drain new clipboard entries (skip if paused)
        let mut got_new = false;
        while let Ok(content) = self.rx.try_recv() {
            if !self.paused {
                self.store.push(content);
                got_new = true;
            }
        }
        if got_new && self.visible { ctx.request_repaint(); }

        // Flush dirty store to disk (debounced, max once per 2s)
        self.store.flush_if_needed();

        if !self.visible {
            ctx.request_repaint_after(std::time::Duration::from_millis(200));
            return;
        }

        if ctx.input(|i| i.key_pressed(egui::Key::Escape)) {
            self.hide(ctx);
            return;
        }

        // Update query_lc only when query actually changes
        if self.query != self.last_query_for_lc {
            self.last_query_for_lc = self.query.clone();
            self.query_lc = self.query.to_lowercase();
            self.selected_idx = None;
        }

        // Refresh cached status string
        self.refresh_status();

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

                // Clear button — only show when query is non-empty
                if !self.query.is_empty() {
                    if ui.small_button("✕").on_hover_text("清空搜索").clicked() {
                        self.query.clear();
                        self.query_lc.clear();
                        self.last_query_for_lc.clear();
                        self.selected_idx = None;
                        resp.request_focus();
                    }
                }

                // Pause toggle
                let pause_label = if self.paused { "▶" } else { "⏸" };
                if ui.small_button(pause_label)
                    .on_hover_text(if self.paused { "恢复记录" } else { "暂停记录" })
                    .clicked()
                {
                    self.paused = !self.paused;
                }

                if ui.button("🗑").on_hover_text("清除所有未固定条目").clicked() {
                    self.store.clear_unpinned();
                    self.img_cache.clear();
                }
            });
            ui.add_space(4.0);
        });

        egui::TopBottomPanel::bottom("hint").show(ctx, |ui| {
            ui.label(RichText::new(&self.status_str).small().color(
                if self.paused { Color32::from_rgb(200, 150, 50) } else { Color32::GRAY }
            ));
        });

        egui::CentralPanel::default().show(ctx, |ui| {
            // Build filtered list once — pre-allocate capacity
            let cap = self.store.entries.len();
            let mut pinned: Vec<&ClipEntry> = Vec::with_capacity(cap / 4);
            let mut recent: Vec<&ClipEntry> = Vec::with_capacity(cap);
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

            // Keyboard navigation + number key shortcuts
            let total = pinned.len() + recent.len();
            if total > 0 {
                let down  = ctx.input(|i| i.key_pressed(egui::Key::ArrowDown));
                let up    = ctx.input(|i| i.key_pressed(egui::Key::ArrowUp));
                let enter = ctx.input(|i| i.key_pressed(egui::Key::Enter));

                // Number keys 1-9: instantly copy nth visible entry
                let num_pressed = ctx.input(|i| {
                    for (k, n) in [
                        (egui::Key::Num1,1),(egui::Key::Num2,2),(egui::Key::Num3,3),
                        (egui::Key::Num4,4),(egui::Key::Num5,5),(egui::Key::Num6,6),
                        (egui::Key::Num7,7),(egui::Key::Num8,8),(egui::Key::Num9,9),
                    ] {
                        if i.key_pressed(k) { return Some(n); }
                    }
                    None
                });
                if let Some(n) = num_pressed {
                    let idx = n - 1;
                    let entry_ref = if idx < pinned.len() {
                        pinned.get(idx).copied()
                    } else {
                        recent.get(idx - pinned.len()).copied()
                    };
                    if let Some(e) = entry_ref {
                        let id = e.id;
                        if let Some(entry) = self.store.entries.iter().find(|e| e.id == id).cloned() {
                            self.copy_entry(&entry);
                            self.store.push(entry.content);
                            self.selected_idx = None;
                            self.hide_next_frame = true;
                            return;
                        }
                    }
                }

                if down || up {
                    self.selected_idx = Some(match self.selected_idx {
                        None => 0,
                        Some(i) => {
                            if down { (i + 1).min(total - 1) }
                            else { i.saturating_sub(1) }
                        }
                    });
                }
                if enter {
                    if let Some(idx) = self.selected_idx {
                        let entry_ref = if idx < pinned.len() {
                            pinned.get(idx).copied()
                        } else {
                            recent.get(idx - pinned.len()).copied()
                        };
                        if let Some(e) = entry_ref {
                            let id = e.id;
                            if let Some(entry) = self.store.entries.iter().find(|e| e.id == id).cloned() {
                                self.copy_entry(&entry);
                                self.store.push(entry.content);
                                self.selected_idx = None;
                                self.hide_next_frame = true;
                                return;
                            }
                        }
                    }
                }
            } else {
                self.selected_idx = None;
            }

            let mut action: Option<Action> = None;

            ScrollArea::vertical().auto_shrink(false).show(ui, |ui| {
                if !pinned.is_empty() {
                    ui.label(RichText::new("📌 已固定").small().color(Color32::GOLD));
                    for (i, e) in pinned.iter().enumerate() {
                        let selected = self.selected_idx == Some(i);
                        if let Some(a) = render_entry(ui, e, ctx, &mut self.img_cache, selected, i + 1) {
                            action = Some(a);
                        }
                    }
                    ui.separator();
                }

                let today = chrono::Local::now().date_naive();
                let yesterday = today.pred_opt().unwrap_or(today);
                let mut last_group = "";

                for (i, e) in recent.iter().enumerate() {
                    let date = e.time.date_naive();
                    let group = if date == today { "今天" }
                        else if date == yesterday { "昨天" }
                        else { "更早" };
                    if group != last_group {
                        ui.add_space(4.0);
                        ui.label(RichText::new(group).small().color(Color32::DARK_GRAY));
                        last_group = group;
                    }
                    let abs_idx = pinned.len() + i;
                    let selected = self.selected_idx == Some(abs_idx);
                    if let Some(a) = render_entry(ui, e, ctx, &mut self.img_cache, selected, abs_idx + 1) {
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
                            // Window stays open — user can continue browsing
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
    selected: bool,
    seq: usize,  // 1-based sequence number for keyboard shortcut hint
) -> Option<Action> {
    let mut action = None;

    let bg = if selected {
        Color32::from_rgb(50, 70, 100)
    } else if entry.pinned {
        Color32::from_rgb(40, 40, 20)
    } else {
        Color32::from_rgb(28, 28, 28)
    };
    egui::Frame::new()
        .fill(bg)
        .corner_radius(6.0)
        .inner_margin(egui::Margin::same(8))
        .show(ui, |ui| {
            ui.horizontal(|ui| {
                // Show sequence number (1-9) as keyboard shortcut hint
                const SEQ_LABELS: [&str; 9] = ["1","2","3","4","5","6","7","8","9"];
                if seq >= 1 && seq <= 9 {
                    ui.label(RichText::new(SEQ_LABELS[seq - 1]).small().color(
                        if selected { Color32::WHITE } else { Color32::from_rgb(80, 80, 80) }
                    ));
                }
                ui.label(RichText::new(&entry.time_str).small().color(Color32::DARK_GRAY));
                ui.label(RichText::new(&entry.stats).small().color(Color32::DARK_GRAY));
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
                        if entry.char_count <= 2000 {
                            ui.label(RichText::new(text.trim()).monospace().size(12.0));
                        } else {
                            let preview: String = text.chars().take(2000).collect();
                            ui.label(RichText::new(format!("{}…", preview.trim())).monospace().size(12.0));
                        }
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
