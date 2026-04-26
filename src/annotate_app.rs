use eframe::egui;
use egui::{Color32, ColorImage, Key, Pos2, Rect, Stroke, TextureHandle, Vec2};

// ── Pinned image (贴图) ───────────────────────────────────────────────────────

pub struct PinnedImage {
    pixels: Vec<u8>, w: usize, h: usize,
    texture: Option<TextureHandle>,
    open: bool,
    zoom: f32,
    id: egui::ViewportId,
}

impl PinnedImage {
    fn new(pixels: Vec<u8>, w: usize, h: usize) -> Self {
        use std::time::{SystemTime, UNIX_EPOCH};
        let ts = SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default().subsec_nanos();
        Self { pixels, w, h, texture: None, open: true, zoom: 1.0,
            id: egui::ViewportId::from_hash_of(ts) }
    }
}

// ── Shape types ───────────────────────────────────────────────────────────────

#[derive(Clone, Copy, PartialEq, serde::Serialize, serde::Deserialize)]
pub enum ShapeKind {
    Rect,           // 矩形
    RoundRect,      // 圆角矩形
    Ellipse,        // 椭圆
    Arrow,          // 单向箭头
    ArrowDouble,    // 双向箭头
    Line,           // 直线
    Pen,            // 画笔
    Marker,         // 记号笔（半透明）
    Mosaic,         // 马赛克
    Eraser,         // 橡皮擦
    Text,           // 文字
    Number,         // 序号标注
}

impl ShapeKind {
    pub fn label(self) -> &'static str {
        match self {
            Self::Rect       => "▭",
            Self::RoundRect  => "▢",
            Self::Ellipse    => "◯",
            Self::Arrow      => "→",
            Self::ArrowDouble=> "↔",
            Self::Line       => "╱",
            Self::Pen        => "✏",
            Self::Marker     => "🖊",
            Self::Mosaic     => "⊞",
            Self::Eraser     => "⬜",
            Self::Text       => "T",
            Self::Number     => "①",
        }
    }
    pub fn tip(self) -> &'static str {
        match self {
            Self::Rect       => "矩形 (R)",
            Self::RoundRect  => "圆角矩形",
            Self::Ellipse    => "椭圆 (E)",
            Self::Arrow      => "箭头 (A)",
            Self::ArrowDouble=> "双向箭头",
            Self::Line       => "直线",
            Self::Pen        => "画笔 (P)",
            Self::Marker     => "记号笔",
            Self::Mosaic     => "马赛克",
            Self::Eraser     => "橡皮擦",
            Self::Text       => "文字 (T)",
            Self::Number     => "序号",
        }
    }
}

// ── Line style ────────────────────────────────────────────────────────────────

#[derive(Clone, Copy, PartialEq, serde::Serialize, serde::Deserialize)]
pub enum LineStyle { Solid, Dashed, Dotted }

// ── Annotation ────────────────────────────────────────────────────────────────

#[derive(Clone)]
pub struct Annotation {
    pub kind:       ShapeKind,
    pub color:      Color32,
    pub width:      f32,
    pub filled:     bool,
    pub line_style: LineStyle,
    pub p1: Pos2, pub p2: Pos2,
    pub pen_points: Vec<Pos2>,
    /// For Text: the string content
    pub text: String,
    /// For Number: the sequence number
    pub number: u32,
}

impl Annotation {
    fn new(kind: ShapeKind, color: Color32, width: f32, filled: bool,
           line_style: LineStyle, p1: Pos2, p2: Pos2) -> Self {
        Self { kind, color, width, filled, line_style, p1, p2,
               pen_points: Vec::new(), text: String::new(), number: 1 }
    }
}

// ── Per-tool color memory ─────────────────────────────────────────────────────

#[derive(Clone, serde::Serialize, serde::Deserialize)]
pub struct ToolColors {
    pub rect:        [u8;4],
    pub round_rect:  [u8;4],
    pub ellipse:     [u8;4],
    pub arrow:       [u8;4],
    pub arrow_double:[u8;4],
    pub line:        [u8;4],
    pub pen:         [u8;4],
    pub marker:      [u8;4],
    pub mosaic:      [u8;4],
    pub eraser:      [u8;4],
    pub text:        [u8;4],
    pub number:      [u8;4],
}

fn c2a(c: Color32) -> [u8;4] { [c.r(),c.g(),c.b(),c.a()] }
fn a2c(a: [u8;4]) -> Color32 { Color32::from_rgba_unmultiplied(a[0],a[1],a[2],a[3]) }

impl Default for ToolColors {
    fn default() -> Self {
        let red  = c2a(Color32::from_rgb(239,68,68));
        let blue = c2a(Color32::from_rgb(59,130,246));
        let marker = c2a(Color32::from_rgba_unmultiplied(249,115,22,160));
        Self {
            rect: red, round_rect: red, ellipse: red,
            arrow: red, arrow_double: red, line: blue,
            pen: red, marker,
            mosaic: c2a(Color32::GRAY), eraser: c2a(Color32::WHITE),
            text: red, number: blue,
        }
    }
}

impl ToolColors {
    fn get(&self, kind: ShapeKind) -> Color32 {
        a2c(match kind {
            ShapeKind::Rect        => self.rect,
            ShapeKind::RoundRect   => self.round_rect,
            ShapeKind::Ellipse     => self.ellipse,
            ShapeKind::Arrow       => self.arrow,
            ShapeKind::ArrowDouble => self.arrow_double,
            ShapeKind::Line        => self.line,
            ShapeKind::Pen         => self.pen,
            ShapeKind::Marker      => self.marker,
            ShapeKind::Mosaic      => self.mosaic,
            ShapeKind::Eraser      => self.eraser,
            ShapeKind::Text        => self.text,
            ShapeKind::Number      => self.number,
        })
    }
    fn set(&mut self, kind: ShapeKind, c: Color32) {
        let a = c2a(c);
        match kind {
            ShapeKind::Rect        => self.rect = a,
            ShapeKind::RoundRect   => self.round_rect = a,
            ShapeKind::Ellipse     => self.ellipse = a,
            ShapeKind::Arrow       => self.arrow = a,
            ShapeKind::ArrowDouble => self.arrow_double = a,
            ShapeKind::Line        => self.line = a,
            ShapeKind::Pen         => self.pen = a,
            ShapeKind::Marker      => self.marker = a,
            ShapeKind::Mosaic      => self.mosaic = a,
            ShapeKind::Eraser      => self.eraser = a,
            ShapeKind::Text        => self.text = a,
            ShapeKind::Number      => self.number = a,
        }
    }
    fn load() -> Self {
        tool_colors_path()
            .and_then(|p| std::fs::read_to_string(p).ok())
            .and_then(|s| serde_json::from_str(&s).ok())
            .unwrap_or_default()
    }
    fn save(&self) {
        if let Some(p) = tool_colors_path() {
            let _ = std::fs::create_dir_all(p.parent().unwrap());
            if let Ok(s) = serde_json::to_string(self) { let _ = std::fs::write(p, s); }
        }
    }
}
fn tool_colors_path() -> Option<std::path::PathBuf> {
    Some(dirs::data_local_dir()?.join("clip-vault").join("tool-colors.json"))
}

// ── Custom palette ────────────────────────────────────────────────────────────

const DEFAULT_PALETTE: &[Color32] = &[
    Color32::from_rgb(239,68,68),  Color32::from_rgb(249,115,22),
    Color32::from_rgb(234,179,8),  Color32::from_rgb(34,197,94),
    Color32::from_rgb(59,130,246), Color32::from_rgb(168,85,247),
    Color32::from_rgb(236,72,153), Color32::WHITE, Color32::BLACK,
];

#[derive(serde::Serialize, serde::Deserialize)]
struct Palette {
    colors: Vec<[u8; 4]>, // RGBA
}

impl Default for Palette {
    fn default() -> Self {
        Self { colors: DEFAULT_PALETTE.iter().map(|c| [c.r(),c.g(),c.b(),c.a()]).collect() }
    }
}

impl Palette {
    fn load() -> Self {
        palette_path()
            .and_then(|p| std::fs::read_to_string(p).ok())
            .and_then(|s| serde_json::from_str(&s).ok())
            .unwrap_or_default()
    }
    fn save(&self) {
        if let Some(p) = palette_path() {
            let _ = std::fs::create_dir_all(p.parent().unwrap());
            if let Ok(s) = serde_json::to_string(self) { let _ = std::fs::write(p, s); }
        }
    }
    fn colors(&self) -> Vec<Color32> {
        self.colors.iter().map(|c| Color32::from_rgba_unmultiplied(c[0],c[1],c[2],c[3])).collect()
    }
    /// Cached version: only reallocates when palette changes.
    fn colors_cached(&self, cache: &mut Vec<Color32>) {
        if cache.len() != self.colors.len() {
            *cache = self.colors();
        }
    }
}
fn palette_path() -> Option<std::path::PathBuf> {
    Some(dirs::data_local_dir()?.join("clip-vault").join("palette.json"))
}

// ── Hotkey ────────────────────────────────────────────────────────────────────

#[derive(Clone, serde::Serialize, serde::Deserialize)]
pub struct HotkeyConfig {
    pub ctrl: bool, pub shift: bool, pub alt: bool, pub key: String,
}
impl Default for HotkeyConfig {
    fn default() -> Self { Self { ctrl: false, shift: false, alt: false, key: "F1".to_string() } }
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
    pub fn load_static() -> Self { Self::load() }
    fn save(&self) {
        if let Some(p) = hotkey_path() {
            let _ = std::fs::create_dir_all(p.parent().unwrap());
            if let Ok(s) = serde_json::to_string(self) { let _ = std::fs::write(p, s); }
        }
    }
    pub fn save_pub(&self) { self.save(); }
    pub fn to_global_hotkey(&self) -> global_hotkey::hotkey::HotKey {
        use global_hotkey::hotkey::{Code, HotKey, Modifiers};
        let mut mods = Modifiers::empty();
        if self.ctrl  { mods |= Modifiers::CONTROL; }
        if self.shift { mods |= Modifiers::SHIFT; }
        if self.alt   { mods |= Modifiers::ALT; }
        let code = match self.key.to_uppercase().as_str() {
            "A"=>Code::KeyA,"B"=>Code::KeyB,"C"=>Code::KeyC,"D"=>Code::KeyD,
            "E"=>Code::KeyE,"F"=>Code::KeyF,"G"=>Code::KeyG,"H"=>Code::KeyH,
            "I"=>Code::KeyI,"J"=>Code::KeyJ,"K"=>Code::KeyK,"L"=>Code::KeyL,
            "M"=>Code::KeyM,"N"=>Code::KeyN,"O"=>Code::KeyO,"P"=>Code::KeyP,
            "Q"=>Code::KeyQ,"R"=>Code::KeyR,"S"=>Code::KeyS,"T"=>Code::KeyT,
            "U"=>Code::KeyU,"V"=>Code::KeyV,"W"=>Code::KeyW,"X"=>Code::KeyX,
            "Y"=>Code::KeyY,"Z"=>Code::KeyZ,
            "F1"=>Code::F1,"F2"=>Code::F2,"F3"=>Code::F3,"F4"=>Code::F4,
            "F5"=>Code::F5,"F6"=>Code::F6,"F7"=>Code::F7,"F8"=>Code::F8,
            "F9"=>Code::F9,"F10"=>Code::F10,"F11"=>Code::F11,"F12"=>Code::F12,
            _ => Code::F1,
        };
        HotKey::new(if mods.is_empty() { None } else { Some(mods) }, code)
    }
}
fn hotkey_path() -> Option<std::path::PathBuf> {
    Some(dirs::data_local_dir()?.join("clip-vault").join("hotkey.json"))
}

// ── Color format ──────────────────────────────────────────────────────────────

#[derive(Clone, Copy, PartialEq)]
enum ColorFmt { Hex, Rgb }
impl ColorFmt {
    fn format(self, c: Color32) -> String {
        match self {
            ColorFmt::Hex => format!("#{:02X}{:02X}{:02X}", c.r(), c.g(), c.b()),
            ColorFmt::Rgb => format!("rgb({},{},{})", c.r(), c.g(), c.b()),
        }
    }
    fn toggle(self) -> Self { match self { ColorFmt::Hex=>ColorFmt::Rgb, ColorFmt::Rgb=>ColorFmt::Hex } }
}

// ── Capture history ───────────────────────────────────────────────────────────

const MAX_HISTORY: usize = 10;
pub struct HistoryEntry { pub pixels: Vec<u8>, pub w: usize, pub h: usize }

// ── State machine ─────────────────────────────────────────────────────────────

#[derive(PartialEq)]
enum CaptureState { Idle, Selecting, Editing }

pub struct CaptureResult { pub pixels: Vec<u8>, pub width: usize, pub height: usize }

// ── Main struct ───────────────────────────────────────────────────────────────

pub struct AnnotateApp {
    capture_state: CaptureState,
    full_pixels: Option<Vec<u8>>, full_w: usize, full_h: usize,
    full_texture: Option<TextureHandle>,
    sel_start: Option<Pos2>, sel_cur: Option<Pos2>,
    cursor_px: (i32, i32),
    show_magnifier: bool,
    color_fmt: ColorFmt,
    picked_color: Option<Color32>,

    pixels: Option<Vec<u8>>, img_w: usize, img_h: usize,
    baked: Option<Vec<u8>>,
    texture: Option<TextureHandle>, texture_dirty: bool,
    /// Zoom level for editing view (Ctrl+scroll)
    zoom: f32,

    annotations: Vec<Annotation>,
    undo_stack:  Vec<Vec<Annotation>>,

    tool:       ShapeKind,
    color:      Color32,
    stroke_w:   f32,
    filled:     bool,
    line_style: LineStyle,
    drag_start: Option<Pos2>, cur_drag: Option<Pos2>,
    pen_points: Vec<Pos2>,
    /// Text input buffer for Text tool
    text_input: String,
    /// Next sequence number for Number tool
    next_number: u32,

