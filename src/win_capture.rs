//! Windows-specific capture helpers.
#![cfg(target_os = "windows")]

use std::sync::Mutex;

use windows::Win32::Foundation::{HWND, POINT, RECT};
use windows::Win32::Graphics::Gdi::{
    BitBlt, CreateCompatibleBitmap, CreateCompatibleDC, DeleteDC,
    GetDC, ReleaseDC, SelectObject, ROP_CODE, HBITMAP, HDC,
};
use windows::Win32::UI::WindowsAndMessaging::{
    CallNextHookEx, GetWindowRect,
    IsIconic, IsWindowVisible, SetWindowsHookExW,
    UnhookWindowsHookEx, WindowFromPoint, HHOOK, MSLLHOOKSTRUCT,
    WH_MOUSE_LL, WM_LBUTTONDOWN,
    GWL_EXSTYLE, WS_EX_TOOLWINDOW, WS_EX_TRANSPARENT,
    GetWindowLongPtrW,
};

// ── Smart window detection ────────────────────────────────────────────────────

/// Find the best window to highlight under the given screen point.
///
/// Strategy (matches Snipaste / ShareX behaviour):
/// 1. Get the top-level window under the cursor via `WindowFromPoint`.
/// 2. Walk up to the root owner to avoid highlighting child controls by default.
/// 3. Filter out: invisible, minimised, tool windows, transparent overlays,
///    zero-size windows, and the caller's own HWND.
/// 4. Return the window's screen rect in physical pixels.
pub fn hovered_window_rect(screen_x: i32, screen_y: i32, own_hwnd: isize) -> Option<[i32; 4]> {
    unsafe {
        let pt = POINT { x: screen_x, y: screen_y };
        let hwnd = WindowFromPoint(pt);
        if hwnd.0.is_null() { return None; }

        // Skip our own overlay window
        if hwnd.0 as isize == own_hwnd { return None; }

        // Walk up to the root (non-child) window — avoids tiny child controls
        let root = get_ancestor_root(hwnd);

        // Filter: must be visible, not minimised
        if !IsWindowVisible(root).as_bool() { return None; }
        if IsIconic(root).as_bool()         { return None; }

        // Filter extended styles
        let ex = GetWindowLongPtrW(root, GWL_EXSTYLE) as u32;
        if ex & WS_EX_TOOLWINDOW.0  != 0 { return None; }
        if ex & WS_EX_TRANSPARENT.0 != 0 { return None; }

        let mut rect = RECT::default();
        GetWindowRect(root, &mut rect).ok()?;
        let w = rect.right  - rect.left;
        let h = rect.bottom - rect.top;
        if w <= 4 || h <= 4 { return None; }

        Some([rect.left, rect.top, rect.right, rect.bottom])
    }
}

/// Walk up the window hierarchy to find the root ancestor (non-child window).
unsafe fn get_ancestor_root(hwnd: HWND) -> HWND {
    use windows::Win32::UI::WindowsAndMessaging::{GetAncestor, GA_ROOT};
    let root = GetAncestor(hwnd, GA_ROOT);
    if root.0.is_null() { hwnd } else { root }
}

/// Get the HWND of the foreground window (used to identify our own window).
pub fn get_own_hwnd() -> isize {
    unsafe {
        use windows::Win32::UI::WindowsAndMessaging::GetForegroundWindow;
        GetForegroundWindow().0 as isize
    }
}

// ── GDI capture ───────────────────────────────────────────────────────────────

/// Capture a screen rectangle using GDI BitBlt + CAPTUREBLT.
/// Handles layered windows, full-screen apps, and DPI scaling correctly.
/// Returns RGBA bytes (width × height × 4).
pub fn capture_rect_gdi(left: i32, top: i32, width: i32, height: i32) -> Option<Vec<u8>> {
    if width <= 0 || height <= 0 { return None; }
    unsafe {
        let screen_dc: HDC = GetDC(HWND(std::ptr::null_mut()));
        if screen_dc.is_invalid() { return None; }

        let mem_dc: HDC = CreateCompatibleDC(screen_dc);
        let bmp: HBITMAP = CreateCompatibleBitmap(screen_dc, width, height);
        let old = SelectObject(mem_dc, bmp);

        // SRCCOPY | CAPTUREBLT — includes layered (WS_EX_LAYERED) windows
        const CAPTUREBLT: u32 = 0x4000_0000;
        const SRCCOPY_VAL: u32 = 0x00CC_0020;
        let _ = BitBlt(mem_dc, 0, 0, width, height,
                       screen_dc, left, top, ROP_CODE(SRCCOPY_VAL | CAPTUREBLT));

        // Read pixels via GetDIBits
        use windows::Win32::Graphics::Gdi::{
            GetDIBits, BITMAPINFO, BITMAPINFOHEADER, DIB_RGB_COLORS, BI_RGB,
        };
        let mut bmi = BITMAPINFO {
            bmiHeader: BITMAPINFOHEADER {
                biSize:        std::mem::size_of::<BITMAPINFOHEADER>() as u32,
                biWidth:       width,
                biHeight:      -height, // top-down DIB
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

        // BGRA → RGBA in-place
        for chunk in bgra.chunks_exact_mut(4) {
            chunk.swap(0, 2);
            chunk[3] = 255;
        }

        SelectObject(mem_dc, old);
        let _ = windows::Win32::Graphics::Gdi::DeleteObject(bmp);
        let _ = DeleteDC(mem_dc);
        ReleaseDC(HWND(std::ptr::null_mut()), screen_dc);

        Some(bgra)
    }
}

// ── Low-level mouse hook ──────────────────────────────────────────────────────

#[derive(Default, Clone)]
pub struct MouseState {
    pub pos:     (i32, i32),
    pub clicked: bool,
}

static HOOK_STATE: Mutex<Option<MouseState>> = Mutex::new(None);

pub struct LowLevelMouseHook {
    hhook: HHOOK,
}

impl LowLevelMouseHook {
    pub fn install() -> Option<Self> {
        unsafe {
            *HOOK_STATE.lock().unwrap() = Some(MouseState::default());
            let h = SetWindowsHookExW(WH_MOUSE_LL, Some(ll_mouse_proc), None, 0).ok()?;
            Some(Self { hhook: h })
        }
    }

    pub fn poll() -> MouseState {
        HOOK_STATE.lock().unwrap().clone().unwrap_or_default()
    }

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
    code: i32,
    wparam: windows::Win32::Foundation::WPARAM,
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
