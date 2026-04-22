//! Windows-specific capture helpers.
//!
//! Provides:
//! - `enum_visible_windows`  — enumerate all visible top-level windows with their rects
//! - `hovered_window_rect`   — find the deepest visible window/control under the cursor
//! - `capture_rect_gdi`      — capture an arbitrary screen rect via GDI BitBlt (faster
//!                             than screenshots crate for sub-regions, handles layered
//!                             windows and full-screen apps correctly)
//! - `LowLevelMouseHook`     — install WH_MOUSE_LL for precise pointer events outside
//!                             the egui window (used during the selection overlay)

#![cfg(target_os = "windows")]

use std::sync::{Arc, Mutex};

// ── Win32 imports ─────────────────────────────────────────────────────────────

use windows::Win32::Foundation::{BOOL, HWND, LPARAM, POINT, RECT, TRUE};
use windows::Win32::Graphics::Gdi::{
    BitBlt, CreateCompatibleBitmap, CreateCompatibleDC, DeleteDC, DeleteObject,
    GetDC, ReleaseDC, SelectObject, SRCCOPY, HBITMAP, HDC,
};
use windows::Win32::UI::WindowsAndMessaging::{
    CallNextHookEx, EnumWindows, GetWindowRect, GetWindowTextW,
    IsIconic, IsWindowVisible, RealChildWindowFromPoint, SetWindowsHookExW,
    UnhookWindowsHookEx, WindowFromPoint, HHOOK, MSLLHOOKSTRUCT,
    WH_MOUSE_LL, WM_LBUTTONDOWN,
    GWL_EXSTYLE, WS_EX_LAYERED, WS_EX_TOOLWINDOW,
    WS_EX_TRANSPARENT,
};

// ── Window info ───────────────────────────────────────────────────────────────

#[derive(Clone, Debug)]
pub struct WindowInfo {
    pub hwnd:  isize,
    pub rect:  [i32; 4], // left, top, right, bottom
    pub title: String,
    pub is_fullscreen: bool,
    pub is_layered:    bool,
}

impl WindowInfo {
    pub fn width(&self)  -> i32 { self.rect[2] - self.rect[0] }
    pub fn height(&self) -> i32 { self.rect[3] - self.rect[1] }
}

/// Enumerate all visible, non-minimised top-level windows.
/// Filters out tool windows, transparent overlays, and zero-size windows.
pub fn enum_visible_windows() -> Vec<WindowInfo> {
    let results: Arc<Mutex<Vec<WindowInfo>>> = Arc::new(Mutex::new(Vec::new()));
    let results_clone = results.clone();

    unsafe {
        let ptr = Arc::into_raw(results_clone) as isize;
        let _ = EnumWindows(Some(enum_callback), LPARAM(ptr));
        // Reconstruct Arc to drop it properly
        let _ = Arc::from_raw(ptr as *const Mutex<Vec<WindowInfo>>);
    }

    Arc::try_unwrap(results).unwrap_or_default().into_inner().unwrap_or_default()
}

unsafe extern "system" fn enum_callback(hwnd: HWND, lparam: LPARAM) -> BOOL {
    if IsWindowVisible(hwnd).as_bool() && !IsIconic(hwnd).as_bool() {
        let mut rect = RECT::default();
        if GetWindowRect(hwnd, &mut rect).is_ok() {
            let w = rect.right  - rect.left;
            let h = rect.bottom - rect.top;
            if w <= 0 || h <= 0 { return TRUE; }

            // Get extended style to filter tool/transparent windows
            use windows::Win32::UI::WindowsAndMessaging::GetWindowLongPtrW;
            let ex_style = GetWindowLongPtrW(hwnd, GWL_EXSTYLE) as u32;
            if ex_style & WS_EX_TOOLWINDOW.0 != 0 { return TRUE; }
            if ex_style & WS_EX_TRANSPARENT.0 != 0 { return TRUE; }

            // Get title (empty-title windows are usually system/background)
            let mut buf = [0u16; 256];
            let len = GetWindowTextW(hwnd, &mut buf);
            let title = String::from_utf16_lossy(&buf[..len as usize]);

            // Detect full-screen: covers entire primary monitor
            let is_fullscreen = rect.left == 0 && rect.top == 0
                && w >= 1920 && h >= 1080; // heuristic

            let is_layered = ex_style & WS_EX_LAYERED.0 != 0;

            let results = &*(lparam.0 as *const Mutex<Vec<WindowInfo>>);
            if let Ok(mut v) = results.lock() {
                v.push(WindowInfo {
                    hwnd: hwnd.0 as isize,
                    rect: [rect.left, rect.top, rect.right, rect.bottom],
                    title,
                    is_fullscreen,
                    is_layered,
                });
            }
        }
    }
    TRUE
}

/// Find the deepest visible window/control under the given screen point.
/// Uses `RealChildWindowFromPoint` to drill into child controls.
/// Returns the bounding rect of the best candidate.
pub fn hovered_window_rect(screen_x: i32, screen_y: i32) -> Option<[i32; 4]> {
    unsafe {
        let pt = POINT { x: screen_x, y: screen_y };
        let top = WindowFromPoint(pt);
        if top.0 == std::ptr::null_mut() { return None; }

        // Convert screen point to client coords for child detection
        let mut client_pt = pt;
        use windows::Win32::Graphics::Gdi::ScreenToClient;
        let _ = ScreenToClient(top, &mut client_pt);

        // Try to find the deepest child control
        let child = RealChildWindowFromPoint(top, client_pt);
        let target = if child.0 != std::ptr::null_mut() && child != top { child } else { top };

        let mut rect = RECT::default();
        GetWindowRect(target, &mut rect).ok()?;

        let w = rect.right  - rect.left;
        let h = rect.bottom - rect.top;
        if w <= 0 || h <= 0 { return None; }

        Some([rect.left, rect.top, rect.right, rect.bottom])
    }
}