    tool_colors: ToolColors,
    palette:     Palette,
    palette_cache: Vec<Color32>, // cached Color32 list, rebuilt only when palette changes
    show_palette_editor: bool,

    history: Vec<HistoryEntry>, history_idx: Option<usize>,

    pub hotkey:      HotkeyConfig,
    editing_hotkey:  bool,
    pub hotkey_changed: bool,

    smart_rect: Option<[i32; 4]>,
    #[cfg(target_os = "windows")]
    own_hwnds: Vec<isize>,
    #[cfg(target_os = "windows")]
    mouse_hook: Option<crate::win_capture::LowLevelMouseHook>,
    /// Throttle cache for hovered_window_rect
    #[cfg(target_os = "windows")]
    smart_last_pos:  (i32, i32),
    #[cfg(target_os = "windows")]
    smart_last_rect: Option<[i32; 4]>,

    capture_rx: Option<std::sync::mpsc::Receiver<CaptureResult>>,
    capture_hotkey_rx: Option<std::sync::mpsc::Receiver<CaptureResult>>,
    pub capture_btn_clicked: bool,
    status: String,
    /// Pinned images: (pixels, w, h, viewport_id, open)
    pinned: Vec<PinnedImage>,

    // Win+drag super-capture state (Windows only)
    #[cfg(target_os = "windows")]
    super_hook_mouse: Option<crate::win_capture::LowLevelMouseHook>,
    #[cfg(target_os = "windows")]
    super_hook_kb: Option<crate::win_capture::LowLevelKeyboardHook>,
    /// Win+drag selection in progress
    super_drag_start: Option<(i32, i32)>,
    super_drag_cur:   Option<(i32, i32)>,
    /// Overlay texture for Win+drag (full screen)
    super_texture: Option<TextureHandle>,
    super_pixels:  Option<Vec<u8>>,
    super_w: usize, super_h: usize,
    super_active: bool,
    /// Set when super-capture completes, so App can switch to Annotate tab
    pub tab_switch_needed: bool,

    // Saved window state for non-Windows overlay mode
    saved_window_pos: Option<Pos2>,
    saved_window_size: Option<Vec2>,
}

impl Default for AnnotateApp {
    fn default() -> Self {
        let tool_colors = ToolColors::load();
        let color = tool_colors.get(ShapeKind::Rect);
        Self {
            capture_state: CaptureState::Idle,
            full_pixels: None, full_w: 0, full_h: 0, full_texture: None,
            sel_start: None, sel_cur: None,
            cursor_px: (0,0), show_magnifier: false,
            color_fmt: ColorFmt::Hex, picked_color: None,
            pixels: None, img_w: 0, img_h: 0,
            baked: None, texture: None, texture_dirty: false,
            annotations: Vec::new(), undo_stack: Vec::new(),
            tool: ShapeKind::Rect, color, stroke_w: 3.0, filled: false,
            line_style: LineStyle::Solid,
            drag_start: None, cur_drag: None, pen_points: Vec::new(),
            text_input: String::new(), next_number: 1,
            tool_colors, palette: Palette::load(), palette_cache: Vec::new(), show_palette_editor: false,
            history: Vec::new(), history_idx: None,
            hotkey: HotkeyConfig::load(), editing_hotkey: false, hotkey_changed: false,
            smart_rect: None,
            #[cfg(target_os = "windows")] own_hwnds: Vec::new(),
            #[cfg(target_os = "windows")] mouse_hook: None,
            #[cfg(target_os = "windows")] smart_last_pos: (i32::MIN, i32::MIN),
            #[cfg(target_os = "windows")] smart_last_rect: None,
            capture_rx: None, capture_hotkey_rx: None, capture_btn_clicked: false,
            status: String::new(),
            zoom: 1.0,
            pinned: Vec::new(),
            #[cfg(target_os = "windows")] super_hook_mouse: None,
            #[cfg(target_os = "windows")] super_hook_kb: None,
            super_drag_start: None, super_drag_cur: None,
            super_texture: None, super_pixels: None,
            super_w: 0, super_h: 0, super_active: false,
            tab_switch_needed: false,
            saved_window_pos: None,
            saved_window_size: None,
        }
    }
}

impl AnnotateApp {
    pub fn is_selecting(&self) -> bool { self.capture_state == CaptureState::Selecting }
    pub fn trigger_capture(&mut self) {
        if self.capture_state == CaptureState::Idle { self.capture_btn_clicked = true; }
    }

    pub fn enter_overlay(&mut self, ctx: &egui::Context) {
        #[cfg(target_os = "windows")]
        {
            ctx.send_viewport_cmd(egui::ViewportCommand::Fullscreen(true));
        }
        #[cfg(not(target_os = "windows"))]
        {
            // Save current window state
            ctx.input(|i| {
                if let Some(pos) = i.viewport().outer_rect.map(|r| r.min) {
                    self.saved_window_pos = Some(pos);
                }
                if let Some(rect) = i.viewport().inner_rect {
                    self.saved_window_size = Some(rect.size());
                }
            });
            // Cover the screen without native fullscreen
            let screen = ctx.screen_rect();
            ctx.send_viewport_cmd(egui::ViewportCommand::Decorations(false));
            ctx.send_viewport_cmd(egui::ViewportCommand::OuterPosition(egui::pos2(screen.min.x, screen.min.y)));
            ctx.send_viewport_cmd(egui::ViewportCommand::InnerSize(screen.size()));
            ctx.send_viewport_cmd(egui::ViewportCommand::WindowLevel(egui::WindowLevel::AlwaysOnTop));
        }
    }

    pub fn exit_overlay(&mut self, ctx: &egui::Context) {
        #[cfg(target_os = "windows")]
        {
            ctx.send_viewport_cmd(egui::ViewportCommand::Fullscreen(false));
        }
        #[cfg(not(target_os = "windows"))]
        {
            ctx.send_viewport_cmd(egui::ViewportCommand::WindowLevel(egui::WindowLevel::Normal));
            ctx.send_viewport_cmd(egui::ViewportCommand::Decorations(true));
            if let Some(pos) = self.saved_window_pos.take() {
                ctx.send_viewport_cmd(egui::ViewportCommand::OuterPosition(pos));
            }
            if let Some(size) = self.saved_window_size.take() {
                ctx.send_viewport_cmd(egui::ViewportCommand::InnerSize(size));
            }
        }
    }

    pub fn start_capture(&mut self) {
        #[cfg(target_os = "windows")]
        { self.own_hwnds = crate::win_capture::get_process_hwnds(); }
        let (tx, rx) = std::sync::mpsc::channel::<CaptureResult>();
        self.capture_rx = Some(rx);
        std::thread::spawn(move || {
            std::thread::sleep(std::time::Duration::from_millis(300));
            #[cfg(target_os = "windows")]
            if let Some((pixels, w, h)) = crate::win_capture::capture_fullscreen() {
                let _ = tx.send(CaptureResult { pixels, width: w, height: h }); return;
            }
            if let Ok(screens) = screenshots::Screen::all() {
                if screens.len() == 1 {
                    // Single monitor: fast path (unchanged behavior)
                    if let Some(screen) = screens.into_iter().next() {
                        if let Ok(img) = screen.capture() {
                            let w = img.width() as usize; let h = img.height() as usize;
                            let _ = tx.send(CaptureResult { pixels: img.into_raw(), width: w, height: h });
                        }
                    }
                } else if !screens.is_empty() {
                    // Multi-monitor: composite all screens
                    let mut captures: Vec<(i32, i32, usize, usize, Vec<u8>)> = Vec::new();
                    let mut min_x = i32::MAX; let mut min_y = i32::MAX;
                    let mut max_x = i32::MIN; let mut max_y = i32::MIN;
                    for screen in &screens {
                        let info = screen.display_info;
                        let x = info.x; let y = info.y;
                        if let Ok(img) = screen.capture() {
                            let sw = img.width() as usize; let sh = img.height() as usize;
                            min_x = min_x.min(x); min_y = min_y.min(y);
                            max_x = max_x.max(x + sw as i32); max_y = max_y.max(y + sh as i32);
                            captures.push((x, y, sw, sh, img.into_raw()));
                        }
                    }
                    if !captures.is_empty() {
                        let total_w = (max_x - min_x) as usize;
                        let total_h = (max_y - min_y) as usize;
                        let mut composite = vec![0u8; total_w * total_h * 4];
                        for (sx, sy, sw, sh, pixels) in &captures {
                            let ox = (*sx - min_x) as usize;
                            let oy = (*sy - min_y) as usize;
                            for row in 0..*sh {
                                let src_start = row * sw * 4;
                                let dst_start = ((oy + row) * total_w + ox) * 4;
                                let len = sw * 4;
                                if src_start + len <= pixels.len() && dst_start + len <= composite.len() {
                                    composite[dst_start..dst_start + len].copy_from_slice(&pixels[src_start..src_start + len]);
                                }
                            }
                        }
                        let _ = tx.send(CaptureResult { pixels: composite, width: total_w, height: total_h });
                    }
                }
            }
        });
    }

    /// Called when hotkey triggers capture — window is already being minimized.
    /// Uses a longer delay to ensure window is fully hidden before capture.
    pub fn start_capture_after_minimize(&mut self) {
        #[cfg(target_os = "windows")]
        { self.own_hwnds = crate::win_capture::get_process_hwnds(); }
        let (tx, rx) = std::sync::mpsc::channel::<CaptureResult>();
        self.capture_hotkey_rx = Some(rx);
        std::thread::spawn(move || {
            std::thread::sleep(std::time::Duration::from_millis(400));
            #[cfg(target_os = "windows")]
            if let Some((pixels, w, h)) = crate::win_capture::capture_fullscreen() {
                let _ = tx.send(CaptureResult { pixels, width: w, height: h }); return;
            }
            if let Ok(screens) = screenshots::Screen::all() {
                if screens.len() == 1 {
                    // Single monitor: fast path (unchanged behavior)
                    if let Some(screen) = screens.into_iter().next() {
                        if let Ok(img) = screen.capture() {
                            let w = img.width() as usize; let h = img.height() as usize;
                            let _ = tx.send(CaptureResult { pixels: img.into_raw(), width: w, height: h });
                        }
                    }
                } else if !screens.is_empty() {
                    // Multi-monitor: composite all screens
                    let mut captures: Vec<(i32, i32, usize, usize, Vec<u8>)> = Vec::new();
                    let mut min_x = i32::MAX; let mut min_y = i32::MAX;
                    let mut max_x = i32::MIN; let mut max_y = i32::MIN;
                    for screen in &screens {
                        let info = screen.display_info;
                        let x = info.x; let y = info.y;
                        if let Ok(img) = screen.capture() {
                            let sw = img.width() as usize; let sh = img.height() as usize;
                            min_x = min_x.min(x); min_y = min_y.min(y);
                            max_x = max_x.max(x + sw as i32); max_y = max_y.max(y + sh as i32);
                            captures.push((x, y, sw, sh, img.into_raw()));
                        }
                    }
                    if !captures.is_empty() {
                        let total_w = (max_x - min_x) as usize;
                        let total_h = (max_y - min_y) as usize;
                        let mut composite = vec![0u8; total_w * total_h * 4];
                        for (sx, sy, sw, sh, pixels) in &captures {
                            let ox = (*sx - min_x) as usize;
                            let oy = (*sy - min_y) as usize;
                            for row in 0..*sh {
                                let src_start = row * sw * 4;
                                let dst_start = ((oy + row) * total_w + ox) * 4;
                                let len = sw * 4;
                                if src_start + len <= pixels.len() && dst_start + len <= composite.len() {
                                    composite[dst_start..dst_start + len].copy_from_slice(&pixels[src_start..src_start + len]);
                                }
                            }
                        }
                        let _ = tx.send(CaptureResult { pixels: composite, width: total_w, height: total_h });
                    }
                }
            }
        });
    }

    pub fn poll_capture_hotkey(&mut self) -> bool {
        let rx = match self.capture_hotkey_rx.as_ref() { Some(r)=>r, None=>return false };
        match rx.try_recv() {
            Ok(result) => {
                self.full_w = result.width; self.full_h = result.height;
                self.full_pixels = Some(result.pixels);
                self.full_texture = None;
                self.sel_start = None; self.sel_cur = None; self.smart_rect = None;
                self.show_magnifier = false; self.picked_color = None;
                self.capture_state = CaptureState::Selecting;
                self.capture_hotkey_rx = None;
                self.status = "拖拽框选  •  悬停单击智能框选  •  Alt放大镜  •  C取色  •  Esc取消".to_string();
                #[cfg(target_os = "windows")]
                { self.mouse_hook = crate::win_capture::LowLevelMouseHook::install(); }
                true
            }
            Err(std::sync::mpsc::TryRecvError::Empty) => false,
            Err(_) => { self.capture_hotkey_rx = None; false }
        }
    }

    pub fn poll_capture(&mut self) -> bool {
        let rx = match self.capture_rx.as_ref() { Some(r)=>r, None=>return false };
        match rx.try_recv() {
            Ok(result) => {
                self.full_w = result.width; self.full_h = result.height;
                self.full_pixels = Some(result.pixels);
                self.full_texture = None;
                self.sel_start = None; self.sel_cur = None; self.smart_rect = None;
                self.show_magnifier = false; self.picked_color = None;
                self.capture_state = CaptureState::Selecting;
                self.capture_rx = None;
                self.status = "拖拽框选  •  悬停单击智能框选  •  Alt放大镜  •  C取色  •  Esc取消".to_string();
                #[cfg(target_os = "windows")]
                { self.mouse_hook = crate::win_capture::LowLevelMouseHook::install(); }
                true
            }
            Err(std::sync::mpsc::TryRecvError::Empty) => false,
            Err(_) => { self.capture_rx = None; false }
        }
    }

