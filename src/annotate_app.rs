use eframe::egui;
use egui::{Color32, ColorImage, Key, Pos2, Rect, Stroke, TextureHandle, Vec2};

#[derive(Clone, Copy, PartialEq)]
pub enum ShapeKind { Rect, Ellipse, Arrow, Pen }

#[derive(Clone)]
pub struct Annotation {
    pub kind: ShapeKind, pub color: Color32, pub width: f32, pub filled: bool,
    pub p1: Pos2, pub p2: Pos2, pub pen_points: Vec<Pos2>,
}

#[derive(Clone, serde::Serialize, serde::Deserialize)]
pub struct HotkeyConfig {
    pub ctrl: bool, pub shift: bool, pub alt: bool, pub key: String,
}
impl Default for HotkeyConfig {
    fn default() -> Self { Self { ctrl: true, shift: true, alt: false, key: "S".to_string() } }
}
impl HotkeyConfig {
    pub fn label(&self) -> String {
        let mut p = Vec::new();
        if self.ctrl  { p.push("Ctrl"); }
        if self.shift { p.push("Shift"); }
        if self.alt   { p.push("Alt"); }
        p.push(&self.key); p.join("+")
    }
    fn matches(&self, ctx: &egui::Context) -> bool {
        ctx.input(|i| {
            let m = i.modifiers;
            if m.ctrl != self.ctrl || m.shift != self.shift || m.alt != self.alt { return false; }
            let key = match self.key.to_uppercase().as_str() {
                "A"=>Key::A,"B"=>Key::B,"C"=>Key::C,"D"=>Key::D,"E"=>Key::E,"F"=>Key::F,
                "G"=>Key::G,"H"=>Key::H,"I"=>Key::I,"J"=>Key::J,"K"=>Key::K,"L"=>Key::L,
                "M"=>Key::M,"N"=>Key::N,"O"=>Key::O,"P"=>Key::P,"Q"=>Key::Q,"R"=>Key::R,
                "S"=>Key::S,"T"=>Key::T,"U"=>Key::U,"V"=>Key::V,"W"=>Key::W,"X"=>Key::X,
                "Y"=>Key::Y,"Z"=>Key::Z,
                "F1"=>Key::F1,"F2"=>Key::F2,"F3"=>Key::F3,"F4"=>Key::F4,
                "F5"=>Key::F5,"F6"=>Key::F6,"F7"=>Key::F7,"F8"=>Key::F8,
                "F9"=>Key::F9,"F10"=>Key::F10,"F11"=>Key::F11,"F12"=>Key::F12,
                _ => return false,
            };
            i.key_pressed(key)
        })
    }
    fn load() -> Self {
        hotkey_path().and_then(|p| std::fs::read_to_string(p).ok())
            .and_then(|s| serde_json::from_str(&s).ok()).unwrap_or_default()
    }
    fn save(&self) {
        if let Some(p) = hotkey_path() {
            let _ = std::fs::create_dir_all(p.parent().unwrap());
            if let Ok(s) = serde_json::to_string(self) { let _ = std::fs::write(p, s); }
        }
    }
}
fn hotkey_path() -> Option<std::path::PathBuf> {
    Some(dirs::data_local_dir()?.join("clip-vault").join("hotkey.json"))
}

const PALETTE: &[Color32] = &[
    Color32::from_rgb(239,68,68), Color32::from_rgb(249,115,22),
    Color32::from_rgb(234,179,8), Color32::from_rgb(34,197,94),
    Color32::from_rgb(59,130,246), Color32::from_rgb(168,85,247),
    Color32::from_rgb(236,72,153), Color32::WHITE, Color32::BLACK,
];

#[derive(PartialEq)]
enum CaptureState { Idle, Selecting, Editing }

pub struct AnnotateApp {
    capture_state: CaptureState,
    full_pixels:   Option<Vec<u8>>,
    full_w: usize, full_h: usize,
    full_texture:  Option<TextureHandle>,
    sel_start: Option<Pos2>, sel_cur: Option<Pos2>,

    pixels: Option<Vec<u8>>,
    img_w: usize, img_h: usize,
    baked: Option<Vec<u8>>,
    texture: Option<TextureHandle>,
    texture_dirty: bool,

    annotations: Vec<Annotation>,
    undo_stack:  Vec<Vec<Annotation>>,

    tool: ShapeKind, color: Color32, stroke_w: f32, filled: bool,
    drag_start: Option<Pos2>, cur_drag: Option<Pos2>, pen_points: Vec<Pos2>,

