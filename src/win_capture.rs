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

// ── Own-window detection ──────────────────────────────────────────────────────

/// Enumerate all HWNDs belonging to the current process — single pass.
pub fn get_process_hwnds() -> Vec<isize> {
    use windows::Win32::System::Threading::GetCurrentProcessId;
    use windows::Win32::UI::WindowsAndMessaging::{EnumWindows, GetWindowThreadProcessId};
    use windows::Win32::Foundation::{BOOL, LPARAM, TRUE};

    struct Ctx { pid: u32, hwnds: Vec<isize> }

    unsafe {
        let pid = GetCurrentProcessId();
        let mut ctx = Ctx { pid, hwnds: Vec::new() };
        let ptr = &mut ctx as *mut Ctx as isize;

        unsafe extern "system" fn cb(hwnd: HWND, lparam: LPARAM) -> BOOL {
            let ctx = &mut *(lparam.0 as *mut Ctx);
            let mut win_pid = 0u32;
            GetWindowThreadProcessId(hwnd, Some(&mut win_pid));
            if win_pid == ctx.pid { ctx.hwnds.push(hwnd.0 as isize); }
            TRUE
        }

        let _ = EnumWindows(Some(cb), LPARAM(ptr));
        ctx.hwnds
    }
}

// ── Smart window detection ────────────────────────────────────────────────────

/// Find the best window to highlight under the given screen point.
/// Throttled: only re-queries Win32 when cursor moves > 2 physical pixels,
/// avoiding 4 system calls per frame during mouse-move.
pub fn hovered_window_rect(
    screen_x: i32, screen_y: i32,
    own_hwnds: &[isize],
    last_pos:  &mut (i32, i32),
    last_rect: &mut Option<[i32; 4]>,
) -> Option<[i32; 4]> {
    const THRESHOLD: i32 = 2;
    if (screen_x - last_pos.0).abs() <= THRESHOLD
    && (screen_y - last_pos.1).abs() <= THRESHOLD {
        return *last_rect;
    }
    *last_pos = (screen_x, screen_y);
    let result = query_window_rect(screen_x, screen_y, own_hwnds);
    *last_rect = result;
    result
}