    fn history_navigate(&mut self, delta: i32) {
        if self.history.is_empty() { return; }
        let n = self.history.len() as i32;
        let cur = self.history_idx.map(|i| i as i32).unwrap_or(n);
        let new_idx = (cur - delta).clamp(0, n-1) as usize;
        if new_idx as i32 == cur { return; }
        let e = &self.history[new_idx];
        self.img_w = e.w; self.img_h = e.h;
        self.pixels = Some(e.pixels.clone()); self.baked = Some(e.pixels.clone());
        self.annotations.clear(); self.undo_stack.clear();
        self.texture = None; self.texture_dirty = true;
        self.capture_state = CaptureState::Editing;
        self.history_idx = Some(new_idx);
        self.status = format!("历史记录 {}/{}", new_idx+1, self.history.len());
    }

    fn commit_selection(&mut self, ctx: &egui::Context) {
        #[cfg(target_os = "windows")] { self.mouse_hook = None; }
        let (x0,y0,x1,y1): (i32,i32,i32,i32);
        if let Some(sr) = self.smart_rect.take() {
            x0=sr[0]; y0=sr[1]; x1=sr[2]; y1=sr[3];
        } else {
            let (s,e) = match (self.sel_start,self.sel_cur) { (Some(s),Some(e))=>(s,e), _=>return };
            let sc = ctx.pixels_per_point();
            x0=(s.x.min(e.x)*sc) as i32; y0=(s.y.min(e.y)*sc) as i32;
            x1=((s.x.max(e.x)*sc) as i32).min(self.full_w as i32);
            y1=((s.y.max(e.y)*sc) as i32).min(self.full_h as i32);
        }
        let cw=(x1-x0).max(0) as usize; let ch=(y1-y0).max(0) as usize;
        if cw<4||ch<4 { return; }

        #[cfg(target_os = "windows")]
        let cropped = crate::win_capture::capture_rect_gdi(x0, y0, cw as i32, ch as i32);
        #[cfg(not(target_os = "windows"))]
        let cropped: Option<Vec<u8>> = None;

        let cropped = cropped.unwrap_or_else(|| {
            let base = match self.full_pixels.as_ref() { Some(p)=>p, None=>return vec![] };
            let mut out = vec![0u8; cw*ch*4];
            for row in 0..ch {
                let sy=(y0 as usize+row).min(self.full_h.saturating_sub(1));
                let src=(sy*self.full_w+x0 as usize)*4; let dst=row*cw*4;
                let len=(cw*4).min(base.len().saturating_sub(src));
                out[dst..dst+len].copy_from_slice(&base[src..src+len]);
            }
            out
        });
        if cropped.is_empty() { return; }

        if self.history_idx.is_none() {
            self.history.push(HistoryEntry { pixels: cropped.clone(), w: cw, h: ch });
            if self.history.len() > MAX_HISTORY { self.history.remove(0); }
        }
        self.history_idx = None;
        self.img_w=cw; self.img_h=ch;
        self.pixels=Some(cropped.clone()); self.baked=Some(cropped);
        self.annotations.clear(); self.undo_stack.clear();
        self.next_number = 1;
        self.texture=None; self.texture_dirty=true;
        self.capture_state=CaptureState::Editing;
        self.status=format!("{}×{}  Enter复制  Ctrl+S保存  Esc关闭", cw, ch);
        self.full_pixels=None; self.full_texture=None;
    }