    pub hotkey:      HotkeyConfig,
    editing_hotkey:  bool,

    smart_rect: Option<[i32; 4]>,
    #[cfg(target_os = "windows")]
    own_hwnd: isize,
    #[cfg(target_os = "windows")]
    mouse_hook: Option<crate::win_capture::LowLevelMouseHook>,

    /// Counts down frames after window hide before grabbing screen
    pub capture_pending: u8,
    /// Set when the capture button is clicked in the settings panel
    pub capture_btn_clicked: bool,
    status: String,
}

impl Default for AnnotateApp {
    fn default() -> Self {
        Self {
            capture_state: CaptureState::Idle,
            full_pixels: None, full_w: 0, full_h: 0, full_texture: None,
            sel_start: None, sel_cur: None,
            pixels: None, img_w: 0, img_h: 0,
            baked: None, texture: None, texture_dirty: false,
            annotations: Vec::new(), undo_stack: Vec::new(),
            tool: ShapeKind::Rect, color: PALETTE[0], stroke_w: 3.0, filled: false,
            drag_start: None, cur_drag: None, pen_points: Vec::new(),
            hotkey: HotkeyConfig::load(), editing_hotkey: false,
            smart_rect: None,
            #[cfg(target_os = "windows")] own_hwnd: 0,
            #[cfg(target_os = "windows")] mouse_hook: None,
            capture_pending: 0,
            capture_btn_clicked: false,
            status: String::new(),
        }
    }
}

impl AnnotateApp {
    pub fn is_selecting(&self) -> bool { self.capture_state == CaptureState::Selecting }

