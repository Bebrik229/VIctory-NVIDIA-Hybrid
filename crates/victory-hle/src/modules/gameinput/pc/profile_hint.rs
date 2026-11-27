//! Shows the selected keyboard profile without activating a second window, including in borderless mode.
use std::sync::OnceLock;
use std::sync::atomic::{AtomicBool, Ordering};
use windows_sys::Win32::Foundation::{HWND, RECT};
use windows_sys::Win32::UI::Input::KeyboardAndMouse::GetAsyncKeyState;
use windows_sys::Win32::UI::WindowsAndMessaging as wm;
use windows_sys::core::w;

static DRIVING: AtomicBool = AtomicBool::new(false);
static SWITCH_DOWN: AtomicBool = AtomicBool::new(false);

fn message() -> u32 {
    static MESSAGE: OnceLock<u32> = OnceLock::new();
    *MESSAGE.get_or_init(|| unsafe { wm::RegisterWindowMessageW(w!("VIctory.KeyboardProfile")) })
}

pub(super) fn attach(hwnd: HWND) {
    // Mark every hooked top-level window. A title can also create an owned dialog.
    unsafe { wm::SetPropW(hwnd, w!("VIctory.KeyboardProfileAttached"), hwnd) };
}

pub(super) fn window_profile(hwnd: HWND) -> Option<bool> {
    if hwnd.is_null() {
        return None;
    }
    // Foreground may be a child or an owned popup. A fallback host window
    // without our window procedure uses the async key snapshot instead.
    let hooked = unsafe {
        !wm::GetPropW(hwnd, w!("VIctory.KeyboardProfileAttached")).is_null()
            || !wm::GetPropW(
                wm::GetAncestor(hwnd, wm::GA_ROOTOWNER),
                w!("VIctory.KeyboardProfileAttached"),
            )
            .is_null()
    };
    hooked.then(|| DRIVING.load(Ordering::Relaxed))
}

pub(super) fn focus_changed(gained: bool) {
    let already_held = gained && unsafe { GetAsyncKeyState(0x75) < 0 };
    SWITCH_DOWN.store(already_held, Ordering::Relaxed);
}

pub(super) fn switch_key(hwnd: HWND, pressed: bool, repeat: bool) {
    if window_profile(hwnd).is_none() {
        return;
    }
    let was_down = SWITCH_DOWN.swap(pressed, Ordering::Relaxed);
    if !pressed || repeat || was_down {
        return;
    }
    let driving = !DRIVING.fetch_xor(true, Ordering::Relaxed);
    tracing::info!(driving, "F6 keyboard profile changed");
    let msg = message();
    if msg != 0 {
        let owner = unsafe { wm::GetAncestor(hwnd, wm::GA_ROOT) };
        unsafe { wm::PostMessageW(owner, msg, usize::from(driving), 0) };
    }
}

pub(super) fn fallback_changed(driving: bool) {
    DRIVING.store(driving, Ordering::Relaxed);
    tracing::info!(
        driving,
        "F6 keyboard profile changed from fallback keyboard input"
    );
}

pub(super) fn is_message(id: u32) -> bool {
    id >= 0xc000 && id == message()
}

pub(super) fn detach(hwnd: HWND) {
    unsafe { wm::RemovePropW(hwnd, w!("VIctory.KeyboardProfileAttached")) };
}

unsafe extern "system" fn expire(hwnd: HWND, _: u32, timer: usize, _: u32) {
    // The timer belongs to this temporary window on the current thread.
    unsafe {
        wm::KillTimer(hwnd, timer);
        let owner = wm::GetWindow(hwnd, wm::GW_OWNER);
        if !owner.is_null() && wm::GetPropW(owner, w!("VIctory.KeyboardHint")) == hwnd {
            wm::RemovePropW(owner, w!("VIctory.KeyboardHint"));
        }
        wm::DestroyWindow(hwnd);
    }
}

pub(super) fn show(owner: HWND, driving: bool) {
    // The notification is an unclickable, owned tool window. A repeated switch
    // replaces the old message, even when the game has no visible title bar.
    unsafe {
        let previous = wm::RemovePropW(owner, w!("VIctory.KeyboardHint"));
        if !previous.is_null() {
            wm::KillTimer(previous, 1);
            wm::DestroyWindow(previous);
        }
        let mut rect = RECT::default();
        if wm::GetForegroundWindow() != owner || wm::GetWindowRect(owner, &mut rect) == 0 {
            return;
        }
        let label = if driving {
            w!("F6  ·  Транспорт")
        } else {
            w!("F6  ·  Пешком")
        };
        let hint = wm::CreateWindowExW(
            wm::WS_EX_TOOLWINDOW | wm::WS_EX_NOACTIVATE | wm::WS_EX_TRANSPARENT,
            w!("STATIC"),
            label,
            wm::WS_POPUP | wm::WS_DISABLED | 0x1 | 0x200, // SS_CENTER | SS_CENTERIMAGE
            rect.left + (rect.right - rect.left - 184) / 2,
            rect.top + 48,
            184,
            36,
            owner,
            std::ptr::null_mut(),
            std::ptr::null_mut(),
            std::ptr::null(),
        );
        if !hint.is_null() {
            if wm::SetTimer(hint, 1, 1500, Some(expire)) == 0 {
                wm::DestroyWindow(hint);
                return;
            }
            wm::SetPropW(owner, w!("VIctory.KeyboardHint"), hint);
            wm::ShowWindow(hint, wm::SW_SHOWNOACTIVATE);
        }
    }
}