    fn undo(&mut self) {
        if let Some(prev) = self.undo_stack.pop() {
            // Restore next_number from annotation count
            self.next_number = prev.iter().filter(|a| a.kind==ShapeKind::Number).count() as u32 + 1;
            self.annotations=prev; self.rebuild_baked(); self.texture_dirty=true;
        }
    }
    fn rebuild_baked(&mut self) {
        if let Some(ref base) = self.pixels {
            let mut buf = base.clone();
            let (w,h)=(self.img_w,self.img_h);
            for ann in &self.annotations { render_annotation_to_buf(&mut buf, w, h, ann, Some(base.as_slice())); }
            self.baked=Some(buf);
        }
    }
    fn bake_last(&mut self) {
        if let (Some(ref mut baked), Some(ann)) = (&mut self.baked, self.annotations.last()) {
            let (w,h)=(self.img_w,self.img_h);
            render_annotation_to_buf(baked, w, h, ann, self.pixels.as_deref());
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
    fn sample_color(&self, px: i32, py: i32) -> Option<Color32> {
        let buf = self.full_pixels.as_ref()?;
        let x=px.clamp(0,self.full_w as i32-1) as usize;
        let y=py.clamp(0,self.full_h as i32-1) as usize;
        let i=(y*self.full_w+x)*4;
        if i+3>=buf.len() { return None; }
        Some(Color32::from_rgb(buf[i],buf[i+1],buf[i+2]))
    }
    /// Switch tool and load its remembered color
    fn set_tool(&mut self, kind: ShapeKind) {
        self.tool = kind;
        self.color = self.tool_colors.get(kind);
    }
    /// Save current color to tool memory
    fn save_tool_color(&mut self) {
        self.tool_colors.set(self.tool, self.color);
        self.tool_colors.save();
    }

    pub fn update(&mut self, ctx: &egui::Context) -> bool {
        // ── Win+drag super-capture (Windows only) ───────────────────────────────
        #[cfg(target_os = "windows")]
        self.update_super_capture(ctx);

        // If super-capture is active, render its overlay and skip normal flow
        #[cfg(target_os = "windows")]
        if self.super_active {
            self.render_super_overlay(ctx);
            return false;
        }

        if self.capture_state == CaptureState::Idle && !self.editing_hotkey {
            if self.hotkey.matches(ctx) { return true; }
        }

        if self.capture_state == CaptureState::Selecting {
            if ctx.input(|i| i.key_pressed(Key::Escape)) {
                #[cfg(target_os = "windows")] { self.mouse_hook = None; }
                self.capture_state=CaptureState::Idle;
                self.full_pixels=None; self.full_texture=None; self.smart_rect=None;
                self.status=String::new();
                self.exit_overlay(ctx);
                return false;
            }
            if self.full_texture.is_none() {
                if let Some(ref px) = self.full_pixels {
                    let ci = ColorImage::from_rgba_unmultiplied([self.full_w,self.full_h], px);
                    self.full_texture = Some(ctx.load_texture("fullscreen", ci, egui::TextureOptions::NEAREST));
                }
            }
            #[cfg(target_os = "windows")]
            let (hook_pos, hook_clicked) = {
                let st = crate::win_capture::LowLevelMouseHook::poll();
                (Some(st.pos), st.clicked)
            };
            #[cfg(not(target_os = "windows"))]
            let (hook_pos, hook_clicked): (Option<(i32,i32)>, bool) = (None, false);

            if let Some((hx,hy)) = hook_pos { self.cursor_px=(hx,hy); }
            self.show_magnifier = ctx.input(|i| i.modifiers.alt);

            if self.show_magnifier && ctx.input(|i| i.key_pressed(Key::C)) {
                if let Some(c) = self.sample_color(self.cursor_px.0, self.cursor_px.1) {
                    self.picked_color = Some(c);
                    let s = self.color_fmt.format(c);
                    let _ = arboard::Clipboard::new().and_then(|mut cb| cb.set_text(s.clone()));
                    self.status = format!("已复制颜色: {s}  (Shift+F切换格式)");
                }
            }
            if ctx.input(|i| i.modifiers.shift && i.key_pressed(Key::F)) {
                self.color_fmt = self.color_fmt.toggle();
            }
            if ctx.input(|i| i.key_pressed(Key::Comma))  { self.history_navigate(-1); return false; }
            if ctx.input(|i| i.key_pressed(Key::Period))  { self.history_navigate(1);  return false; }

            let scale = ctx.pixels_per_point();
            let (mut cx,mut cy) = self.cursor_px;
            if ctx.input(|i| i.key_pressed(Key::W)) { cy-=1; }
            if ctx.input(|i| i.key_pressed(Key::S)) { cy+=1; }
            if ctx.input(|i| i.key_pressed(Key::A)) { cx-=1; }
            if ctx.input(|i| i.key_pressed(Key::D)) { cx+=1; }
            self.cursor_px=(cx,cy);

            if self.sel_start.is_some() {
                let step = 1.0/scale;
                let ctrl=ctx.input(|i| i.modifiers.ctrl);
                let shift=ctx.input(|i| i.modifiers.shift);
                let mut dx=0.0f32; let mut dy=0.0f32;
                if ctx.input(|i| i.key_pressed(Key::ArrowLeft))  { dx=-step; }
                if ctx.input(|i| i.key_pressed(Key::ArrowRight)) { dx= step; }
                if ctx.input(|i| i.key_pressed(Key::ArrowUp))    { dy=-step; }
                if ctx.input(|i| i.key_pressed(Key::ArrowDown))  { dy= step; }
                if dx!=0.0||dy!=0.0 {
                    if ctrl { if let Some(ref mut e)=self.sel_cur { e.x+=dx; e.y+=dy; } }
                    else if shift { if let Some(ref mut e)=self.sel_cur { e.x-=dx; e.y-=dy; } }
                    else {
                        if let Some(ref mut s)=self.sel_start { s.x+=dx; s.y+=dy; }
                        if let Some(ref mut e)=self.sel_cur   { e.x+=dx; e.y+=dy; }
                    }
                    ctx.request_repaint();
                }
            }

            if self.sel_start.is_none() {
                #[cfg(target_os = "windows")]
                { self.smart_rect = crate::win_capture::hovered_window_rect(
                    self.cursor_px.0, self.cursor_px.1, &self.own_hwnds,
                    &mut self.smart_last_pos, &mut self.smart_last_rect,
                ); }
            }
            if hook_clicked && self.sel_start.is_none() && self.smart_rect.is_some() {
                #[cfg(target_os = "windows")]
                if let Some(ref hook) = self.mouse_hook { hook.consume_click(); }
                self.exit_overlay(ctx);
                self.commit_selection(ctx); return false;
            }

            egui::CentralPanel::default().frame(egui::Frame::NONE).show(ctx, |ui| {
                let avail = ui.available_size();
                let (overlay_rect, resp) = ui.allocate_exact_size(avail, egui::Sense::drag());
                let painter = ui.painter_at(overlay_rect);
                if let Some(ref tex) = self.full_texture {
                    painter.image(tex.id(), overlay_rect, Rect::from_min_max(Pos2::ZERO, Pos2::new(1.0,1.0)), Color32::WHITE);
                }
                painter.rect_filled(overlay_rect, 0.0, Color32::from_black_alpha(80));

                if self.sel_start.is_none() {
                    if let Some(sr) = self.smart_rect {
                        let r = Rect::from_min_max(
                            Pos2::new(sr[0] as f32/scale, sr[1] as f32/scale),
                            Pos2::new(sr[2] as f32/scale, sr[3] as f32/scale),
                        );
                        if let Some(ref tex) = self.full_texture {
                            let uv = Rect::from_min_max(
                                Pos2::new((r.min.x-overlay_rect.min.x)/overlay_rect.width(),(r.min.y-overlay_rect.min.y)/overlay_rect.height()),
                                Pos2::new((r.max.x-overlay_rect.min.x)/overlay_rect.width(),(r.max.y-overlay_rect.min.y)/overlay_rect.height()),
                            );
                            painter.image(tex.id(), r, uv, Color32::WHITE);
                        }
                        painter.rect_stroke(r, 0.0, Stroke::new(2.0, Color32::from_rgb(56,189,248)), egui::StrokeKind::Outside);
                        painter.text(r.max+Vec2::new(4.0,-14.0), egui::Align2::LEFT_TOP,
                            format!("{}×{}  单击确认", sr[2]-sr[0], sr[3]-sr[1]),
                            egui::FontId::proportional(12.0), Color32::from_rgb(56,189,248));
                    }
                }

                if resp.drag_started() {
                    self.sel_start=resp.interact_pointer_pos(); self.sel_cur=self.sel_start; self.smart_rect=None;
                }
                if resp.dragged() {
                    self.sel_cur=resp.interact_pointer_pos();
                    if let Some(p)=resp.interact_pointer_pos() { self.cursor_px=((p.x*scale) as i32,(p.y*scale) as i32); }
                    ctx.request_repaint();
                }
                if ui.input(|i| i.pointer.secondary_clicked()) && self.sel_start.is_some() {
                    self.sel_start=None; self.sel_cur=None;
                }
                if resp.drag_stopped() {
                    self.sel_cur=resp.interact_pointer_pos();
                    self.exit_overlay(ctx);
                    self.commit_selection(ctx); return;
                }

                if let (Some(s),Some(e)) = (self.sel_start,self.sel_cur) {
                    let sr = Rect::from_two_pos(s,e);
                    if let Some(ref tex) = self.full_texture {
                        let uv = Rect::from_min_max(
                            Pos2::new((sr.min.x-overlay_rect.min.x)/overlay_rect.width(),(sr.min.y-overlay_rect.min.y)/overlay_rect.height()),
                            Pos2::new((sr.max.x-overlay_rect.min.x)/overlay_rect.width(),(sr.max.y-overlay_rect.min.y)/overlay_rect.height()),
                        );
                        painter.image(tex.id(), sr, uv, Color32::WHITE);
                    }
                    painter.rect_stroke(sr, 0.0, Stroke::new(2.0, Color32::WHITE), egui::StrokeKind::Outside);
                    let pw=((sr.width()*scale) as u32).max(0); let ph=((sr.height()*scale) as u32).max(0);
                    painter.text(sr.max+Vec2::new(4.0,-14.0), egui::Align2::LEFT_TOP,
                        format!("{}×{}", pw, ph), egui::FontId::proportional(12.0), Color32::WHITE);
                }

                let cursor_logical = Pos2::new(self.cursor_px.0 as f32/scale, self.cursor_px.1 as f32/scale);
                if self.show_magnifier || self.sel_start.is_none() {
                    draw_magnifier(&painter, &self.full_texture, self.full_w, self.full_h,
                        cursor_logical, overlay_rect, scale,
                        self.sample_color(self.cursor_px.0, self.cursor_px.1), self.color_fmt);
                }
                if self.sel_start.is_none() && self.smart_rect.is_none() {
                    painter.text(overlay_rect.center()+Vec2::new(0.0,40.0), egui::Align2::CENTER_CENTER,
                        "拖拽框选  •  悬停单击智能框选  •  Alt放大镜  •  C取色  •  Esc取消",
                        egui::FontId::proportional(14.0), Color32::WHITE);
                }
                ctx.request_repaint();
            });
            return false;
        }

        // Settings panel
        egui::TopBottomPanel::top("ann_settings")
            .frame(egui::Frame::side_top_panel(&ctx.style())
                .inner_margin(egui::Margin { left:10,right:10,top:6,bottom:6 }))
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
                    ui.checkbox(&mut self.hotkey.ctrl,"Ctrl");
                    ui.checkbox(&mut self.hotkey.shift,"Shift");
                    ui.checkbox(&mut self.hotkey.alt,"Alt");
                    ui.add(egui::TextEdit::singleline(&mut self.hotkey.key).desired_width(40.0).hint_text("F1"));
                    if ui.button("✓ 保存").clicked() { self.hotkey.save_pub(); self.hotkey_changed=true; self.editing_hotkey=false; }
                    if ui.button("✕").clicked() { self.editing_hotkey=false; }
                } else {
                    ui.label(egui::RichText::new(self.hotkey.label()).color(Color32::from_rgb(56,189,248)).size(11.5));
                    if ui.small_button("✎").on_hover_text("自定义快捷键").clicked() { self.editing_hotkey=true; }
                }
                if !self.status.is_empty() {
                    ui.add(egui::Separator::default().vertical().spacing(4.0));
                    ui.label(egui::RichText::new(&self.status).color(Color32::from_rgb(148,163,184)).size(11.5));
                }
            });
        });
        if self.capture_btn_clicked { self.capture_btn_clicked=false; return true; }
        false
    }

    pub fn editing_panel(&mut self, ctx: &egui::Context) {
        if self.capture_state != CaptureState::Editing { return; }

        // Keyboard shortcuts
        if ctx.input(|i| i.key_pressed(Key::Enter) || (i.modifiers.ctrl && i.key_pressed(Key::C))) {
            match self.copy_to_clipboard() {
                Ok(_)  => self.status="已复制到剪贴板".to_string(),
                Err(e) => self.status=format!("复制失败: {e}"),
            }
        }
        if ctx.input(|i| i.modifiers.ctrl && i.key_pressed(Key::S)) {
            if let Some(path) = rfd::FileDialog::new().add_filter("PNG",&["png"]).set_file_name("screenshot.png").save_file() {
                match self.save_png(&path) {
                    Ok(_)  => self.status=format!("已保存: {}", path.display()),
                    Err(e) => self.status=format!("保存失败: {e}"),
                }
            }
        }
        if ctx.input(|i| i.key_pressed(Key::Escape)) {
            self.capture_state=CaptureState::Idle;
            self.pixels=None; self.baked=None; self.texture=None;
            self.annotations.clear(); self.undo_stack.clear(); self.status=String::new();
            return;
        }
        if ctx.input(|i| i.modifiers.ctrl && i.key_pressed(Key::Z)) { self.undo(); }
        // Ctrl+0 reset zoom
        if ctx.input(|i| i.modifiers.ctrl && i.key_pressed(Key::Num0)) { self.zoom = 1.0; }
        // Tool shortcuts
        ctx.input(|i| {
            if i.key_pressed(Key::R) { self.set_tool(ShapeKind::Rect); }
            if i.key_pressed(Key::E) { self.set_tool(ShapeKind::Ellipse); }
            if i.key_pressed(Key::A) { self.set_tool(ShapeKind::Arrow); }
            if i.key_pressed(Key::P) { self.set_tool(ShapeKind::Pen); }
            if i.key_pressed(Key::T) { self.set_tool(ShapeKind::Text); }
            if i.key_pressed(Key::Num1) { self.stroke_w=2.0; }
            if i.key_pressed(Key::Num2) { self.stroke_w=5.0; }
        });

        if self.texture_dirty {
            if let Some(ref rgba) = self.baked {
                let ci = ColorImage::from_rgba_unmultiplied([self.img_w,self.img_h], rgba);
                self.texture = Some(ctx.load_texture("screenshot", ci, egui::TextureOptions::NEAREST));
            }
            self.texture_dirty=false;
        }

        egui::CentralPanel::default().show(ctx, |ui| {
            // Ctrl+scroll anywhere in panel = zoom
            let scroll = ui.input(|i| i.smooth_scroll_delta.y);
            if ui.input(|i| i.modifiers.ctrl) && scroll.abs() > 0.1 {
                self.zoom = (self.zoom * (1.0 + scroll * 0.002)).clamp(0.1, 8.0);
            }
            egui::ScrollArea::both().auto_shrink(false).show(ui, |ui| {
                let img_size = Vec2::new(self.img_w as f32 * self.zoom, self.img_h as f32 * self.zoom);
                if let Some(ref tex) = self.texture {
                    let (cr, resp) = ui.allocate_exact_size(img_size, egui::Sense::drag());
                    let painter = ui.painter_at(cr);
                    painter.image(tex.id(), cr, Rect::from_min_max(Pos2::ZERO, Pos2::new(1.0,1.0)), Color32::WHITE);
                    // Convert between screen and image coords (accounting for zoom)
                    let zoom = self.zoom;
                    let to_img    = |p: Pos2| Pos2::new((p.x-cr.min.x).clamp(0.0,img_size.x)/zoom,(p.y-cr.min.y).clamp(0.0,img_size.y)/zoom);
                    let to_screen = |p: Pos2| Pos2::new(cr.min.x+p.x*zoom, cr.min.y+p.y*zoom);
                    for ann in &self.annotations { draw_annotation_egui(&painter, ann, zoom, &to_screen); }

                    // Scroll wheel without Ctrl = stroke width
                    if resp.hovered() && scroll.abs()>0.1 && !ui.input(|i| i.modifiers.ctrl) {
                        self.stroke_w = (self.stroke_w + scroll*0.3).clamp(1.0,30.0);
                    }

                    // Right-click: cancel pen stroke
                    if resp.secondary_clicked() && self.tool==ShapeKind::Pen && !self.pen_points.is_empty() {
                        self.pen_points.clear();
                    }

                    if resp.drag_started() {
                        if let Some(pos) = resp.interact_pointer_pos() {
                            match self.tool {
                                ShapeKind::Pen | ShapeKind::Marker => {
                                    self.undo_stack.push(self.annotations.clone());
                                    self.pen_points.clear(); self.pen_points.push(to_img(pos));
                                }
                                ShapeKind::Text => {
                                    // Text: place at click position
                                    if !self.text_input.is_empty() {
                                        self.undo_stack.push(self.annotations.clone());
                                        let mut ann = Annotation::new(ShapeKind::Text, self.color, self.stroke_w, false, self.line_style, to_img(pos), to_img(pos));
                                        ann.text = self.text_input.clone();
                                        self.annotations.push(ann);
                                        self.bake_last(); self.texture_dirty=true;
                                    }
                                }
                                _ => { self.drag_start=Some(to_img(pos)); }
                            }
                        }
                    }
                    if resp.dragged() {
                        if let Some(pos) = resp.interact_pointer_pos() {
                            match self.tool {
                                ShapeKind::Pen | ShapeKind::Marker => { self.pen_points.push(to_img(pos)); }
                                _ => { self.cur_drag=Some(to_img(pos)); }
                            }
                            ctx.request_repaint();
                        }
                    }
                    if resp.drag_stopped() {
                        if let Some(pos) = resp.interact_pointer_pos() {
                            match self.tool {
                                ShapeKind::Pen | ShapeKind::Marker => {
                                    self.pen_points.push(to_img(pos));
                                    if self.pen_points.len()>=2 {
                                        let mut ann = Annotation::new(self.tool, self.color, self.stroke_w, false, self.line_style,
                                            *self.pen_points.first().unwrap(), *self.pen_points.last().unwrap());
                                        ann.pen_points = self.pen_points.clone();
                                        self.annotations.push(ann);
                                        self.bake_last(); self.texture_dirty=true;
                                    }
                                    self.pen_points.clear();
                                }
                                ShapeKind::Text => {}
                                _ => {
                                    if let Some(start) = self.drag_start.take() {
                                        let end = to_img(pos);
                                        if (end-start).length()>2.0 {
                                            self.undo_stack.push(self.annotations.clone());
                                            let mut ann = Annotation::new(self.tool, self.color, self.stroke_w, self.filled, self.line_style, start, end);
                                            if self.tool==ShapeKind::Number {
                                                ann.number = self.next_number;
                                                self.next_number += 1;
                                            }
                                            self.annotations.push(ann);
                                            self.bake_last(); self.texture_dirty=true;
                                        }
                                        self.cur_drag=None;
                                    }
                                }
                            }
                        }
                    }

                    // Preview
                    match self.tool {
                        ShapeKind::Pen | ShapeKind::Marker => {
                            if self.pen_points.len()>=2 {
                                for pair in self.pen_points.windows(2) {
                                    painter.line_segment([to_screen(pair[0]),to_screen(pair[1])], Stroke::new(self.stroke_w, self.color));
                                }
                            }
                        }
                        _ => {
                            if let (Some(s),Some(e)) = (self.drag_start,self.cur_drag) {
                                let mut preview = Annotation::new(self.tool, self.color, self.stroke_w, self.filled, self.line_style, s, e);
                                preview.number = self.next_number;
                                draw_annotation_egui(&painter, &preview, 1.0, &to_screen);
                            }
                        }
                    }
                }

                // ── Floating toolbar ──────────────────────────────────────────
                ui.add_space(8.0);
                // Zoom indicator
                if (self.zoom - 1.0).abs() > 0.01 {
                    ui.label(egui::RichText::new(format!("缩放: {:.0}%  (Ctrl+滚轮调整，Ctrl+0 重置)", self.zoom*100.0))
                        .color(Color32::from_rgb(148,163,184)).size(11.0));
                }
                egui::Frame::new()
                    .fill(Color32::from_rgba_unmultiplied(18,20,28,245))
                    .corner_radius(10.0)
                    .inner_margin(egui::Margin { left:10,right:10,top:8,bottom:8 })
                    .show(ui, |ui| {
                    // Row 1: tools
                    ui.horizontal_wrapped(|ui| {
                        let tools = [
                            ShapeKind::Rect, ShapeKind::RoundRect, ShapeKind::Ellipse,
                            ShapeKind::Arrow, ShapeKind::ArrowDouble, ShapeKind::Line,
                            ShapeKind::Pen, ShapeKind::Marker,
                            ShapeKind::Mosaic, ShapeKind::Eraser,
                            ShapeKind::Text, ShapeKind::Number,
                        ];
                        for kind in tools {
                            let sel = self.tool == kind;
                            if ui.add(egui::Button::new(kind.label())
                                .fill(if sel {Color32::from_rgb(6,78,59)} else {Color32::from_rgb(38,42,54)})
                                .min_size(egui::vec2(30.0,28.0))
                            ).on_hover_text(kind.tip()).clicked() { self.set_tool(kind); }
                        }
                    });
                    ui.add_space(4.0);
                    // Row 2: style options
                    ui.horizontal_wrapped(|ui| {
                        // Line style
                        for (ls, label, tip) in [
                            (LineStyle::Solid,  "—", "实线"),
                            (LineStyle::Dashed, "╌", "虚线"),
                            (LineStyle::Dotted, "·", "点线"),
                        ] {
                            let sel = self.line_style == ls;
                            if ui.add(egui::Button::new(label)
                                .fill(if sel {Color32::from_rgb(6,78,59)} else {Color32::from_rgb(38,42,54)})
                                .min_size(egui::vec2(28.0,24.0))
                            ).on_hover_text(tip).clicked() { self.line_style=ls; }
                        }
                        ui.add(egui::Separator::default().vertical().spacing(4.0));
                        // Fill toggle (for shapes)
                        ui.checkbox(&mut self.filled, "填充");
                        ui.add(egui::Separator::default().vertical().spacing(4.0));
                        // Stroke width
                        ui.add(egui::Slider::new(&mut self.stroke_w, 1.0..=30.0).show_value(false));
                        ui.label(egui::RichText::new(format!("{:.0}px", self.stroke_w)).size(11.0));
                        ui.add(egui::Separator::default().vertical().spacing(4.0));
                        // Opacity display
                        let alpha = self.color.a();
                        ui.label(egui::RichText::new(format!("{}%", alpha*100/255)).size(11.0).color(Color32::from_rgb(148,163,184)));
                    });
                    ui.add_space(4.0);
                    // Row 3: palette + actions
                    ui.horizontal_wrapped(|ui| {
                        // Custom palette — use cached Vec to avoid per-frame allocation
                        self.palette.colors_cached(&mut self.palette_cache);
                        let palette_colors = self.palette_cache.clone();
                        for (idx, &c) in palette_colors.iter().enumerate() {
                            let sel = self.color == c;
                            let (rect, resp) = ui.allocate_exact_size(egui::vec2(22.0,22.0), egui::Sense::click());
                            // Checkerboard for transparent colors
                            if c.a() < 255 {
                                ui.painter().rect_filled(rect, 3.0, Color32::WHITE);
                                ui.painter().rect_filled(Rect::from_min_size(rect.min, Vec2::splat(11.0)), 0.0, Color32::GRAY);
                                ui.painter().rect_filled(Rect::from_min_size(rect.min+Vec2::splat(11.0), Vec2::splat(11.0)), 0.0, Color32::GRAY);
                            }
                            ui.painter().rect_filled(rect, 3.0, c);
                            if sel { ui.painter().rect_stroke(rect, 3.0, Stroke::new(2.0,Color32::WHITE), egui::StrokeKind::Outside); }
                            if resp.clicked() {
                                self.color=c; self.save_tool_color();
                            }
                            // Right-click to replace palette slot
                            resp.context_menu(|ui| {
                                ui.label(format!("槽位 {}", idx+1));
                                if ui.button("替换为当前颜色").clicked() {
                                    self.palette.colors[idx] = [self.color.r(),self.color.g(),self.color.b(),self.color.a()];
                                    self.palette.save();
                                    ui.close_menu();
                                }
                            });
                        }
                        // Color picker
                        let old_color = self.color;
                        if ui.color_edit_button_srgba(&mut self.color).changed() && self.color != old_color {
                            self.save_tool_color();
                        }
                        ui.add(egui::Separator::default().vertical().spacing(4.0));
                        // Text input for Text tool
                        if self.tool == ShapeKind::Text {
                            ui.add(egui::TextEdit::singleline(&mut self.text_input)
                                .desired_width(120.0).hint_text("输入文字后点击放置"));
                            ui.add(egui::Separator::default().vertical().spacing(4.0));
                        }
                        // Actions
                        let has = !self.annotations.is_empty();
                        if ui.add_enabled(has, egui::Button::new("↩").min_size(egui::vec2(28.0,28.0))).on_hover_text("撤销 Ctrl+Z").clicked() { self.undo(); }
                        if ui.add_enabled(has, egui::Button::new("🗑").fill(Color32::from_rgb(100,20,20)).min_size(egui::vec2(28.0,28.0))).on_hover_text("清空").clicked() {
                            self.undo_stack.push(self.annotations.clone()); self.annotations.clear();
                            self.next_number=1; self.baked=self.pixels.clone(); self.texture_dirty=true;
                        }
                        ui.add(egui::Separator::default().vertical().spacing(4.0));
                        if ui.add(egui::Button::new("📋 复制").min_size(egui::vec2(60.0,28.0))).on_hover_text("Enter").clicked() {
                            match self.copy_to_clipboard() {
                                Ok(_) => self.status="已复制到剪贴板".to_string(),
                                Err(e) => self.status=format!("复制失败: {e}"),
                            }
                        }
                        if ui.add(egui::Button::new("💾 保存").min_size(egui::vec2(60.0,28.0))).on_hover_text("Ctrl+S").clicked() {
                            if let Some(path) = rfd::FileDialog::new().add_filter("PNG",&["png"]).set_file_name("screenshot.png").save_file() {
                                match self.save_png(&path) {
                                    Ok(_) => self.status=format!("已保存: {}", path.display()),
                                    Err(e) => self.status=format!("保存失败: {e}"),
                                }
                            }
                        }
                        // Pin to screen
                        if ui.add(egui::Button::new("📌 贴图").fill(Color32::from_rgb(80,50,10)).min_size(egui::vec2(60.0,28.0))).on_hover_text("将截图贴在屏幕上（置顶悬浮）").clicked() {
                            if let Some(ref baked) = self.baked {
                                self.pinned.push(PinnedImage::new(baked.clone(), self.img_w, self.img_h));
                            }
                        }
                        if ui.add(egui::Button::new("✕").min_size(egui::vec2(28.0,28.0))).on_hover_text("关闭 Esc").clicked() {
                            self.capture_state=CaptureState::Idle;
                            self.pixels=None; self.baked=None; self.texture=None;
                            self.annotations.clear(); self.undo_stack.clear(); self.status=String::new();
                        }
                    });
                });
            });
        });
    }
    /// Render all pinned floating windows. Call from App::update every frame.
    pub fn render_pinned(&mut self, ctx: &egui::Context) {
        self.pinned.retain(|p| p.open);
        for pin in &mut self.pinned {
            if pin.texture.is_none() {
                let ci = ColorImage::from_rgba_unmultiplied([pin.w, pin.h], &pin.pixels);
                pin.texture = Some(ctx.load_texture(
                    format!("pin_{:?}", pin.id), ci, egui::TextureOptions::LINEAR));
            }
            let tex_id = pin.texture.as_ref().map(|t| t.id());
            let w = pin.w; let h = pin.h;
            let open = &mut pin.open;
            let zoom = &mut pin.zoom;
            ctx.show_viewport_immediate(
                pin.id,
                egui::ViewportBuilder::default()
                    .with_title(format!("贴图 {}×{}", w, h))
                    .with_inner_size([w as f32 * *zoom, h as f32 * *zoom])
                    .with_always_on_top()
                    .with_resizable(true),
                move |ctx, _| {
                    if ctx.input(|i| i.viewport().close_requested()) { *open = false; }
                    if ctx.input(|i| i.key_pressed(egui::Key::Escape)) { *open = false; }
                    let scroll = ctx.input(|i| i.smooth_scroll_delta.y);
                    if ctx.input(|i| i.modifiers.ctrl) && scroll.abs() > 0.1 {
                        *zoom = (*zoom * (1.0 + scroll * 0.002)).clamp(0.1, 8.0);
                    }
                    if ctx.input(|i| i.modifiers.ctrl && i.key_pressed(egui::Key::Num0)) { *zoom = 1.0; }
                    egui::CentralPanel::default().frame(egui::Frame::NONE).show(ctx, |ui| {
                        if let Some(tid) = tex_id {
                            let avail = ui.available_size();
                            ui.add(egui::Image::new(egui::load::SizedTexture::new(
                                tid, egui::vec2(w as f32, h as f32)))
                                .fit_to_exact_size(avail));
                        }
                    });
                },
            );
        }
    }

    /// Install always-on hooks for Win+drag detection.
    /// Called once at startup from App.
    pub fn install_super_hooks(&mut self) {
        #[cfg(target_os = "windows")] {
            self.super_hook_mouse = crate::win_capture::LowLevelMouseHook::install();
            self.super_hook_kb    = crate::win_capture::LowLevelKeyboardHook::install();
        }
    }

    #[cfg(target_os = "windows")]
    fn update_super_capture(&mut self, ctx: &egui::Context) {
        let kb = crate::win_capture::LowLevelKeyboardHook::poll();
        let ms = crate::win_capture::LowLevelMouseHook::poll();

        // Only act when not already in a capture flow
        if self.capture_state != CaptureState::Idle { return; }

        if kb.win_held {
            if ms.clicked && self.super_drag_start.is_none() {
                // Win key held + LButton down: start drag
                // Grab full screen immediately (no window hide needed)
                if let Some((pixels, w, h)) = crate::win_capture::capture_fullscreen() {
                    self.super_pixels = Some(pixels);
                    self.super_w = w; self.super_h = h;
                    self.super_texture = None;
                    self.super_drag_start = Some(ms.pos);
                    self.super_drag_cur   = Some(ms.pos);
                    self.super_active = true;
                    // Consume the click so it doesn't trigger other things
                    if let Some(ref hook) = self.super_hook_mouse { hook.consume_click(); }
                    self.enter_overlay(ctx);
                    ctx.request_repaint();
                }
            } else if self.super_active {
                // Update drag position
                self.super_drag_cur = Some(ms.pos);
                ctx.request_repaint();

                if ms.released {
                    // Drag ended: commit selection
                    if let Some(ref hook) = self.super_hook_mouse { hook.consume_release(); }
                    self.exit_overlay(ctx);
                    self.commit_super_selection();
                }
            }
        } else if self.super_active {
            // Win key released mid-drag: cancel
            self.cancel_super_capture(ctx);
        }
    }

    #[cfg(target_os = "windows")]
    fn commit_super_selection(&mut self) {
        let (sx, sy) = match self.super_drag_start { Some(p)=>p, None=>{ self.cancel_super_capture_silent(); return; } };
        let (ex, ey) = match self.super_drag_cur   { Some(p)=>p, None=>{ self.cancel_super_capture_silent(); return; } };
        let x0 = sx.min(ex); let y0 = sy.min(ey);
        let x1 = sx.max(ex); let y1 = sy.max(ey);
        let cw = (x1-x0).max(0) as usize; let ch = (y1-y0).max(0) as usize;

        self.super_active = false;
        self.super_drag_start = None; self.super_drag_cur = None;
        self.super_texture = None; self.super_pixels = None;

        if cw < 4 || ch < 4 { return; }

        // Use GDI for a fresh capture of the exact rect
        if let Some(cropped) = crate::win_capture::capture_rect_gdi(x0, y0, cw as i32, ch as i32) {
            if !cropped.is_empty() {
                self.history.push(crate::annotate_app::HistoryEntry { pixels: cropped.clone(), w: cw, h: ch });
                if self.history.len() > MAX_HISTORY { self.history.remove(0); }
                self.img_w = cw; self.img_h = ch;
                self.pixels = Some(cropped.clone()); self.baked = Some(cropped);
                self.annotations.clear(); self.undo_stack.clear();
                self.next_number = 1;
                self.texture = None; self.texture_dirty = true;
                self.capture_state = CaptureState::Editing;
                self.status = format!("{}×{} — Win+拖拽截图完成", cw, ch);
                self.tab_switch_needed = true;
            }
        }
    }

    #[cfg(target_os = "windows")]
    fn cancel_super_capture(&mut self, ctx: &egui::Context) {
        self.exit_overlay(ctx);
        self.cancel_super_capture_silent();
    }

    #[cfg(target_os = "windows")]
    fn cancel_super_capture_silent(&mut self) {
        self.super_active = false;
        self.super_drag_start = None; self.super_drag_cur = None;
        self.super_texture = None; self.super_pixels = None;
    }

    #[cfg(target_os = "windows")]
    fn render_super_overlay(&mut self, ctx: &egui::Context) {
        // Upload full-screen texture once
        if self.super_texture.is_none() {
            if let Some(ref px) = self.super_pixels {
                use egui::ColorImage;
                let ci = ColorImage::from_rgba_unmultiplied([self.super_w, self.super_h], px);
                self.super_texture = Some(ctx.load_texture("super_fs", ci, egui::TextureOptions::NEAREST));
            }
        }

        // Esc cancels
        if ctx.input(|i| i.key_pressed(egui::Key::Escape)) {
            self.cancel_super_capture(ctx);
            return;
        }

        let scale = ctx.pixels_per_point();

        egui::CentralPanel::default().frame(egui::Frame::NONE).show(ctx, |ui| {
            let avail = ui.available_size();
            let (overlay_rect, _) = ui.allocate_exact_size(avail, egui::Sense::hover());
            let painter = ui.painter_at(overlay_rect);

            // Draw full-screen screenshot
            if let Some(ref tex) = self.super_texture {
                painter.image(tex.id(), overlay_rect,
                    egui::Rect::from_min_max(egui::Pos2::ZERO, egui::Pos2::new(1.0,1.0)),
                    egui::Color32::WHITE);
            }
            // Dim
            painter.rect_filled(overlay_rect, 0.0, egui::Color32::from_black_alpha(60));

            // Draw selection rect
            if let (Some(s), Some(e)) = (self.super_drag_start, self.super_drag_cur) {
                let sl = egui::Pos2::new(s.0 as f32/scale, s.1 as f32/scale);
                let el = egui::Pos2::new(e.0 as f32/scale, e.1 as f32/scale);
                let sr = egui::Rect::from_two_pos(sl, el);

                // Un-dim selected area
                if let Some(ref tex) = self.super_texture {
                    let uv = egui::Rect::from_min_max(
                        egui::Pos2::new((sr.min.x-overlay_rect.min.x)/overlay_rect.width(),
                                        (sr.min.y-overlay_rect.min.y)/overlay_rect.height()),
                        egui::Pos2::new((sr.max.x-overlay_rect.min.x)/overlay_rect.width(),
                                        (sr.max.y-overlay_rect.min.y)/overlay_rect.height()),
                    );
                    painter.image(tex.id(), sr, uv, egui::Color32::WHITE);
                }

                // Border
                painter.rect_stroke(sr, 0.0,
                    egui::Stroke::new(2.0, egui::Color32::from_rgb(56,189,248)),
                    egui::StrokeKind::Outside);

                // Size label
                let pw = ((sr.width()*scale) as u32).max(0);
                let ph = ((sr.height()*scale) as u32).max(0);
                painter.text(sr.max + egui::Vec2::new(4.0,-14.0),
                    egui::Align2::LEFT_TOP, format!("{}×{}", pw, ph),
                    egui::FontId::proportional(12.0), egui::Color32::WHITE);
            }

            // Hint
            if self.super_drag_start.is_none() {
                painter.text(overlay_rect.center(), egui::Align2::CENTER_CENTER,
                    "Win + 拖拽选择截图区域  •  Esc 取消",
                    egui::FontId::proportional(18.0), egui::Color32::WHITE);
            }
            ctx.request_repaint();
        });
    }

} // end impl AnnotateApp