    pub fn grab_fullscreen(&mut self) {
        match screenshots::Screen::all() {
            Ok(screens) => {
                if let Some(screen) = screens.into_iter().next() {
                    match screen.capture() {
                        Ok(img) => {
                            self.full_w = img.width() as usize;
                            self.full_h = img.height() as usize;
                            self.full_pixels = Some(img.into_raw());
                            self.full_texture = None;
                            self.sel_start = None; self.sel_cur = None; self.smart_rect = None;
                            self.capture_state = CaptureState::Selecting;
                            self.status = "拖拽手动框选  或  悬停窗口后单击智能框选  •  Esc 取消".to_string();
                            #[cfg(target_os = "windows")] {
                                self.own_hwnd = crate::win_capture::get_own_hwnd();
                                self.mouse_hook = crate::win_capture::LowLevelMouseHook::install();
                            }
                        }
                        Err(e) => { self.status = format!("截图失败: {e}"); }
                    }
                } else { self.status = "未找到显示器".to_string(); }
            }
            Err(e) => { self.status = format!("截图失败: {e}"); }
        }
    }

    fn commit_selection(&mut self, ctx: &egui::Context) {
        #[cfg(target_os = "windows")] { self.mouse_hook = None; }

        let (x0, y0, x1, y1): (i32, i32, i32, i32);
        if let Some(sr) = self.smart_rect.take() {
            x0 = sr[0]; y0 = sr[1]; x1 = sr[2]; y1 = sr[3];
        } else {
            let (s, e) = match (self.sel_start, self.sel_cur) { (Some(s),Some(e)) => (s,e), _ => return };
            let sc = ctx.pixels_per_point();
            x0 = (s.x.min(e.x) * sc) as i32; y0 = (s.y.min(e.y) * sc) as i32;
            x1 = ((s.x.max(e.x) * sc) as i32).min(self.full_w as i32);
            y1 = ((s.y.max(e.y) * sc) as i32).min(self.full_h as i32);
        }
        let cw = (x1 - x0).max(0) as usize;
        let ch = (y1 - y0).max(0) as usize;
        if cw < 4 || ch < 4 { return; }

        #[cfg(target_os = "windows")]
        let cropped = crate::win_capture::capture_rect_gdi(x0, y0, cw as i32, ch as i32);
        #[cfg(not(target_os = "windows"))]
        let cropped: Option<Vec<u8>> = None;

        let cropped = cropped.unwrap_or_else(|| {
            let base = match self.full_pixels.as_ref() { Some(p) => p, None => return vec![] };
            let mut out = vec![0u8; cw * ch * 4];
            for row in 0..ch {
                let sy = (y0 as usize + row).min(self.full_h.saturating_sub(1));
                let src = (sy * self.full_w + x0 as usize) * 4;
                let dst = row * cw * 4;
                let len = (cw * 4).min(base.len().saturating_sub(src));
                out[dst..dst+len].copy_from_slice(&base[src..src+len]);
            }
            out
        });
        if cropped.is_empty() { return; }

        self.img_w = cw; self.img_h = ch;
        self.pixels = Some(cropped.clone()); self.baked = Some(cropped);
        self.annotations.clear(); self.undo_stack.clear();
        self.texture = None; self.texture_dirty = true;
        self.capture_state = CaptureState::Editing;
        self.status = format!("{}×{} — 选择工具开始标注", cw, ch);
        self.full_pixels = None; self.full_texture = None;
    }

    fn undo(&mut self) {
        if let Some(prev) = self.undo_stack.pop() {
            self.annotations = prev; self.rebuild_baked(); self.texture_dirty = true;
        }
    }
    fn rebuild_baked(&mut self) {
        if let Some(ref base) = self.pixels {
            let mut buf = base.clone();
            let (w, h) = (self.img_w, self.img_h);
            for ann in &self.annotations { render_annotation_to_buf(&mut buf, w, h, ann); }
            self.baked = Some(buf);
        }
    }
    fn bake_last(&mut self) {
        if let (Some(ref mut baked), Some(ann)) = (&mut self.baked, self.annotations.last()) {
            let (w, h) = (self.img_w, self.img_h);
            render_annotation_to_buf(baked, w, h, ann);
        }
    }
    pub fn save_png(&self, path: &std::path::Path) -> anyhow::Result<()> {
        let rgba = self.baked.as_ref().ok_or_else(|| anyhow::anyhow!("no image"))?;
        image::save_buffer(path, rgba, self.img_w as u32, self.img_h as u32, image::ColorType::Rgba8)?;
        Ok(())
    }
    pub fn copy_to_clipboard(&self) -> anyhow::Result<()> {
        let rgba = self.baked.as_ref().ok_or_else(|| anyhow::anyhow!("no image"))?.clone();
        arboard::Clipboard::new()?.set_image(arboard::ImageData {
            width: self.img_w, height: self.img_h, bytes: std::borrow::Cow::Owned(rgba),
        })?;
        Ok(())
    }

    /// Returns true when the window should be hidden to start capture.
    pub fn update(&mut self, ctx: &egui::Context) -> bool {
        // ── Pending: count down frames after window hide ──────────────────────
        if self.capture_pending > 0 {
            self.capture_pending -= 1;
            if self.capture_pending == 0 { self.grab_fullscreen(); }
            ctx.request_repaint();
            return false;
        }

        // ── Hotkey ────────────────────────────────────────────────────────────
        if self.capture_state == CaptureState::Idle && !self.editing_hotkey {
            if self.hotkey.matches(ctx) { return true; }
        }

        // ── Selecting overlay ─────────────────────────────────────────────────
        if self.capture_state == CaptureState::Selecting {
            if ctx.input(|i| i.key_pressed(Key::Escape)) {
                #[cfg(target_os = "windows")] { self.mouse_hook = None; }
                self.capture_state = CaptureState::Idle;
                self.full_pixels = None; self.full_texture = None; self.smart_rect = None;
                self.status = String::new();
                ctx.send_viewport_cmd(egui::ViewportCommand::Fullscreen(false));
                return false;
            }

            // Upload full-screen texture once
            if self.full_texture.is_none() {
                if let Some(ref px) = self.full_pixels {
                    let ci = ColorImage::from_rgba_unmultiplied([self.full_w, self.full_h], px);
                    self.full_texture = Some(ctx.load_texture("fullscreen", ci, egui::TextureOptions::NEAREST));
                }
            }

            // Poll hook
            #[cfg(target_os = "windows")]
            let (hook_pos, hook_clicked) = {
                let st = crate::win_capture::LowLevelMouseHook::poll();
                (Some(st.pos), st.clicked)
            };
            #[cfg(not(target_os = "windows"))]
            let (hook_pos, hook_clicked): (Option<(i32,i32)>, bool) = (None, false);

            // Smart window detection
            if self.sel_start.is_none() {
                if let Some((hx, hy)) = hook_pos {
                    #[cfg(target_os = "windows")]
                    { self.smart_rect = crate::win_capture::hovered_window_rect(hx, hy, self.own_hwnd); }
                }
            }

            // Smart click
            if hook_clicked && self.sel_start.is_none() && self.smart_rect.is_some() {
                #[cfg(target_os = "windows")]
                crate::win_capture::LowLevelMouseHook::consume_click();
                ctx.send_viewport_cmd(egui::ViewportCommand::Fullscreen(false));
                self.commit_selection(ctx);
                return false;
            }

            egui::CentralPanel::default().frame(egui::Frame::NONE).show(ctx, |ui| {
                let avail = ui.available_size();
                let (overlay_rect, resp) = ui.allocate_exact_size(avail, egui::Sense::drag());
                let painter = ui.painter_at(overlay_rect);

                if let Some(ref tex) = self.full_texture {
                    painter.image(tex.id(), overlay_rect,
                        Rect::from_min_max(Pos2::ZERO, Pos2::new(1.0,1.0)), Color32::WHITE);
                }
                painter.rect_filled(overlay_rect, 0.0, Color32::from_black_alpha(80));

                let scale = ctx.pixels_per_point();

                // Smart highlight
                if self.sel_start.is_none() {
                    if let Some(sr) = self.smart_rect {
                        let r = Rect::from_min_max(
                            Pos2::new(sr[0] as f32 / scale, sr[1] as f32 / scale),
                            Pos2::new(sr[2] as f32 / scale, sr[3] as f32 / scale),
                        );
                        if let Some(ref tex) = self.full_texture {
                            let uv = Rect::from_min_max(
                                Pos2::new((r.min.x-overlay_rect.min.x)/overlay_rect.width(),
                                          (r.min.y-overlay_rect.min.y)/overlay_rect.height()),
                                Pos2::new((r.max.x-overlay_rect.min.x)/overlay_rect.width(),
                                          (r.max.y-overlay_rect.min.y)/overlay_rect.height()),
                            );
                            painter.image(tex.id(), r, uv, Color32::WHITE);
                        }
                        painter.rect_stroke(r, 0.0,
                            Stroke::new(2.0, Color32::from_rgb(56,189,248)), egui::StrokeKind::Outside);
                        painter.text(r.max + Vec2::new(4.0,-14.0), egui::Align2::LEFT_TOP,
                            format!("{}×{}  单击确认", sr[2]-sr[0], sr[3]-sr[1]),
                            egui::FontId::proportional(12.0), Color32::from_rgb(56,189,248));
                    }
                }

                // Manual drag
                if resp.drag_started() {
                    self.sel_start = resp.interact_pointer_pos();
                    self.sel_cur   = self.sel_start;
                    self.smart_rect = None;
                }
                if resp.dragged() { self.sel_cur = resp.interact_pointer_pos(); ctx.request_repaint(); }
                if resp.drag_stopped() {
                    self.sel_cur = resp.interact_pointer_pos();
                    ctx.send_viewport_cmd(egui::ViewportCommand::Fullscreen(false));
                    self.commit_selection(ctx);
                    return;
                }

                // Draw selection rect
                if let (Some(s), Some(e)) = (self.sel_start, self.sel_cur) {
                    let sr = Rect::from_two_pos(s, e);
                    if let Some(ref tex) = self.full_texture {
                        let uv = Rect::from_min_max(
                            Pos2::new((sr.min.x-overlay_rect.min.x)/overlay_rect.width(),
                                      (sr.min.y-overlay_rect.min.y)/overlay_rect.height()),
                            Pos2::new((sr.max.x-overlay_rect.min.x)/overlay_rect.width(),
                                      (sr.max.y-overlay_rect.min.y)/overlay_rect.height()),
                        );
                        painter.image(tex.id(), sr, uv, Color32::WHITE);
                    }
                    painter.rect_stroke(sr, 0.0, Stroke::new(2.0, Color32::WHITE), egui::StrokeKind::Outside);
                    let pw = ((sr.width() * scale) as u32).max(0);
                    let ph = ((sr.height() * scale) as u32).max(0);
                    painter.text(sr.max + Vec2::new(4.0,-14.0), egui::Align2::LEFT_TOP,
                        format!("{}×{}", pw, ph), egui::FontId::proportional(12.0), Color32::WHITE);
                }

                if self.sel_start.is_none() && self.smart_rect.is_none() {
                    painter.text(overlay_rect.center(), egui::Align2::CENTER_CENTER,
                        "拖拽手动框选  或  悬停到窗口上单击智能框选  •  Esc 取消",
                        egui::FontId::proportional(16.0), Color32::WHITE);
                }
                ctx.request_repaint();
            });
            return false;
        }

        // ── Settings panel ────────────────────────────────────────────────────
        egui::TopBottomPanel::top("ann_settings")
            .frame(egui::Frame::side_top_panel(&ctx.style())
                .inner_margin(egui::Margin { left: 10, right: 10, top: 6, bottom: 6 }))
            .show(ctx, |ui| {
            ui.horizontal(|ui| {
                if ui.add(egui::Button::new("📷 截图")
                    .fill(Color32::from_rgb(6,78,59)).min_size(egui::vec2(72.0,26.0))
                ).on_hover_text(format!("快捷键: {}", self.hotkey.label())).clicked() {
                    self.capture_btn_clicked = true;
                }
                ui.add(egui::Separator::default().vertical().spacing(4.0));
                ui.label(egui::RichText::new("快捷键:").color(Color32::from_rgb(140,155,175)).size(11.5));
                if self.editing_hotkey {
                    ui.checkbox(&mut self.hotkey.ctrl, "Ctrl");
                    ui.checkbox(&mut self.hotkey.shift, "Shift");
                    ui.checkbox(&mut self.hotkey.alt, "Alt");
                    ui.add(egui::TextEdit::singleline(&mut self.hotkey.key).desired_width(36.0).hint_text("S"));
                    if ui.button("✓ 保存").clicked() { self.hotkey.save(); self.editing_hotkey = false; }
                    if ui.button("✕").clicked() { self.editing_hotkey = false; }
                } else {
                    ui.label(egui::RichText::new(self.hotkey.label()).color(Color32::from_rgb(56,189,248)).size(11.5));
                    if ui.small_button("✎").on_hover_text("自定义快捷键").clicked() { self.editing_hotkey = true; }
                }
                if !self.status.is_empty() {
                    ui.add(egui::Separator::default().vertical().spacing(4.0));
                    ui.label(egui::RichText::new(&self.status).color(Color32::from_rgb(148,163,184)).size(11.5));
                }
            });
        });

        // Check capture button flag
        if self.capture_btn_clicked {
            self.capture_btn_clicked = false;
            return true;
        }
        false
    }

    pub fn editing_panel(&mut self, ctx: &egui::Context) {
        if self.capture_state != CaptureState::Editing { return; }
        if self.texture_dirty {
            if let Some(ref rgba) = self.baked {
                let ci = ColorImage::from_rgba_unmultiplied([self.img_w, self.img_h], rgba);
                self.texture = Some(ctx.load_texture("screenshot", ci, egui::TextureOptions::NEAREST));
            }
            self.texture_dirty = false;
        }
        egui::CentralPanel::default().show(ctx, |ui| {
            egui::ScrollArea::both().auto_shrink(false).show(ui, |ui| {
                let img_size = Vec2::new(self.img_w as f32, self.img_h as f32);
                if let Some(ref tex) = self.texture {
                    let (cr, resp) = ui.allocate_exact_size(img_size, egui::Sense::drag());
                    let painter = ui.painter_at(cr);
                    painter.image(tex.id(), cr, Rect::from_min_max(Pos2::ZERO, Pos2::new(1.0,1.0)), Color32::WHITE);
                    let to_img    = |p: Pos2| Pos2::new((p.x-cr.min.x).clamp(0.0,img_size.x),(p.y-cr.min.y).clamp(0.0,img_size.y));
                    let to_screen = |p: Pos2| Pos2::new(cr.min.x+p.x, cr.min.y+p.y);
                    for ann in &self.annotations { draw_annotation_egui(&painter, ann, 1.0, &to_screen); }
                    if resp.drag_started() {
                        if let Some(pos) = resp.interact_pointer_pos() {
                            if self.tool == ShapeKind::Pen {
                                self.undo_stack.push(self.annotations.clone());
                                self.pen_points.clear(); self.pen_points.push(to_img(pos));
                            } else { self.drag_start = Some(to_img(pos)); }
                        }
                    }
                    if resp.dragged() {
                        if let Some(pos) = resp.interact_pointer_pos() {
                            if self.tool == ShapeKind::Pen { self.pen_points.push(to_img(pos)); }
                            else { self.cur_drag = Some(to_img(pos)); }
                            ctx.request_repaint();
                        }
                    }
                    if resp.drag_stopped() {
                        if let Some(pos) = resp.interact_pointer_pos() {
                            if self.tool == ShapeKind::Pen {
                                self.pen_points.push(to_img(pos));
                                if self.pen_points.len() >= 2 {
                                    self.annotations.push(Annotation {
                                        kind: ShapeKind::Pen, color: self.color, width: self.stroke_w, filled: false,
                                        p1: *self.pen_points.first().unwrap(), p2: *self.pen_points.last().unwrap(),
                                        pen_points: self.pen_points.clone(),
                                    });
                                    self.bake_last(); self.texture_dirty = true;
                                }
                                self.pen_points.clear();
                            } else if let Some(start) = self.drag_start.take() {
                                let end = to_img(pos);
                                if (end - start).length() > 2.0 {
                                    self.undo_stack.push(self.annotations.clone());
                                    self.annotations.push(Annotation {
                                        kind: self.tool, color: self.color, width: self.stroke_w, filled: self.filled,
                                        p1: start, p2: end, pen_points: Vec::new(),
                                    });
                                    self.bake_last(); self.texture_dirty = true;
                                }
                                self.cur_drag = None;
                            }
                        }
                    }
                    if self.tool == ShapeKind::Pen && self.pen_points.len() >= 2 {
                        for pair in self.pen_points.windows(2) {
                            painter.line_segment([to_screen(pair[0]),to_screen(pair[1])], Stroke::new(self.stroke_w, self.color));
                        }
                    } else if let (Some(s), Some(e)) = (self.drag_start, self.cur_drag) {
                        draw_annotation_egui(&painter, &Annotation { kind: self.tool, color: self.color,
                            width: self.stroke_w, filled: self.filled, p1: s, p2: e, pen_points: Vec::new() }, 1.0, &to_screen);
                    }
                }
                // Inline toolbar below image
                ui.add_space(6.0);
                egui::Frame::new().fill(Color32::from_rgba_unmultiplied(20,22,30,230))
                    .corner_radius(8.0).inner_margin(egui::Margin { left:8,right:8,top:6,bottom:6 })
                    .show(ui, |ui| {
                    ui.horizontal_wrapped(|ui| {
                        for (kind, label, tip) in [(ShapeKind::Rect,"▭","矩形"),(ShapeKind::Ellipse,"◯","椭圆"),
                                                   (ShapeKind::Arrow,"→","箭头"),(ShapeKind::Pen,"✏","画笔")] {
                            let sel = self.tool == kind;
                            if ui.add(egui::Button::new(label)
                                .fill(if sel {Color32::from_rgb(6,78,59)} else {Color32::from_rgb(38,42,54)})
                                .min_size(egui::vec2(28.0,26.0))).on_hover_text(tip).clicked() { self.tool = kind; }
                        }
                        ui.add(egui::Separator::default().vertical().spacing(4.0));
                        ui.checkbox(&mut self.filled, "填充");
                        ui.add(egui::Slider::new(&mut self.stroke_w, 1.0..=20.0).show_value(false));
                        ui.label(egui::RichText::new(format!("{:.0}px", self.stroke_w)).size(11.0));
                        ui.add(egui::Separator::default().vertical().spacing(4.0));
                        for &c in PALETTE {
                            let sel = self.color == c;
                            let (rect, resp) = ui.allocate_exact_size(egui::vec2(20.0,20.0), egui::Sense::click());
                            ui.painter().rect_filled(rect, 3.0, c);
                            if sel { ui.painter().rect_stroke(rect, 3.0, Stroke::new(2.0,Color32::WHITE), egui::StrokeKind::Outside); }
                            if resp.clicked() { self.color = c; }
                        }
                        ui.color_edit_button_srgba(&mut self.color);
                        ui.add(egui::Separator::default().vertical().spacing(4.0));
                        let has = !self.annotations.is_empty();
                        if ui.add_enabled(has, egui::Button::new("↩").min_size(egui::vec2(26.0,26.0))).on_hover_text("撤销").clicked() { self.undo(); }
                        if ui.add_enabled(has, egui::Button::new("🗑").fill(Color32::from_rgb(100,20,20)).min_size(egui::vec2(26.0,26.0))).on_hover_text("清空").clicked() {
                            self.undo_stack.push(self.annotations.clone()); self.annotations.clear();
                            self.baked = self.pixels.clone(); self.texture_dirty = true;
                        }
                        ui.add(egui::Separator::default().vertical().spacing(4.0));
                        if ui.add(egui::Button::new("📋 复制").min_size(egui::vec2(56.0,26.0))).on_hover_text("复制到剪贴板").clicked() {
                            match self.copy_to_clipboard() {
                                Ok(_) => self.status = "已复制到剪贴板".to_string(),
                                Err(e) => self.status = format!("复制失败: {e}"),
                            }
                        }
                        if ui.add(egui::Button::new("💾 保存").min_size(egui::vec2(56.0,26.0))).clicked() {
                            if let Some(path) = rfd::FileDialog::new().add_filter("PNG",&["png"]).set_file_name("screenshot.png").save_file() {
                                match self.save_png(&path) {
                                    Ok(_) => self.status = format!("已保存: {}", path.display()),
                                    Err(e) => self.status = format!("保存失败: {e}"),
                                }
                            }
                        }
                        if ui.add(egui::Button::new("✕ 关闭").min_size(egui::vec2(56.0,26.0))).clicked() {
                            self.capture_state = CaptureState::Idle;
                            self.pixels = None; self.baked = None; self.texture = None;
                            self.annotations.clear(); self.undo_stack.clear(); self.status = String::new();
                        }
                    });
                });
            });
        });
    }
} // end impl AnnotateApp