fn query_window_rect(screen_x: i32, screen_y: i32, own_hwnds: &[isize]) -> Option<[i32; 4]> {
    unsafe {
        let pt = POINT { x: screen_x, y: screen_y };
        let hwnd = WindowFromPoint(pt);
        if hwnd.0.is_null() { return None; }

        let root = get_ancestor_root(hwnd);

        if own_hwnds.contains(&(root.0 as isize)) { return None; }
        if own_hwnds.contains(&(hwnd.0 as isize))  { return None; }

        if !IsWindowVisible(root).as_bool() { return None; }
        if IsIconic(root).as_bool()         { return None; }

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

unsafe fn get_ancestor_root(hwnd: HWND) -> HWND {
    use windows::Win32::UI::WindowsAndMessaging::{GetAncestor, GA_ROOT};
    let root = GetAncestor(hwnd, GA_ROOT);
    if root.0.is_null() { hwnd } else { root }
}

// ── GDI capture ───────────────────────────────────────────────────────────────

pub fn capture_rect_gdi(left: i32, top: i32, width: i32, height: i32) -> Option<Vec<u8>> {
    if width <= 0 || height <= 0 { return None; }
    unsafe {
        let screen_dc: HDC = GetDC(HWND(std::ptr::null_mut()));
        if screen_dc.is_invalid() { return None; }

        let mem_dc: HDC = CreateCompatibleDC(screen_dc);
        let bmp: HBITMAP = CreateCompatibleBitmap(screen_dc, width, height);
        let old = SelectObject(mem_dc, bmp);

        const CAPTUREBLT: u32 = 0x4000_0000;
        const SRCCOPY_VAL: u32 = 0x00CC_0020;
        let _ = BitBlt(mem_dc, 0, 0, width, height,
                       screen_dc, left, top, ROP_CODE(SRCCOPY_VAL | CAPTUREBLT));

        use windows::Win32::Graphics::Gdi::{
            GetDIBits, BITMAPINFO, BITMAPINFOHEADER, DIB_RGB_COLORS, BI_RGB,
        };
        let mut bmi = BITMAPINFO {
            bmiHeader: BITMAPINFOHEADER {
                biSize:        std::mem::size_of::<BITMAPINFOHEADER>() as u32,
                biWidth:       width,
                biHeight:      -height,
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

pub fn capture_fullscreen() -> Option<(Vec<u8>, usize, usize)> {
    use windows::Win32::UI::WindowsAndMessaging::{GetSystemMetrics, SM_CXSCREEN, SM_CYSCREEN};
    let (w, h) = unsafe { (GetSystemMetrics(SM_CXSCREEN), GetSystemMetrics(SM_CYSCREEN)) };
    let pixels = capture_rect_gdi(0, 0, w, h)?;
    Some((pixels, w as usize, h as usize))
}

// ── Low-level keyboard hook ───────────────────────────────────────────────────

#[derive(Default, Clone)]
pub struct KeyboardState {
    pub win_held: bool,
}

static KB_STATE: Mutex<Option<KeyboardState>> = Mutex::new(None);

pub struct LowLevelKeyboardHook { hhook: HHOOK }

impl LowLevelKeyboardHook {
    pub fn install() -> Option<Self> {
        use windows::Win32::UI::WindowsAndMessaging::WH_KEYBOARD_LL;
        unsafe {
            *KB_STATE.lock().unwrap() = Some(KeyboardState::default());
            let h = SetWindowsHookExW(WH_KEYBOARD_LL, Some(ll_kb_proc), None, 0).ok()?;
            Some(Self { hhook: h })
        }
    }
    pub fn poll() -> KeyboardState {
        KB_STATE.lock().unwrap().clone().unwrap_or_default()
    }
}

impl Drop for LowLevelKeyboardHook {
    fn drop(&mut self) {
        unsafe { let _ = UnhookWindowsHookEx(self.hhook); }
        *KB_STATE.lock().unwrap() = None;
    }
}

unsafe extern "system" fn ll_kb_proc(
    code: i32,
    wparam: windows::Win32::Foundation::WPARAM,
    lparam: windows::Win32::Foundation::LPARAM,
) -> windows::Win32::Foundation::LRESULT {
    use windows::Win32::UI::WindowsAndMessaging::{KBDLLHOOKSTRUCT, WM_KEYDOWN, WM_SYSKEYDOWN};
    const VK_LWIN: u32 = 0x5B;
    const VK_RWIN: u32 = 0x5C;
    if code >= 0 {
        let ks = &*(lparam.0 as *const KBDLLHOOKSTRUCT);
        let vk = ks.vkCode;
        if vk == VK_LWIN || vk == VK_RWIN {
            let down = wparam.0 as u32 == WM_KEYDOWN || wparam.0 as u32 == WM_SYSKEYDOWN;
            if let Ok(mut g) = KB_STATE.lock() {
                if let Some(ref mut state) = *g { state.win_held = down; }
            }
        }
    }
    CallNextHookEx(HHOOK(std::ptr::null_mut()), code, wparam, lparam)
}

// ── Low-level mouse hook ──────────────────────────────────────────────────────

#[derive(Default, Clone)]
pub struct MouseState {
    pub pos:      (i32, i32),
    pub clicked:  bool,
    pub released: bool,
    pub btn_down: bool,
}

static HOOK_STATE: Mutex<Option<MouseState>> = Mutex::new(None);

pub struct LowLevelMouseHook { hhook: HHOOK }

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
    pub fn consume_click(&self) {
        if let Ok(mut g) = HOOK_STATE.lock() {
            if let Some(ref mut s) = *g { s.clicked = false; }
        }
    }
    pub fn consume_release(&self) {
        if let Ok(mut g) = HOOK_STATE.lock() {
            if let Some(ref mut s) = *g { s.released = false; }
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
    use windows::Win32::UI::WindowsAndMessaging::{WM_LBUTTONUP, WM_MOUSEMOVE};
    if code >= 0 {
        let ms = &*(lparam.0 as *const MSLLHOOKSTRUCT);
        if let Ok(mut g) = HOOK_STATE.lock() {
            if let Some(ref mut state) = *g {
                state.pos = (ms.pt.x, ms.pt.y);
                let msg = wparam.0 as u32;
                if msg == WM_LBUTTONDOWN { state.clicked = true;  state.btn_down = true; }
                if msg == WM_LBUTTONUP   { state.released = true; state.btn_down = false; }
                let _ = WM_MOUSEMOVE; // pos already updated above
            }
        }
    }
    CallNextHookEx(HHOOK(std::ptr::null_mut()), code, wparam, lparam)
}