// ── Magnifier ─────────────────────────────────────────────────────────────────

fn draw_magnifier(
    painter: &egui::Painter, full_tex: &Option<TextureHandle>,
    full_w: usize, full_h: usize, cursor: Pos2, overlay_rect: Rect,
    scale: f32, pixel_color: Option<Color32>, color_fmt: ColorFmt,
) {
    let mag_size = 160.0f32; let zoom = 8.0f32;
    let sample_r = (mag_size/zoom/2.0) as i32;
    let mut mx = cursor.x+20.0; let mut my = cursor.y+20.0;
    if mx+mag_size > overlay_rect.max.x-10.0 { mx=cursor.x-mag_size-20.0; }
    if my+mag_size+24.0 > overlay_rect.max.y-10.0 { my=cursor.y-mag_size-44.0; }
    let mag_rect = Rect::from_min_size(Pos2::new(mx,my), Vec2::splat(mag_size));
    if let Some(ref tex) = full_tex {
        let cx_px=(cursor.x*scale) as f32; let cy_px=(cursor.y*scale) as f32;
        let fw=full_w as f32; let fh=full_h as f32;
        let uv_min=Pos2::new(((cx_px-sample_r as f32)/fw).clamp(0.0,1.0),((cy_px-sample_r as f32)/fh).clamp(0.0,1.0));
        let uv_max=Pos2::new(((cx_px+sample_r as f32)/fw).clamp(0.0,1.0),((cy_px+sample_r as f32)/fh).clamp(0.0,1.0));
        painter.rect_filled(mag_rect, 4.0, Color32::BLACK);
        painter.image(tex.id(), mag_rect, Rect::from_min_max(uv_min,uv_max), Color32::WHITE);
        let center=mag_rect.center();
        painter.line_segment([Pos2::new(center.x,mag_rect.min.y),Pos2::new(center.x,mag_rect.max.y)], Stroke::new(1.0,Color32::from_rgba_unmultiplied(255,255,255,120)));
        painter.line_segment([Pos2::new(mag_rect.min.x,center.y),Pos2::new(mag_rect.max.x,center.y)], Stroke::new(1.0,Color32::from_rgba_unmultiplied(255,255,255,120)));
        painter.rect_stroke(mag_rect, 4.0, Stroke::new(2.0,Color32::from_rgb(56,189,248)), egui::StrokeKind::Outside);
    }
    if let Some(c) = pixel_color {
        let swatch=Rect::from_min_size(Pos2::new(mx,my+mag_size+2.0), Vec2::new(mag_size,20.0));
        painter.rect_filled(swatch, 3.0, c);
        let tc=if c.r() as u32+c.g() as u32+c.b() as u32>382 {Color32::BLACK} else {Color32::WHITE};
        painter.text(swatch.center(), egui::Align2::CENTER_CENTER, color_fmt.format(c), egui::FontId::monospace(11.0), tc);
    }
}