fn draw_annotation_egui(painter: &egui::Painter, ann: &Annotation, scale: f32, to_screen: &impl Fn(Pos2)->Pos2) {
    let stroke = Stroke::new(ann.width * scale, ann.color);
    match ann.kind {
        ShapeKind::Rect => {
            let r = Rect::from_two_pos(to_screen(ann.p1), to_screen(ann.p2));
            if ann.filled { painter.rect_filled(r, 0.0, ann.color); }
            else { painter.rect_stroke(r, 0.0, stroke, egui::StrokeKind::Middle); }
        }
        ShapeKind::Ellipse => {
            let center = to_screen(Pos2::new((ann.p1.x+ann.p2.x)/2.0,(ann.p1.y+ann.p2.y)/2.0));
            let radii = Vec2::new((ann.p2.x-ann.p1.x).abs()/2.0*scale,(ann.p2.y-ann.p1.y).abs()/2.0*scale);
            let pts: Vec<Pos2> = (0..=64).map(|i| { let t=i as f32/64.0*std::f32::consts::TAU; center+Vec2::new(radii.x*t.cos(),radii.y*t.sin()) }).collect();
            if ann.filled { painter.add(egui::Shape::convex_polygon(pts, ann.color, Stroke::NONE)); }
            else { painter.add(egui::Shape::closed_line(pts, stroke)); }
        }
        ShapeKind::Arrow => { let (s,e)=(to_screen(ann.p1),to_screen(ann.p2)); painter.arrow(s, e-s, stroke); }
        ShapeKind::Pen => { for pair in ann.pen_points.windows(2) { painter.line_segment([to_screen(pair[0]),to_screen(pair[1])], stroke); } }
    }
}