// ── GDI capture ───────────────────────────────────────────────────────────────

/// Capture a screen rectangle using GDI BitBlt.
///
/// Advantages over `screenshots` crate for sub-regions:
/// - Handles layered (WS_EX_LAYERED) windows correctly via CAPTUREBLT
/// - Works with full-screen DirectX/OpenGL apps (uses desktop DC)
/// - No intermediate full-screen allocation — only allocates the target rect
///
/// Returns RGBA bytes (width × height × 4).
pub fn capture_rect_gdi(left: i32, top: i32, width: i32, height: i32) -> Option<Vec<u8>> {
    if width <= 0 || height <= 0 { return None; }
    unsafe {
        let screen_dc: HDC = GetDC(HWND(std::ptr::null_mut()));
        if screen_dc.is_invalid() { return None; }

        let mem_dc: HDC = CreateCompatibleDC(screen_dc);
        let bmp: HBITMAP = CreateCompatibleBitmap(screen_dc, width, height);
        let old = SelectObject(mem_dc, bmp);

        // CAPTUREBLT (0x40000000) includes layered windows
        const CAPTUREBLT: u32 = 0x40000000;
        let _ = BitBlt(mem_dc, 0, 0, width, height,
                       screen_dc, left, top, SRCCOPY | windows::Win32::Graphics::Gdi::ROP_CODE(CAPTUREBLT));

        // Read pixels via GetDIBits
        use windows::Win32::Graphics::Gdi::{GetDIBits, BITMAPINFO, BITMAPINFOHEADER, DIB_RGB_COLORS, BI_RGB};
        let mut bmi = BITMAPINFO {
            bmiHeader: BITMAPINFOHEADER {
                biSize:        std::mem::size_of::<BITMAPINFOHEADER>() as u32,
                biWidth:       width,
                biHeight:      -height, // top-down
                biPlanes:      1,
                biBitCount:    32,
                biCompression: BI_RGB.0,
                ..Default::default()
            },
            ..Default::default()
        };
        let mut bgra = vec![0u8; (width * height * 4) as usize];
        GetDIBits(mem_dc, bmp, 0, height as u32,
                  Some(bgra.as_mut_ptr() as *mut _), &mut bmi, DIB_RGB_COLORS);

        // Convert BGRA → RGBA
        for chunk in bgra.chunks_exact_mut(4) {
            chunk.swap(0, 2); // B↔R
            chunk[3] = 255;
        }

        SelectObject(mem_dc, old);
        DeleteObject(bmp);
        DeleteDC(mem_dc);
        ReleaseDC(HWND(std::ptr::null_mut()), screen_dc);

        Some(bgra)
    }
}

// ── Low-level mouse hook ──────────────────────────────────────────────────────

/// Shared state written by the hook, read by the UI thread.
#[derive(Default, Clone)]
pub struct MouseState {
    /// Current cursor position in screen pixels
    pub pos:      (i32, i32),
    /// Left button just pressed (cleared after read)
    pub clicked:  bool,
}

static HOOK_STATE: Mutex<Option<MouseState>> = Mutex::new(None);

pub struct LowLevelMouseHook {
    hhook: HHOOK,
}

impl LowLevelMouseHook {
    /// Install WH_MOUSE_LL. Must be called from a thread with a message loop,
    /// or from the main thread (egui's event loop satisfies this on Windows).
    pub fn install() -> Option<Self> {
        unsafe {
            *HOOK_STATE.lock().unwrap() = Some(MouseState::default());
            let h = SetWindowsHookExW(WH_MOUSE_LL, Some(ll_mouse_proc), None, 0).ok()?;
            Some(Self { hhook: h })
        }
    }

    /// Read and clear the current mouse state.
    pub fn poll() -> MouseState {
        HOOK_STATE.lock().unwrap().clone().unwrap_or_default()
    }

    /// Clear the clicked flag after consuming it.
    pub fn consume_click() {
        if let Ok(mut g) = HOOK_STATE.lock() {
            if let Some(ref mut s) = *g { s.clicked = false; }
        }
    }
}

impl Drop for LowLevelMouseHook {
    fn drop(&mut self) {
        unsafe { let _ = UnhookWindowsHookEx(self.hhook); }
        *HOOK_STATE.lock().unwrap() = None;
    }
}

unsafe extern "system" fn ll_mouse_proc(
    code: i32, wparam: windows::Win32::Foundation::WPARAM,
    lparam: windows::Win32::Foundation::LPARAM,
) -> windows::Win32::Foundation::LRESULT {
    if code >= 0 {
        let ms = &*(lparam.0 as *const MSLLHOOKSTRUCT);
        if let Ok(mut g) = HOOK_STATE.lock() {
            if let Some(ref mut state) = *g {
                state.pos = (ms.pt.x, ms.pt.y);
                if wparam.0 as u32 == WM_LBUTTONDOWN { state.clicked = true; }
            }
        }
    }
    CallNextHookEx(HHOOK(std::ptr::null_mut()), code, wparam, lparam)
}