// ── egui painter helpers ──────────────────────────────────────────────────────

fn draw_annotation_egui(painter: &egui::Painter, ann: &Annotation, scale: f32, to_screen: &impl Fn(Pos2)->Pos2) {
    let stroke = Stroke::new(ann.width*scale, ann.color);
    match ann.kind {
        ShapeKind::Rect => {
            let r=Rect::from_two_pos(to_screen(ann.p1),to_screen(ann.p2));
            if ann.filled { painter.rect_filled(r, 0.0, ann.color); }
            else { draw_stroked_rect(painter, r, 0.0, stroke, ann.line_style); }
        }
        ShapeKind::RoundRect => {
            let r=Rect::from_two_pos(to_screen(ann.p1),to_screen(ann.p2));
            let radius = (r.width().min(r.height())*0.15).max(4.0);
            if ann.filled { painter.rect_filled(r, radius, ann.color); }
            else { painter.rect_stroke(r, radius, stroke, egui::StrokeKind::Middle); }
        }
        ShapeKind::Ellipse => {
            let center=to_screen(Pos2::new((ann.p1.x+ann.p2.x)/2.0,(ann.p1.y+ann.p2.y)/2.0));
            let radii=Vec2::new((ann.p2.x-ann.p1.x).abs()/2.0*scale,(ann.p2.y-ann.p1.y).abs()/2.0*scale);
            let pts: Vec<Pos2>=(0..=64).map(|i| { let t=i as f32/64.0*std::f32::consts::TAU; center+Vec2::new(radii.x*t.cos(),radii.y*t.sin()) }).collect();
            if ann.filled { painter.add(egui::Shape::convex_polygon(pts, ann.color, Stroke::NONE)); }
            else { painter.add(egui::Shape::closed_line(pts, stroke)); }
        }
        ShapeKind::Arrow => {
            let (s,e)=(to_screen(ann.p1),to_screen(ann.p2));
            draw_dashed_line(painter, s, e, stroke, ann.line_style);
            draw_arrowhead(painter, s, e, stroke);
        }
        ShapeKind::ArrowDouble => {
            let (s,e)=(to_screen(ann.p1),to_screen(ann.p2));
            draw_dashed_line(painter, s, e, stroke, ann.line_style);
            draw_arrowhead(painter, s, e, stroke);
            draw_arrowhead(painter, e, s, stroke);
        }
        ShapeKind::Line => {
            let (s,e)=(to_screen(ann.p1),to_screen(ann.p2));
            draw_dashed_line(painter, s, e, stroke, ann.line_style);
        }
        ShapeKind::Pen => {
            for pair in ann.pen_points.windows(2) {
                painter.line_segment([to_screen(pair[0]),to_screen(pair[1])], stroke);
            }
        }
        ShapeKind::Marker => {
            // Semi-transparent thick stroke
            for pair in ann.pen_points.windows(2) {
                painter.line_segment([to_screen(pair[0]),to_screen(pair[1])], Stroke::new(ann.width*scale*3.0, ann.color));
            }
        }
        ShapeKind::Mosaic | ShapeKind::Eraser => {
            // Shown as a rectangle outline during preview
            let r=Rect::from_two_pos(to_screen(ann.p1),to_screen(ann.p2));
            painter.rect_stroke(r, 0.0, Stroke::new(1.0, Color32::from_rgba_unmultiplied(255,255,255,100)), egui::StrokeKind::Middle);
        }
        ShapeKind::Text => {
            let pos=to_screen(ann.p1);
            painter.text(pos, egui::Align2::LEFT_TOP, &ann.text,
                egui::FontId::proportional(ann.width*scale*4.0), ann.color);
        }
        ShapeKind::Number => {
            let center=to_screen(Pos2::new((ann.p1.x+ann.p2.x)/2.0,(ann.p1.y+ann.p2.y)/2.0));
            let r=(ann.p2-ann.p1).length()/2.0*scale;
            painter.circle_filled(center, r, ann.color);
            painter.text(center, egui::Align2::CENTER_CENTER, &ann.number.to_string(),
                egui::FontId::proportional(r*1.2), Color32::WHITE);
        }
    }
}

fn draw_arrowhead(painter: &egui::Painter, from: Pos2, to: Pos2, stroke: Stroke) {
    painter.arrow(from, to-from, stroke);
}

fn draw_dashed_line(painter: &egui::Painter, s: Pos2, e: Pos2, stroke: Stroke, style: LineStyle) {
    match style {
        LineStyle::Solid  => { painter.line_segment([s,e], stroke); }
        LineStyle::Dashed => {
            let total=(e-s).length(); if total<1.0 { return; }
            let dir=(e-s)/total; let dash=12.0*stroke.width.max(1.0); let gap=6.0*stroke.width.max(1.0);
            let mut t=0.0f32;
            while t<total {
                let t2=(t+dash).min(total);
                painter.line_segment([s+dir*t, s+dir*t2], stroke);
                t+=dash+gap;
            }
        }
        LineStyle::Dotted => {
            let total=(e-s).length(); if total<1.0 { return; }
            let dir=(e-s)/total; let gap=stroke.width.max(1.0)*4.0;
            let mut t=0.0f32;
            while t<total {
                painter.circle_filled(s+dir*t, stroke.width*0.6, stroke.color);
                t+=gap;
            }
        }
    }
}

fn draw_stroked_rect(painter: &egui::Painter, r: Rect, rounding: f32, stroke: Stroke, style: LineStyle) {
    match style {
        LineStyle::Solid => { painter.rect_stroke(r, rounding, stroke, egui::StrokeKind::Middle); }
        _ => {
            let corners = [r.min, Pos2::new(r.max.x,r.min.y), r.max, Pos2::new(r.min.x,r.max.y)];
            for i in 0..4 { draw_dashed_line(painter, corners[i], corners[(i+1)%4], stroke, style); }
        }
    }
}

// ── Pixel-level drawing ───────────────────────────────────────────────────────

// 5×7 bitmap font for printable ASCII (chars 32..=126).
// Each glyph is [u8; 7] where each u8 encodes a row of 5 bits (MSB = leftmost pixel).
const BITMAP_FONT: [[u8; 7]; 95] = [
    // 32 ' ' (space)
    [0x00,0x00,0x00,0x00,0x00,0x00,0x00],
    // 33 '!'
    [0x04,0x04,0x04,0x04,0x04,0x00,0x04],
    // 34 '"'
    [0x0A,0x0A,0x0A,0x00,0x00,0x00,0x00],
    // 35 '#'
    [0x0A,0x0A,0x1F,0x0A,0x1F,0x0A,0x0A],
    // 36 '$'
    [0x04,0x0F,0x14,0x0E,0x05,0x1E,0x04],
    // 37 '%'
    [0x18,0x19,0x02,0x04,0x08,0x13,0x03],
    // 38 '&'
    [0x0C,0x12,0x14,0x08,0x15,0x12,0x0D],
    // 39 '''
    [0x04,0x04,0x08,0x00,0x00,0x00,0x00],
    // 40 '('
    [0x02,0x04,0x08,0x08,0x08,0x04,0x02],
    // 41 ')'
    [0x08,0x04,0x02,0x02,0x02,0x04,0x08],
    // 42 '*'
    [0x00,0x04,0x15,0x0E,0x15,0x04,0x00],
    // 43 '+'
    [0x00,0x04,0x04,0x1F,0x04,0x04,0x00],
    // 44 ','
    [0x00,0x00,0x00,0x00,0x00,0x04,0x08],
    // 45 '-'
    [0x00,0x00,0x00,0x1F,0x00,0x00,0x00],
    // 46 '.'
    [0x00,0x00,0x00,0x00,0x00,0x00,0x04],
    // 47 '/'
    [0x00,0x01,0x02,0x04,0x08,0x10,0x00],
    // 48 '0'
    [0x0E,0x11,0x13,0x15,0x19,0x11,0x0E],
    // 49 '1'
    [0x04,0x0C,0x04,0x04,0x04,0x04,0x0E],
    // 50 '2'
    [0x0E,0x11,0x01,0x02,0x04,0x08,0x1F],
    // 51 '3'
    [0x1F,0x02,0x04,0x02,0x01,0x11,0x0E],
    // 52 '4'
    [0x02,0x06,0x0A,0x12,0x1F,0x02,0x02],
    // 53 '5'
    [0x1F,0x10,0x1E,0x01,0x01,0x11,0x0E],
    // 54 '6'
    [0x06,0x08,0x10,0x1E,0x11,0x11,0x0E],
    // 55 '7'
    [0x1F,0x01,0x02,0x04,0x08,0x08,0x08],
    // 56 '8'
    [0x0E,0x11,0x11,0x0E,0x11,0x11,0x0E],
    // 57 '9'
    [0x0E,0x11,0x11,0x0F,0x01,0x02,0x0C],
    // 58 ':'
    [0x00,0x00,0x04,0x00,0x04,0x00,0x00],
    // 59 ';'
    [0x00,0x00,0x04,0x00,0x04,0x04,0x08],
    // 60 '<'
    [0x02,0x04,0x08,0x10,0x08,0x04,0x02],
    // 61 '='
    [0x00,0x00,0x1F,0x00,0x1F,0x00,0x00],
    // 62 '>'
    [0x08,0x04,0x02,0x01,0x02,0x04,0x08],
    // 63 '?'
    [0x0E,0x11,0x01,0x02,0x04,0x00,0x04],
    // 64 '@'
    [0x0E,0x11,0x17,0x15,0x17,0x10,0x0E],
    // 65 'A'
    [0x0E,0x11,0x11,0x1F,0x11,0x11,0x11],
    // 66 'B'
    [0x1E,0x11,0x11,0x1E,0x11,0x11,0x1E],
    // 67 'C'
    [0x0E,0x11,0x10,0x10,0x10,0x11,0x0E],
    // 68 'D'
    [0x1C,0x12,0x11,0x11,0x11,0x12,0x1C],
    // 69 'E'
    [0x1F,0x10,0x10,0x1E,0x10,0x10,0x1F],
    // 70 'F'
    [0x1F,0x10,0x10,0x1E,0x10,0x10,0x10],
    // 71 'G'
    [0x0E,0x11,0x10,0x17,0x11,0x11,0x0F],
    // 72 'H'
    [0x11,0x11,0x11,0x1F,0x11,0x11,0x11],
    // 73 'I'
    [0x0E,0x04,0x04,0x04,0x04,0x04,0x0E],
    // 74 'J'
    [0x07,0x02,0x02,0x02,0x02,0x12,0x0C],
    // 75 'K'
    [0x11,0x12,0x14,0x18,0x14,0x12,0x11],
    // 76 'L'
    [0x10,0x10,0x10,0x10,0x10,0x10,0x1F],
    // 77 'M'
    [0x11,0x1B,0x15,0x15,0x11,0x11,0x11],
    // 78 'N'
    [0x11,0x11,0x19,0x15,0x13,0x11,0x11],
    // 79 'O'
    [0x0E,0x11,0x11,0x11,0x11,0x11,0x0E],
    // 80 'P'
    [0x1E,0x11,0x11,0x1E,0x10,0x10,0x10],
    // 81 'Q'
    [0x0E,0x11,0x11,0x11,0x15,0x12,0x0D],
    // 82 'R'
    [0x1E,0x11,0x11,0x1E,0x14,0x12,0x11],
    // 83 'S'
    [0x0F,0x10,0x10,0x0E,0x01,0x01,0x1E],
    // 84 'T'
    [0x1F,0x04,0x04,0x04,0x04,0x04,0x04],
    // 85 'U'
    [0x11,0x11,0x11,0x11,0x11,0x11,0x0E],
    // 86 'V'
    [0x11,0x11,0x11,0x11,0x11,0x0A,0x04],
    // 87 'W'
    [0x11,0x11,0x11,0x15,0x15,0x15,0x0A],
    // 88 'X'
    [0x11,0x11,0x0A,0x04,0x0A,0x11,0x11],
    // 89 'Y'
    [0x11,0x11,0x11,0x0A,0x04,0x04,0x04],
    // 90 'Z'
    [0x1F,0x01,0x02,0x04,0x08,0x10,0x1F],
    // 91 '['
    [0x0E,0x08,0x08,0x08,0x08,0x08,0x0E],
    // 92 '\'
    [0x00,0x10,0x08,0x04,0x02,0x01,0x00],
    // 93 ']'
    [0x0E,0x02,0x02,0x02,0x02,0x02,0x0E],
    // 94 '^'
    [0x04,0x0A,0x11,0x00,0x00,0x00,0x00],
    // 95 '_'
    [0x00,0x00,0x00,0x00,0x00,0x00,0x1F],
    // 96 '`'
    [0x08,0x04,0x02,0x00,0x00,0x00,0x00],
    // 97 'a'
    [0x00,0x00,0x0E,0x01,0x0F,0x11,0x0F],
    // 98 'b'
    [0x10,0x10,0x16,0x19,0x11,0x11,0x1E],
    // 99 'c'
    [0x00,0x00,0x0E,0x10,0x10,0x11,0x0E],
    // 100 'd'
    [0x01,0x01,0x0D,0x13,0x11,0x11,0x0F],
    // 101 'e'
    [0x00,0x00,0x0E,0x11,0x1F,0x10,0x0E],
    // 102 'f'
    [0x06,0x09,0x08,0x1C,0x08,0x08,0x08],
    // 103 'g'
    [0x00,0x0F,0x11,0x11,0x0F,0x01,0x0E],
    // 104 'h'
    [0x10,0x10,0x16,0x19,0x11,0x11,0x11],
    // 105 'i'
    [0x04,0x00,0x0C,0x04,0x04,0x04,0x0E],
    // 106 'j'
    [0x02,0x00,0x06,0x02,0x02,0x12,0x0C],
    // 107 'k'
    [0x10,0x10,0x12,0x14,0x18,0x14,0x12],
    // 108 'l'
    [0x0C,0x04,0x04,0x04,0x04,0x04,0x0E],
    // 109 'm'
    [0x00,0x00,0x1A,0x15,0x15,0x11,0x11],
    // 110 'n'
    [0x00,0x00,0x16,0x19,0x11,0x11,0x11],
    // 111 'o'
    [0x00,0x00,0x0E,0x11,0x11,0x11,0x0E],
    // 112 'p'
    [0x00,0x00,0x1E,0x11,0x1E,0x10,0x10],
    // 113 'q'
    [0x00,0x00,0x0D,0x13,0x0F,0x01,0x01],
    // 114 'r'
    [0x00,0x00,0x16,0x19,0x10,0x10,0x10],
    // 115 's'
    [0x00,0x00,0x0E,0x10,0x0E,0x01,0x1E],
    // 116 't'
    [0x08,0x08,0x1C,0x08,0x08,0x09,0x06],
    // 117 'u'
    [0x00,0x00,0x11,0x11,0x11,0x13,0x0D],
    // 118 'v'
    [0x00,0x00,0x11,0x11,0x11,0x0A,0x04],
    // 119 'w'
    [0x00,0x00,0x11,0x11,0x15,0x15,0x0A],
    // 120 'x'
    [0x00,0x00,0x11,0x0A,0x04,0x0A,0x11],
    // 121 'y'
    [0x00,0x00,0x11,0x11,0x0F,0x01,0x0E],
    // 122 'z'
    [0x00,0x00,0x1F,0x02,0x04,0x08,0x1F],
    // 123 '{'
    [0x02,0x04,0x04,0x08,0x04,0x04,0x02],
    // 124 '|'
    [0x04,0x04,0x04,0x04,0x04,0x04,0x04],
    // 125 '}'
    [0x08,0x04,0x04,0x02,0x04,0x04,0x08],
    // 126 '~'
    [0x00,0x04,0x15,0x0E,0x00,0x00,0x00],
];