pub fn render_annotation_to_buf(buf: &mut [u8], w: usize, h: usize, ann: &Annotation) {
    match ann.kind {
        ShapeKind::Rect => {
            let (x0,y0,x1,y1) = (ann.p1.x.min(ann.p2.x) as i32, ann.p1.y.min(ann.p2.y) as i32,
                                  ann.p1.x.max(ann.p2.x) as i32, ann.p1.y.max(ann.p2.y) as i32);
            let lw = ann.width as i32;
            if ann.filled { fill_rect(buf,w,h,x0,y0,x1,y1,ann.color); }
            else { for t in 0..lw { draw_rect_outline(buf,w,h,x0+t,y0+t,x1-t,y1-t,ann.color); } }
        }
        ShapeKind::Ellipse => {
            let (cx,cy) = (((ann.p1.x+ann.p2.x)/2.0) as i32, ((ann.p1.y+ann.p2.y)/2.0) as i32);
            let (rx,ry) = (((ann.p2.x-ann.p1.x).abs()/2.0) as i32, ((ann.p2.y-ann.p1.y).abs()/2.0) as i32);
            draw_ellipse(buf,w,h,cx,cy,rx,ry,ann.color,ann.width as i32,ann.filled);
        }
        ShapeKind::Arrow => { draw_arrow(buf,w,h,ann.p1,ann.p2,ann.color,ann.width as i32); }
        ShapeKind::Pen => { for pair in ann.pen_points.windows(2) { draw_line_thick(buf,w,h,pair[0],pair[1],ann.color,ann.width as i32); } }
    }
}

