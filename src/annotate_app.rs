use eframe::egui;
use egui::{Color32, ColorImage, Key, Pos2, Rect, Stroke, TextureHandle, Vec2};

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
struct HistoryEntry { pixels: Vec<u8>, w: usize, h: usize }

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
    /// Whether palette editor is open
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

    capture_rx: Option<std::sync::mpsc::Receiver<CaptureResult>>,
    pub capture_btn_clicked: bool,
    status: String,
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
            tool_colors, palette: Palette::load(), show_palette_editor: false,
            history: Vec::new(), history_idx: None,
            hotkey: HotkeyConfig::load(), editing_hotkey: false, hotkey_changed: false,
            smart_rect: None,
            #[cfg(target_os = "windows")] own_hwnds: Vec::new(),
            #[cfg(target_os = "windows")] mouse_hook: None,
            capture_rx: None, capture_btn_clicked: false,
            status: String::new(),
        }
    }
}

impl AnnotateApp {
    pub fn is_selecting(&self) -> bool { self.capture_state == CaptureState::Selecting }
    pub fn trigger_capture(&mut self) {
        if self.capture_state == CaptureState::Idle { self.capture_btn_clicked = true; }
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
                if let Some(screen) = screens.into_iter().next() {
                    if let Ok(img) = screen.capture() {
                        let w = img.width() as usize; let h = img.height() as usize;
                        let _ = tx.send(CaptureResult { pixels: img.into_raw(), width: w, height: h });
                    }
                }
            }
        });
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
            for ann in &self.annotations { render_annotation_to_buf(&mut buf, w, h, ann); }
            self.baked=Some(buf);
        }
    }
    fn bake_last(&mut self) {
        if let (Some(ref mut baked), Some(ann)) = (&mut self.baked, self.annotations.last()) {
            let (w,h)=(self.img_w,self.img_h);
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
        if self.capture_state == CaptureState::Idle && !self.editing_hotkey {
            if self.hotkey.matches(ctx) { return true; }
        }

        if self.capture_state == CaptureState::Selecting {
            if ctx.input(|i| i.key_pressed(Key::Escape)) {
                #[cfg(target_os = "windows")] { self.mouse_hook = None; }
                self.capture_state=CaptureState::Idle;
                self.full_pixels=None; self.full_texture=None; self.smart_rect=None;
                self.status=String::new();
                ctx.send_viewport_cmd(egui::ViewportCommand::Fullscreen(false));
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
                { self.smart_rect = crate::win_capture::hovered_window_rect(self.cursor_px.0, self.cursor_px.1, &self.own_hwnds); }
            }
            if hook_clicked && self.sel_start.is_none() && self.smart_rect.is_some() {
                #[cfg(target_os = "windows")]
                crate::win_capture::LowLevelMouseHook::consume_click();
                ctx.send_viewport_cmd(egui::ViewportCommand::Fullscreen(false));
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
                    ctx.send_viewport_cmd(egui::ViewportCommand::Fullscreen(false));
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
            egui::ScrollArea::both().auto_shrink(false).show(ui, |ui| {
                let img_size = Vec2::new(self.img_w as f32, self.img_h as f32);
                if let Some(ref tex) = self.texture {
                    let (cr, resp) = ui.allocate_exact_size(img_size, egui::Sense::drag());
                    let painter = ui.painter_at(cr);
                    painter.image(tex.id(), cr, Rect::from_min_max(Pos2::ZERO, Pos2::new(1.0,1.0)), Color32::WHITE);
                    let to_img    = |p: Pos2| Pos2::new((p.x-cr.min.x).clamp(0.0,img_size.x),(p.y-cr.min.y).clamp(0.0,img_size.y));
                    let to_screen = |p: Pos2| Pos2::new(cr.min.x+p.x, cr.min.y+p.y);
                    for ann in &self.annotations { draw_annotation_egui(&painter, ann, 1.0, &to_screen); }

                    // Scroll wheel: Ctrl = opacity, else = stroke width
                    let scroll = ui.input(|i| i.smooth_scroll_delta.y);
                    if resp.hovered() && scroll.abs()>0.1 {
                        if ui.input(|i| i.modifiers.ctrl) {
                            let a = self.color.a();
                            let new_a = (a as f32 + scroll*5.0).clamp(20.0,255.0) as u8;
                            self.color = Color32::from_rgba_unmultiplied(self.color.r(),self.color.g(),self.color.b(),new_a);
                            self.save_tool_color();
                        } else {
                            self.stroke_w = (self.stroke_w + scroll*0.3).clamp(1.0,30.0);
                        }
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
                        // Custom palette
                        let palette_colors = self.palette.colors();
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

pub fn render_annotation_to_buf(buf: &mut [u8], w: usize, h: usize, ann: &Annotation) {
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
            let mut by=y0;
            while by<y1 {
                let mut bx=x0;
                while bx<x1 {
                    // Average color in block
                    let (mut sr,mut sg,mut sb,mut cnt)=(0u32,0u32,0u32,0u32);
                    for py in by..(by+block).min(y1) { for px in bx..(bx+block).min(x1) {
                        if px>=0&&py>=0&&(px as usize)<w&&(py as usize)<h {
                            let i=(py as usize*w+px as usize)*4;
                            sr+=buf[i] as u32; sg+=buf[i+1] as u32; sb+=buf[i+2] as u32; cnt+=1;
                        }
                    }}
                    if cnt>0 {
                        let (ar,ag,ab)=((sr/cnt) as u8,(sg/cnt) as u8,(sb/cnt) as u8);
                        for py in by..(by+block).min(y1) { for px in bx..(bx+block).min(x1) {
                            set_pixel(buf,w,h,px,py,Color32::from_rgb(ar,ag,ab));
                        }}
                    }
                    bx+=block;
                }
                by+=block;
            }
        }
        ShapeKind::Eraser => {
            // Eraser: restore original pixels (we don't have original here, use white)
            let (x0,y0,x1,y1)=(ann.p1.x.min(ann.p2.x) as i32, ann.p1.y.min(ann.p2.y) as i32,
                                ann.p1.x.max(ann.p2.x) as i32, ann.p1.y.max(ann.p2.y) as i32);
            fill_rect_alpha(buf,w,h,x0,y0,x1,y1,Color32::WHITE);
        }
        ShapeKind::Text => {
            // Text rendering to pixel buffer is complex; skip (egui handles display)
        }
        ShapeKind::Number => {
            let cx=((ann.p1.x+ann.p2.x)/2.0) as i32; let cy=((ann.p1.y+ann.p2.y)/2.0) as i32;
            let r=((ann.p2-ann.p1).length()/2.0) as i32;
            draw_filled_circle(buf,w,h,cx,cy,r,ann.color);
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