fn draw_bitmap_char(buf: &mut [u8], w: usize, h: usize, ch: char, x: i32, y: i32, color: Color32, scale: i32) {
    let idx = ch as usize;
    let glyph = if idx >= 32 && idx <= 126 {
        &BITMAP_FONT[idx - 32]
    } else {
        &BITMAP_FONT[0] // fallback to space for unsupported chars
    };
    for row in 0..7 {
        let bits = glyph[row];
        for col in 0..5 {
            if bits & (0x10 >> col) != 0 {
                // Draw a scale×scale block for this pixel
                for sy in 0..scale {
                    for sx in 0..scale {
                        set_pixel(buf, w, h, x + col as i32 * scale + sx, y + row as i32 * scale + sy, color);
                    }
                }
            }
        }
    }
}

fn draw_bitmap_text(buf: &mut [u8], w: usize, h: usize, text: &str, x: i32, y: i32, color: Color32, font_size: f32) {
    let scale = (font_size / 7.0).max(1.0) as i32;
    let mut cx = x;
    for ch in text.chars() {
        draw_bitmap_char(buf, w, h, ch, cx, y, color, scale);
        cx += 6 * scale; // 5 pixels + 1 gap
    }
}

pub fn render_annotation_to_buf(buf: &mut [u8], w: usize, h: usize, ann: &Annotation, base_pixels: Option<&[u8]>) {
    match ann.kind {
        ShapeKind::Rect | ShapeKind::RoundRect => {
            let (x0,y0,x1,y1)=(ann.p1.x.min(ann.p2.x) as i32, ann.p1.y.min(ann.p2.y) as i32,
                                ann.p1.x.max(ann.p2.x) as i32, ann.p1.y.max(ann.p2.y) as i32);
            let lw=ann.width as i32;
            if ann.filled { fill_rect_alpha(buf,w,h,x0,y0,x1,y1,ann.color); }
            else { for t in 0..lw { draw_rect_outline_alpha(buf,w,h,x0+t,y0+t,x1-t,y1-t,ann.color,ann.line_style); } }
        }
        ShapeKind::Ellipse => {
            let (cx,cy)=(((ann.p1.x+ann.p2.x)/2.0) as i32,((ann.p1.y+ann.p2.y)/2.0) as i32);
            let (rx,ry)=(((ann.p2.x-ann.p1.x).abs()/2.0) as i32,((ann.p2.y-ann.p1.y).abs()/2.0) as i32);
            draw_ellipse_alpha(buf,w,h,cx,cy,rx,ry,ann.color,ann.width as i32,ann.filled);
        }
        ShapeKind::Arrow => { draw_arrow_buf(buf,w,h,ann.p1,ann.p2,ann.color,ann.width as i32,false); }
        ShapeKind::ArrowDouble => { draw_arrow_buf(buf,w,h,ann.p1,ann.p2,ann.color,ann.width as i32,true); }
        ShapeKind::Line => { draw_line_thick(buf,w,h,ann.p1,ann.p2,ann.color,ann.width as i32); }
        ShapeKind::Pen => {
            for pair in ann.pen_points.windows(2) { draw_line_thick(buf,w,h,pair[0],pair[1],ann.color,ann.width as i32); }
        }
        ShapeKind::Marker => {
            let lw=(ann.width*3.0) as i32;
            for pair in ann.pen_points.windows(2) { draw_line_thick(buf,w,h,pair[0],pair[1],ann.color,lw); }
        }
        ShapeKind::Mosaic => {
            let (x0,y0,x1,y1)=(ann.p1.x.min(ann.p2.x) as i32, ann.p1.y.min(ann.p2.y) as i32,
                                ann.p1.x.max(ann.p2.x) as i32, ann.p1.y.max(ann.p2.y) as i32);
            let block=((ann.width*2.0) as i32).max(8);
            // Pre-clamp bounds once outside the loop
            let bx_end = x1.min(w as i32);
            let by_end = y1.min(h as i32);
            let mut by = y0.max(0);
            while by < by_end {
                let block_y_end = (by + block).min(by_end);
                let mut bx = x0.max(0);
                while bx < bx_end {
                    let block_x_end = (bx + block).min(bx_end);
                    let (mut sr,mut sg,mut sb,mut cnt)=(0u32,0u32,0u32,0u32);
                    for py in by..block_y_end { for px in bx..block_x_end {
                        let i=(py as usize*w+px as usize)*4;
                        sr+=buf[i] as u32; sg+=buf[i+1] as u32; sb+=buf[i+2] as u32; cnt+=1;
                    }}
                    if cnt>0 {
                        let (ar,ag,ab)=((sr/cnt) as u8,(sg/cnt) as u8,(sb/cnt) as u8);
                        for py in by..block_y_end { for px in bx..block_x_end {
                            set_pixel(buf,w,h,px,py,Color32::from_rgb(ar,ag,ab));
                        }}
                    }
                    bx+=block;
                }
                by+=block;
            }
        }
        ShapeKind::Eraser => {
            let (x0,y0,x1,y1)=(ann.p1.x.min(ann.p2.x) as i32, ann.p1.y.min(ann.p2.y) as i32,
                                ann.p1.x.max(ann.p2.x) as i32, ann.p1.y.max(ann.p2.y) as i32);
            if let Some(base) = base_pixels {
                // Restore original pixels from base image
                let cx0 = x0.max(0) as usize;
                let cy0 = y0.max(0) as usize;
                let cx1 = (x1 as usize).min(w);
                let cy1 = (y1 as usize).min(h);
                for py in cy0..cy1 {
                    for px in cx0..cx1 {
                        let i = (py * w + px) * 4;
                        if i + 4 <= base.len() && i + 4 <= buf.len() {
                            buf[i]     = base[i];
                            buf[i + 1] = base[i + 1];
                            buf[i + 2] = base[i + 2];
                            buf[i + 3] = base[i + 3];
                        }
                    }
                }
            } else {
                // Fallback: fill with white if no base pixels available
                fill_rect_alpha(buf,w,h,x0,y0,x1,y1,Color32::WHITE);
            }
        }
        ShapeKind::Text => {
            draw_bitmap_text(buf, w, h, &ann.text, ann.p1.x as i32, ann.p1.y as i32, ann.color, ann.width * 4.0);
        }
        ShapeKind::Number => {
            let cx=((ann.p1.x+ann.p2.x)/2.0) as i32; let cy=((ann.p1.y+ann.p2.y)/2.0) as i32;
            let r=((ann.p2-ann.p1).length()/2.0) as i32;
            draw_filled_circle(buf,w,h,cx,cy,r,ann.color);
            let digit_str = ann.number.to_string();
            let font_size = (r as f32 * 1.2).max(7.0);
            let scale = (font_size / 7.0).max(1.0) as i32;
            let text_w = digit_str.len() as i32 * 6 * scale;
            let text_h = 7 * scale;
            let tx = cx - text_w / 2;
            let ty = cy - text_h / 2;
            draw_bitmap_text(buf, w, h, &digit_str, tx, ty, Color32::WHITE, font_size);
        }
    }
}