#[inline]
fn set_pixel(buf: &mut [u8], w: usize, h: usize, x: i32, y: i32, c: Color32) {
    if x<0||y<0||x>=w as i32||y>=h as i32 { return; }
    let i=(y as usize*w+x as usize)*4;
    buf[i]=c.r(); buf[i+1]=c.g(); buf[i+2]=c.b(); buf[i+3]=255;
}
fn draw_rect_outline(buf:&mut[u8],w:usize,h:usize,x0:i32,y0:i32,x1:i32,y1:i32,c:Color32) {
    for x in x0..=x1 { set_pixel(buf,w,h,x,y0,c); set_pixel(buf,w,h,x,y1,c); }
    for y in y0..=y1 { set_pixel(buf,w,h,x0,y,c); set_pixel(buf,w,h,x1,y,c); }
}
fn fill_rect(buf:&mut[u8],w:usize,h:usize,x0:i32,y0:i32,x1:i32,y1:i32,c:Color32) {
    for y in y0..=y1 { for x in x0..=x1 { set_pixel(buf,w,h,x,y,c); } }
}
fn draw_ellipse(buf:&mut[u8],w:usize,h:usize,cx:i32,cy:i32,rx:i32,ry:i32,c:Color32,lw:i32,filled:bool) {
    if rx<=0||ry<=0 { return; }
    let steps=(2.0*std::f32::consts::PI*rx.max(ry) as f32) as usize*2;
    let mut prev:Option<(i32,i32)>=None;
    for i in 0..=steps {
        let t=i as f32/steps as f32*2.0*std::f32::consts::PI;
        let (px,py)=(cx+(rx as f32*t.cos()) as i32, cy+(ry as f32*t.sin()) as i32);
        if !filled { if let Some((ppx,ppy))=prev { draw_line_thick_i(buf,w,h,ppx,ppy,px,py,c,lw); } }
        prev=Some((px,py));
    }
    if filled {
        for y in (cy-ry)..=(cy+ry) {
            let dy=(y-cy) as f32/ry as f32; if dy.abs()>1.0 { continue; }
            let dx=(1.0-dy*dy).sqrt()*rx as f32;
            for x in (cx-dx as i32)..=(cx+dx as i32) { set_pixel(buf,w,h,x,y,c); }
        }
    }
}
fn draw_line_thick(buf:&mut[u8],w:usize,h:usize,p1:Pos2,p2:Pos2,c:Color32,lw:i32) {
    draw_line_thick_i(buf,w,h,p1.x as i32,p1.y as i32,p2.x as i32,p2.y as i32,c,lw);
}
fn draw_line_thick_i(buf:&mut[u8],w:usize,h:usize,x0:i32,y0:i32,x1:i32,y1:i32,c:Color32,lw:i32) {
    if x0==x1&&y0==y1 { let half=lw/2; for ox in -half..=half { for oy in -half..=half { set_pixel(buf,w,h,x0+ox,y0+oy,c); } } return; }
    let (dx,dy)=((x1-x0).abs(),(y1-y0).abs());
    let (sx,sy)=(if x0<x1{1}else{-1},if y0<y1{1}else{-1});
    let mut err=dx-dy; let (mut x,mut y)=(x0,y0); let half=lw/2;
    loop {
        for ox in -half..=half { for oy in -half..=half { set_pixel(buf,w,h,x+ox,y+oy,c); } }
        if x==x1&&y==y1 { break; }
        let e2=2*err;
        if e2>-dy { err-=dy; x+=sx; }
        if e2<dx  { err+=dx; y+=sy; }
    }
}
fn draw_arrow(buf:&mut[u8],w:usize,h:usize,p1:Pos2,p2:Pos2,c:Color32,lw:i32) {
    draw_line_thick(buf,w,h,p1,p2,c,lw);
    let (dx,dy)=(p2.x-p1.x,p2.y-p1.y);
    let len=(dx*dx+dy*dy).sqrt().max(1.0);
    let (ux,uy)=(dx/len,dy/len);
    let (head,angle)=(18.0_f32,0.4_f32);
    let ax1=Pos2::new(p2.x-head*(ux*angle.cos()+uy*angle.sin()),p2.y-head*(-ux*angle.sin()+uy*angle.cos()));
    let ax2=Pos2::new(p2.x-head*(ux*angle.cos()-uy*angle.sin()),p2.y-head*(ux*angle.sin()+uy*angle.cos()));
    draw_line_thick(buf,w,h,p2,ax1,c,lw);
    draw_line_thick(buf,w,h,p2,ax2,c,lw);
}