#[inline]
fn set_pixel(buf: &mut [u8], w: usize, h: usize, x: i32, y: i32, c: Color32) {
    if x<0||y<0||x>=w as i32||y>=h as i32 { return; }
    let i=(y as usize*w+x as usize)*4;
    let a=c.a() as u32;
    if a==255 { buf[i]=c.r(); buf[i+1]=c.g(); buf[i+2]=c.b(); buf[i+3]=255; }
    else {
        // Alpha blend
        let ia=255-a;
        buf[i]  =((buf[i]   as u32*ia + c.r() as u32*a)/255) as u8;
        buf[i+1]=((buf[i+1] as u32*ia + c.g() as u32*a)/255) as u8;
        buf[i+2]=((buf[i+2] as u32*ia + c.b() as u32*a)/255) as u8;
        buf[i+3]=255;
    }
}
fn draw_rect_outline_alpha(buf:&mut[u8],w:usize,h:usize,x0:i32,y0:i32,x1:i32,y1:i32,c:Color32,style:LineStyle) {
    let dash=12i32; let gap=6i32;
    let draw_seg = |buf:&mut[u8], ax:i32, ay:i32, bx:i32, by:i32| {
        let len=((bx-ax).abs()+(by-ay).abs()).max(1);
        let mut t=0i32;
        while t<len {
            let on = match style { LineStyle::Solid=>true, LineStyle::Dashed=>t%(dash+gap)<dash, LineStyle::Dotted=>t%(gap)==0 };
            if on {
                let x=ax+(bx-ax)*t/len; let y=ay+(by-ay)*t/len;
                set_pixel(buf,w,h,x,y,c);
            }
            t+=1;
        }
    };
    for x in x0..=x1 { draw_seg(buf,x,y0,x,y0); draw_seg(buf,x,y1,x,y1); }
    for y in y0..=y1 { draw_seg(buf,x0,y,x0,y); draw_seg(buf,x1,y,x1,y); }
}
fn fill_rect_alpha(buf:&mut[u8],w:usize,h:usize,x0:i32,y0:i32,x1:i32,y1:i32,c:Color32) {
    for y in y0..=y1 { for x in x0..=x1 { set_pixel(buf,w,h,x,y,c); } }
}
fn draw_ellipse_alpha(buf:&mut[u8],w:usize,h:usize,cx:i32,cy:i32,rx:i32,ry:i32,c:Color32,lw:i32,filled:bool) {
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
fn draw_filled_circle(buf:&mut[u8],w:usize,h:usize,cx:i32,cy:i32,r:i32,c:Color32) {
    for y in (cy-r)..=(cy+r) { for x in (cx-r)..=(cx+r) {
        if (x-cx)*(x-cx)+(y-cy)*(y-cy)<=r*r { set_pixel(buf,w,h,x,y,c); }
    }}
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
fn draw_arrow_buf(buf:&mut[u8],w:usize,h:usize,p1:Pos2,p2:Pos2,c:Color32,lw:i32,double_headed:bool) {
    draw_line_thick(buf,w,h,p1,p2,c,lw);
    let (dx,dy)=(p2.x-p1.x,p2.y-p1.y);
    let len=(dx*dx+dy*dy).sqrt().max(1.0);
    let (ux,uy)=(dx/len,dy/len);
    let (head,angle)=(18.0_f32,0.4_f32);
    let ax1=Pos2::new(p2.x-head*(ux*angle.cos()-uy*angle.sin()),p2.y-head*(ux*angle.sin()+uy*angle.cos()));
    let ax2=Pos2::new(p2.x-head*(ux*angle.cos()+uy*angle.sin()),p2.y-head*(-ux*angle.sin()+uy*angle.cos()));
    draw_line_thick(buf,w,h,p2,ax1,c,lw);
    draw_line_thick(buf,w,h,p2,ax2,c,lw);
    if double_headed {
        let bx1=Pos2::new(p1.x+head*(ux*angle.cos()-uy*angle.sin()),p1.y+head*(ux*angle.sin()+uy*angle.cos()));
        let bx2=Pos2::new(p1.x+head*(ux*angle.cos()+uy*angle.sin()),p1.y+head*(-ux*angle.sin()+uy*angle.cos()));
        draw_line_thick(buf,w,h,p1,bx1,c,lw);
        draw_line_thick(buf,w,h,p1,bx2,c,lw);
    }
}


// ── Bug condition exploration tests ───────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use egui::{Color32, Pos2};

    /// Helper: create a w×h RGBA buffer filled with a single color.
    fn make_filled_buf(w: usize, h: usize, c: Color32) -> Vec<u8> {
        let mut buf = vec![0u8; w * h * 4];
        for i in 0..(w * h) {
            buf[i * 4]     = c.r();
            buf[i * 4 + 1] = c.g();
            buf[i * 4 + 2] = c.b();
            buf[i * 4 + 3] = c.a();
        }
        buf
    }

    /// Bug exploration: Eraser fills white instead of restoring original pixels.
    ///
    /// We create a 100×100 red buffer, apply an eraser annotation over a region,
    /// then assert the erased pixels still match the original red. On unfixed code
    /// this FAILS because the eraser fills with white [255,255,255,255].
    ///
    /// **Validates: Requirements 1.1**
    #[test]
    fn test_eraser_fills_white_bug() {
        let w = 100usize;
        let h = 100usize;
        let red = Color32::from_rgb(255, 0, 0);

        // Create a red-filled buffer (simulating a red screenshot)
        let mut buf = make_filled_buf(w, h, red);

        // Create eraser annotation covering region (10,10) to (30,30)
        let ann = Annotation::new(
            ShapeKind::Eraser,
            Color32::WHITE,
            10.0,
            false,
            LineStyle::Solid,
            Pos2::new(10.0, 10.0),
            Pos2::new(30.0, 30.0),
        );

        // Keep a copy of the original red buffer as base_pixels
        let base_pixels = buf.clone();

        // Render the eraser on the buffer with base_pixels for restoration
        render_annotation_to_buf(&mut buf, w, h, &ann, Some(base_pixels.as_slice()));

        // Assert: erased pixels should NOT be white — they should be the original red.
        // On unfixed code, the eraser fills with white, so this assertion FAILS.
        let mut found_white = false;
        for y in 10..=30 {
            for x in 10..=30 {
                let i = (y * w + x) * 4;
                if buf[i] == 255 && buf[i + 1] == 255 && buf[i + 2] == 255 && buf[i + 3] == 255 {
                    found_white = true;
                }
            }
        }
        assert!(
            !found_white,
            "BUG CONFIRMED: Eraser filled pixels with white instead of restoring original red pixels"
        );
    }

    /// Bug exploration: Text annotations are not rendered to the export buffer.
    ///
    /// We create a 100×100 black buffer, apply a text annotation "A", then assert
    /// that at least some pixels in the text region differ from black. On unfixed
    /// code this FAILS because the Text arm is a no-op.
    ///
    /// **Validates: Requirements 1.2**
    #[test]
    fn test_text_not_rendered_bug() {
        let w = 100usize;
        let h = 100usize;
        let black = Color32::from_rgb(0, 0, 0);

        let mut buf = make_filled_buf(w, h, black);
        let snapshot_before = buf.clone();

        // Create text annotation "A" in red at position (10,10)-(50,50)
        let mut ann = Annotation::new(
            ShapeKind::Text,
            Color32::from_rgb(255, 0, 0),
            16.0,
            false,
            LineStyle::Solid,
            Pos2::new(10.0, 10.0),
            Pos2::new(50.0, 50.0),
        );
        ann.text = "A".to_string();

        render_annotation_to_buf(&mut buf, w, h, &ann, None);

        // Assert: at least some pixels should have changed (text was rendered).
        // On unfixed code, NO pixels change because the Text arm is empty.
        let pixels_changed = buf.iter()
            .zip(snapshot_before.iter())
            .filter(|(a, b)| a != b)
            .count();

        assert!(
            pixels_changed > 0,
            "BUG CONFIRMED: Text annotation produced zero pixel changes — text is not rendered to export buffer"
        );
    }

    /// Bug exploration: Number annotations render the circle but not the digit.
    ///
    /// We create a 100×100 black buffer, apply a number annotation with number=3,
    /// then check for white pixels in the center area of the circle (the digit).
    /// On unfixed code this FAILS because no digit is rendered — only the colored
    /// circle background.
    ///
    /// **Validates: Requirements 1.3**
    #[test]
    fn test_number_no_digit_bug() {
        let w = 100usize;
        let h = 100usize;
        let black = Color32::from_rgb(0, 0, 0);

        let mut buf = make_filled_buf(w, h, black);

        // Create number annotation with number=3, blue circle
        // p1=(30,30), p2=(60,60) → center=(45,45), radius≈21
        let mut ann = Annotation::new(
            ShapeKind::Number,
            Color32::from_rgb(0, 0, 255),
            10.0,
            false,
            LineStyle::Solid,
            Pos2::new(30.0, 30.0),
            Pos2::new(60.0, 60.0),
        );
        ann.number = 3;

        render_annotation_to_buf(&mut buf, w, h, &ann, None);

        // Check for white pixels in the center area (where the digit "3" should be).
        // The circle center is at (45,45). Check a small region around center for
        // white pixels that would form the digit.
        let mut white_pixel_count = 0;
        for y in 40..=50 {
            for x in 40..=50 {
                let i = (y * w + x) * 4;
                if buf[i] == 255 && buf[i + 1] == 255 && buf[i + 2] == 255 && buf[i + 3] == 255 {
                    white_pixel_count += 1;
                }
            }
        }

        assert!(
            white_pixel_count > 0,
            "BUG CONFIRMED: Number annotation has no white digit pixels in center — digit '3' is not rendered"
        );
    }

    // ── Preservation property tests ───────────────────────────────────────────
    // These tests verify that existing (non-buggy) annotation rendering works
    // correctly and will continue to work after the bugfix.
    //
    // **Validates: Requirements 3.1, 3.2, 3.6**

    /// Preservation: Rect rendering produces expected red pixels in the filled region.
    #[test]
    fn test_rect_rendering_preserved() {
        let w = 100usize;
        let h = 100usize;
        let black = Color32::from_rgb(0, 0, 0);
        let red = Color32::from_rgb(255, 0, 0);

        let mut buf = make_filled_buf(w, h, black);

        // Filled red rectangle from (10,10) to (50,50)
        let ann = Annotation::new(
            ShapeKind::Rect,
            red,
            2.0,
            true,
            LineStyle::Solid,
            Pos2::new(10.0, 10.0),
            Pos2::new(50.0, 50.0),
        );

        render_annotation_to_buf(&mut buf, w, h, &ann, None);

        // Verify pixels inside the rect are red
        for y in 10..=50 {
            for x in 10..=50 {
                let i = (y * w + x) * 4;
                assert_eq!(buf[i], 255, "Rect pixel ({x},{y}) R channel should be 255");
                assert_eq!(buf[i + 1], 0, "Rect pixel ({x},{y}) G channel should be 0");
                assert_eq!(buf[i + 2], 0, "Rect pixel ({x},{y}) B channel should be 0");
                assert_eq!(buf[i + 3], 255, "Rect pixel ({x},{y}) A channel should be 255");
            }
        }

        // Verify a pixel outside the rect is still black
        let i = (0 * w + 0) * 4;
        assert_eq!(buf[i], 0, "Pixel outside rect should remain black");
        assert_eq!(buf[i + 1], 0);
        assert_eq!(buf[i + 2], 0);
    }

    /// Preservation: Ellipse rendering produces expected blue pixels at the center.
    #[test]
    fn test_ellipse_rendering_preserved() {
        let w = 100usize;
        let h = 100usize;
        let black = Color32::from_rgb(0, 0, 0);
        let blue = Color32::from_rgb(0, 0, 255);

        let mut buf = make_filled_buf(w, h, black);

        // Filled blue ellipse: p1=(30,30), p2=(70,70) → center=(50,50), rx=20, ry=20
        let ann = Annotation::new(
            ShapeKind::Ellipse,
            blue,
            2.0,
            true,
            LineStyle::Solid,
            Pos2::new(30.0, 30.0),
            Pos2::new(70.0, 70.0),
        );

        render_annotation_to_buf(&mut buf, w, h, &ann, None);

        // The center pixel (50,50) must be blue
        let i = (50 * w + 50) * 4;
        assert_eq!(buf[i], 0, "Ellipse center R should be 0");
        assert_eq!(buf[i + 1], 0, "Ellipse center G should be 0");
        assert_eq!(buf[i + 2], 255, "Ellipse center B should be 255");
        assert_eq!(buf[i + 3], 255, "Ellipse center A should be 255");

        // A pixel well outside the ellipse should still be black
        let i_out = (0 * w + 0) * 4;
        assert_eq!(buf[i_out], 0, "Pixel outside ellipse should remain black");
    }

    /// Preservation: Line rendering modifies pixels along the diagonal.
    #[test]
    fn test_line_rendering_preserved() {
        let w = 100usize;
        let h = 100usize;
        let black = Color32::from_rgb(0, 0, 0);
        let green = Color32::from_rgb(0, 255, 0);

        let mut buf = make_filled_buf(w, h, black);
        let snapshot_before = buf.clone();

        // Green line from (0,0) to (99,99) with width 3
        let ann = Annotation::new(
            ShapeKind::Line,
            green,
            3.0,
            false,
            LineStyle::Solid,
            Pos2::new(0.0, 0.0),
            Pos2::new(99.0, 99.0),
        );

        render_annotation_to_buf(&mut buf, w, h, &ann, None);

        // Verify that diagonal pixels have changed (line was drawn)
        let mut changed_count = 0;
        for d in 0..100 {
            let i = (d * w + d) * 4;
            if buf[i] != snapshot_before[i]
                || buf[i + 1] != snapshot_before[i + 1]
                || buf[i + 2] != snapshot_before[i + 2]
            {
                changed_count += 1;
            }
        }
        assert!(
            changed_count > 50,
            "Line should modify most diagonal pixels, but only {changed_count} changed"
        );

        // Verify at least one diagonal pixel is green
        let i_mid = (50 * w + 50) * 4;
        assert_eq!(buf[i_mid + 1], 255, "Diagonal pixel (50,50) G channel should be 255 (green)");
    }

    /// Preservation: Mosaic rendering pixelates the region (pixels in a block become uniform).
    #[test]
    fn test_mosaic_rendering_preserved() {
        let w = 100usize;
        let h = 100usize;

        // Create a buffer with varied pixel content (gradient pattern)
        let mut buf = vec![0u8; w * h * 4];
        for y in 0..h {
            for x in 0..w {
                let i = (y * w + x) * 4;
                buf[i]     = (x * 255 / w) as u8;     // R varies by x
                buf[i + 1] = (y * 255 / h) as u8;     // G varies by y
                buf[i + 2] = 128;                       // B constant
                buf[i + 3] = 255;                       // A opaque
            }
        }
        let snapshot_before = buf.clone();

        // Mosaic annotation over region (20,20) to (60,60) with width=5 → block size = max(10, 8) = 10
        let ann = Annotation::new(
            ShapeKind::Mosaic,
            Color32::WHITE, // color is unused for mosaic
            5.0,
            false,
            LineStyle::Solid,
            Pos2::new(20.0, 20.0),
            Pos2::new(60.0, 60.0),
        );

        render_annotation_to_buf(&mut buf, w, h, &ann, None);

        // Verify pixelation occurred: within a mosaic block, all pixels should be uniform.
        // Block size = max(5.0*2.0, 8) = 10. First block starts at (20,20), ends at (29,29).
        let block_x0 = 20usize;
        let block_y0 = 20usize;
        let block_x1 = 29usize;
        let block_y1 = 29usize;

        // All pixels in this block should have the same color
        let ref_i = (block_y0 * w + block_x0) * 4;
        let (ref_r, ref_g, ref_b) = (buf[ref_i], buf[ref_i + 1], buf[ref_i + 2]);

        for y in block_y0..=block_y1 {
            for x in block_x0..=block_x1 {
                let i = (y * w + x) * 4;
                assert_eq!(buf[i], ref_r, "Mosaic block pixel ({x},{y}) R should be uniform");
                assert_eq!(buf[i + 1], ref_g, "Mosaic block pixel ({x},{y}) G should be uniform");
                assert_eq!(buf[i + 2], ref_b, "Mosaic block pixel ({x},{y}) B should be uniform");
            }
        }

        // Verify the mosaic actually changed pixels (the gradient was averaged)
        let mut changed = false;
        for y in block_y0..=block_y1 {
            for x in block_x0..=block_x1 {
                let i = (y * w + x) * 4;
                if buf[i] != snapshot_before[i] || buf[i + 1] != snapshot_before[i + 1] {
                    changed = true;
                    break;
                }
            }
            if changed { break; }
        }
        assert!(changed, "Mosaic should have changed at least some pixels from the original gradient");

        // Verify pixels outside the mosaic region are unchanged
        let i_out = (0 * w + 0) * 4;
        assert_eq!(buf[i_out], snapshot_before[i_out], "Pixel outside mosaic should be unchanged");
    }
}
