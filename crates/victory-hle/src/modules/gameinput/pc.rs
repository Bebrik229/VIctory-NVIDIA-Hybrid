//! Foreground keyboard and relative mouse input for the console game's pad.
use super::{
    A, B, DOWN, KeyState, LEFT, LEFT_SHOULDER, LEFT_STICK, MENU, PadState, RIGHT, RIGHT_SHOULDER,
    RIGHT_STICK, UP, VIEW, X, Y,
};
use std::collections::VecDeque;
use std::sync::atomic::{AtomicPtr, Ordering};
use std::sync::{Mutex, OnceLock};
use std::time::{Duration, Instant};
use windows_sys::Win32::Foundation::{HWND, LPARAM, LRESULT, POINT, RECT, WPARAM};
use windows_sys::Win32::Graphics::Gdi::ClientToScreen;
use windows_sys::Win32::UI::{Input as raw, WindowsAndMessaging as wm};

mod profile_hint;

static MOTION: std::sync::LazyLock<Mutex<Motion>> = std::sync::LazyLock::new(Mutex::default);
static CAPTURE: AtomicPtr<std::ffi::c_void> = AtomicPtr::new(std::ptr::null_mut());

const MOUSE_WINDOW: Duration = Duration::from_millis(64);

#[derive(Default)]
struct Motion {
    samples: VecDeque<(Instant, [f32; 2])>,
    absolute: Option<(usize, bool, [i32; 2])>,
}

impl Motion {
    fn expire(&mut self, now: Instant) {
        while self
            .samples
            .front()
            .is_some_and(|(at, _)| now.saturating_duration_since(*at) >= MOUSE_WINDOW)
        {
            self.samples.pop_front();
        }
    }

    fn push(&mut self, now: Instant, delta: [f32; 2]) {
        if delta == [0.0; 2] {
            return;
        }
        self.expire(now);
        if let Some((at, accumulated)) = self.samples.back_mut()
            && now.saturating_duration_since(*at) < Duration::from_millis(1)
        {
            accumulated[0] += delta[0];
            accumulated[1] += delta[1];
        } else {
            self.samples.push_back((now, delta));
        }
    }

    fn absolute_position(
        &mut self,
        now: Instant,
        device: usize,
        desktop: bool,
        point: [i32; 2],
        size: [i32; 2],
    ) {
        let previous = self.absolute.replace((device, desktop, point));
        if let Some((old_device, old_desktop, old)) = previous
            && old_device == device
            && old_desktop == desktop
        {
            let delta = std::array::from_fn(|i| {
                (point[i] as f32 - old[i] as f32) * size[i] as f32 / 65535.0
            });
            self.push(now, delta);
        }
    }

    fn stick(&mut self, now: Instant, speed: f32) -> [f32; 2] {
        self.expire(now);
        let mut total = [0.0f32; 2];
        let window = MOUSE_WINDOW.as_secs_f32();
        for &(at, delta) in &self.samples {
            let weight = 1.0 - now.saturating_duration_since(at).as_secs_f32() / window;
            total[0] += delta[0] * weight;
            total[1] -= delta[1] * weight; // Windows Y is down, GameInput Y is up.
        }
        let length = total[0].hypot(total[1]);
        if length < 0.05 {
            return [0.0; 2];
        }
        // A triangular time window has half the area of a rectangular one.
        // Apply the Xbox right-stick dead zone to the radius, preserving the direction.
        let speed_fraction = (length * speed / (1650.0 * window * 0.5)).min(1.0);
        let dead_zone = 8689.0 / 32767.0;
        let radius = dead_zone + (1.0 - dead_zone) * speed_fraction;
        [total[0] * radius / length, total[1] * radius / length]
    }
}

fn motion() -> std::sync::MutexGuard<'static, Motion> {
    MOTION.lock().unwrap_or_else(|e| e.into_inner())
}

fn digital_stick(x: f32, y: f32) -> [f32; 2] {
    let length = x.hypot(y).max(1.0);
    [x / length, y / length]
}

pub(crate) fn enabled() -> bool {
    static ON: OnceLock<bool> = OnceLock::new();
    *ON.get_or_init(|| std::env::var_os("VICTORY_PC_CONTROLS").is_some_and(|v| v == "1"))
}

pub(super) fn bench_script() -> bool {
    static ON: OnceLock<bool> = OnceLock::new();
    *ON.get_or_init(|| std::env::var_os("VICTORY_BENCHMARK_INPUT_ONLY").is_some_and(|v| v == "1"))
}

#[allow(dead_code)]
pub(crate) fn attach(hwnd: HWND) {
    if !enabled() || bench_script() || unsafe { wm::GetAncestor(hwnd, wm::GA_ROOT) } != hwnd {
        return;
    }
    let mouse = raw::RAWINPUTDEVICE {
        usUsagePage: 1,
        usUsage: 2,
        dwFlags: 0,
        hwndTarget: hwnd,
    };
    // SAFETY: one mouse registration for this process and its own client window.
    let ok = unsafe { raw::RegisterRawInputDevices(&mouse, 1, size_of_val(&mouse) as u32) };
    profile_hint::attach(hwnd);
    tracing::info!(
        registered = ok != 0,
        "PC controls: foreground raw mouse and keyboard"
    );
}

pub(super) fn clear_motion() {
    let mut motion = motion();
    motion.samples.clear();
    motion.absolute = None;
}

fn confine(hwnd: HWND) {
    let mut rect = RECT::default();
    // SAFETY: window client rectangle converted to screen coordinates.
    unsafe {
        if wm::GetForegroundWindow() != hwnd || wm::GetClientRect(hwnd, &mut rect) == 0 {
            return;
        }
        let mut origin = POINT::default();
        if rect.right <= 0 || rect.bottom <= 0 || ClientToScreen(hwnd, &mut origin) == 0 {
            return;
        }
        rect.left = origin.x;
        rect.top = origin.y;
        rect.right += origin.x;
        rect.bottom += origin.y;
        if wm::ClipCursor(&rect) != 0 {
            CAPTURE.store(hwnd, Ordering::Relaxed);
        }
    }
}

fn release(hwnd: HWND) {
    if CAPTURE
        .compare_exchange(
            hwnd,
            std::ptr::null_mut(),
            Ordering::Relaxed,
            Ordering::Relaxed,
        )
        .is_ok()
    {
        unsafe {
            wm::ClipCursor(std::ptr::null());
        }
    }
    clear_motion();
}

#[allow(dead_code)]
pub(crate) fn window_event(hwnd: HWND, msg: u32, w: WPARAM, l: LPARAM) -> Option<LRESULT> {
    if !enabled() || bench_script() {
        return None;
    }
    if profile_hint::is_message(msg) {
        profile_hint::show(hwnd, w != 0);
        return Some(0);
    }
    match msg {
        wm::WM_KEYDOWN | wm::WM_KEYUP if w == 0x75 => {
            profile_hint::switch_key(hwnd, msg == wm::WM_KEYDOWN, l & (1 << 30) != 0);
            return Some(0);
        }
        wm::WM_INPUT if w & 0xff == 0 => {
            let mut event = raw::RAWINPUT::default();
            let mut size = size_of_val(&event) as u32;
            // SAFETY: WM_INPUT owns the HRAWINPUT for this call. A fixed RAWINPUT holds a mouse packet.
            let n = unsafe {
                raw::GetRawInputData(
                    l as _,
                    raw::RID_INPUT,
                    (&raw mut event).cast(),
                    &mut size,
                    size_of::<raw::RAWINPUTHEADER>() as u32,
                )
            };
            if n != u32::MAX
                && n >= (size_of::<raw::RAWINPUTHEADER>() + size_of::<raw::RAWMOUSE>()) as u32
                && event.header.dwType == raw::RIM_TYPEMOUSE
            {
                let m = unsafe { event.data.mouse };
                // A foreground owned window (e.g. a dialog) belongs to the same title.
                let foreground = unsafe { wm::GetForegroundWindow() };
                if !foreground.is_null()
                    && (foreground == hwnd
                        || unsafe {
                            wm::GetAncestor(foreground, wm::GA_ROOTOWNER)
                                == wm::GetAncestor(hwnd, wm::GA_ROOTOWNER)
                        })
                {
                    let now = Instant::now();
                    let mut movement = motion();
                    if m.usFlags & raw::MOUSE_MOVE_ABSOLUTE == 0 {
                        movement.push(now, [m.lLastX as f32, m.lLastY as f32]);
                    } else {
                        let desktop = m.usFlags & raw::MOUSE_VIRTUAL_DESKTOP != 0;
                        let metrics = if desktop {
                            [wm::SM_CXVIRTUALSCREEN, wm::SM_CYVIRTUALSCREEN]
                        } else {
                            [wm::SM_CXSCREEN, wm::SM_CYSCREEN]
                        };
                        let size = metrics.map(|metric| unsafe { wm::GetSystemMetrics(metric) });
                        movement.absolute_position(
                            now,
                            event.header.hDevice as usize,
                            desktop,
                            [m.lLastX, m.lLastY],
                            size,
                        );
                    }
                }
                return Some(unsafe { wm::DefWindowProcW(hwnd, msg, w, l) });
            }
        }
        wm::WM_LBUTTONDOWN | wm::WM_RBUTTONDOWN => confine(hwnd),
        wm::WM_SIZE | wm::WM_MOVE if CAPTURE.load(Ordering::Relaxed) == hwnd => confine(hwnd),
        wm::WM_KEYDOWN if w == 0x1b => release(hwnd),
        wm::WM_SETFOCUS => profile_hint::focus_changed(true),
        wm::WM_KILLFOCUS => {
            profile_hint::focus_changed(false);
            release(hwnd);
        }
        wm::WM_NCDESTROY => {
            profile_hint::detach(hwnd);
            release(hwnd);
        }
        wm::WM_SETCURSOR
            if l & 0xffff == wm::HTCLIENT as isize && CAPTURE.load(Ordering::Relaxed) == hwnd =>
        {
            unsafe {
                wm::SetCursor(std::ptr::null_mut());
            }
            return Some(1);
        }
        _ => {}
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;
    fn keys(vks: &[u8]) -> Vec<KeyState> {
        vks.iter()
            .map(|&virtual_key| KeyState {
                virtual_key,
                ..Default::default()
            })
            .collect()
    }
    #[test]
    fn enter_the_house_then_hold_y_to_rest() {
        let mut layout = Layout::default();
        assert_eq!(layout.read(&keys(&[b'F']), false, false).buttons, Y);
        layout.read(&[], false, false);
        let forward = layout.read(&keys(&[b'W']), false, false);
        let back = layout.read(&keys(&[b'S']), false, false);
        assert_eq!(
            (
                forward.left_y,
                forward.right_trigger,
                back.left_y,
                back.left_trigger
            ),
            (1.0, 0.0, -1.0, 0.0)
        );
        for _ in 0..20 {
            assert_eq!(layout.read(&keys(&[b'Y']), false, false).buttons, Y);
        }
        assert_eq!(layout.read(&[], false, false).buttons, 0);
    }

    #[test]
    fn foot_profile_maps_the_requested_buttons_and_held_inputs() {
        let mut layout = Layout::default();
        for (key, button) in [
            (0xa0, LEFT_STICK),
            (0x20, A),
            (0xa2, B),
            (b'Q', RIGHT_SHOULDER),
            (b'F', Y),
            (b'V', X),
            (0x09, LEFT_SHOULDER),
            (b'C', RIGHT_STICK),
            (0x26, UP),
            (0x28, DOWN),
            (0x25, LEFT),
            (0x27, RIGHT),
            (b'M', VIEW),
            (0x1b, MENU),
        ] {
            for _ in 0..3 {
                let s = layout.read(&keys(&[key]), false, false);
                assert_eq!(
                    (s.buttons, s.left_trigger, s.right_trigger),
                    (button, 0.0, 0.0)
                );
            }
            assert_eq!(layout.read(&[], false, false).buttons, 0);
        }
        let s = layout.read(&keys(&[b'W', b'D']), true, true);
        assert_eq!((s.buttons, s.left_trigger, s.right_trigger), (0, 1.0, 1.0));
        assert!(s.left_x > 0.0 && s.left_x == s.left_y);
    }

    #[test]
    fn vehicle_has_independent_pedals_steering_and_face_buttons() {
        let mut layout = Layout::default();
        layout.read(&keys(&[0x75]), false, false);
        let s = layout.read(&keys(&[b'W', b'A', b'H', 0x20, b'F', b'C']), true, false);
        assert_eq!(s.buttons, LEFT_STICK | A | Y | RIGHT_STICK | RIGHT_SHOULDER);
        assert_eq!(
            (s.left_x, s.left_y, s.left_trigger, s.right_trigger),
            (-1.0, 0.0, 0.0, 1.0)
        );
        let s = layout.read(&keys(&[b'D', b'S']), false, true);
        assert_eq!(
            (
                s.left_x,
                s.left_y,
                s.left_trigger,
                s.right_trigger,
                s.buttons
            ),
            (1.0, 0.0, 1.0, 0.0, 0)
        );
        let s = layout.read(&keys(&[b'W', b'S', b'D', b'A']), false, false);
        assert_eq!(
            (s.left_x, s.left_y, s.left_trigger, s.right_trigger),
            (0.0, 0.0, 1.0, 1.0)
        );
    }

    #[test]
    fn switching_does_not_turn_held_inputs_into_new_actions() {
        let mut layout = Layout::default();
        layout.read(&keys(&[b'W', b'F']), true, true);
        for _ in 0..3 {
            let s = layout.read(&keys(&[0x75, b'W', b'F']), true, true);
            assert_eq!(
                (s.buttons, s.left_y, s.left_trigger, s.right_trigger),
                (Y, 0.0, 0.0, 0.0)
            );
        }
        layout.read(&[], false, false);
        let s = layout.read(&keys(&[b'W']), true, false);
        assert_eq!(
            (s.buttons, s.left_y, s.right_trigger),
            (RIGHT_SHOULDER, 0.0, 1.0)
        );
        layout.read(&keys(&[0x75]), false, false);
        let s = layout.read(&keys(&[b'W']), true, false);
        assert_eq!((s.buttons, s.left_y, s.right_trigger), (0, 1.0, 1.0));
    }

    #[test]
    fn regaining_focus_while_f6_is_held_does_not_change_profile() {
        let mut layout = Layout::default();
        layout.suspend();
        let walk = layout.read(&keys(&[0x75, b'W']), false, false);
        assert_eq!((walk.left_y, walk.right_trigger), (1.0, 0.0));
        layout.read(&[], false, false);
        let drive = layout.read(&keys(&[0x75, b'W']), false, false);
        assert_eq!((drive.left_y, drive.right_trigger), (0.0, 1.0));
    }

    #[test]
    fn previously_held_mouse_stays_released_when_focus_returns() {
        let mut layout = Layout::default();
        assert_eq!(layout.read(&[], true, false).right_trigger, 1.0);
        layout.suspend();
        layout.suspend();
        assert_eq!(layout.read(&[], true, false).right_trigger, 0.0);
        layout.read(&[], false, false);
        assert_eq!(layout.read(&[], true, false).right_trigger, 1.0);
    }

    #[test]
    fn quick_f6_between_polls_uses_the_hooked_window() {
        use windows_sys::core::w;
        const CHILD: &str = "VICTORY_TEST_F6_HWND_CHILD";
        if std::env::var_os(CHILD).is_none() {
            // The regular test process can have PC controls disabled. Run the
            // production hook in an isolated process with an invisible real HWND.
            let ok = std::process::Command::new(std::env::current_exe().unwrap())
                .args([
                    "--exact",
                    "modules::gameinput::pc::tests::quick_f6_between_polls_uses_the_hooked_window",
                ])
                .env(CHILD, "1")
                .env("VICTORY_PC_CONTROLS", "1")
                .env("VICTORY_BENCHMARK_INPUT_ONLY", "0")
                .status()
                .unwrap();
            assert!(ok.success());
            return;
        }
        let null = std::ptr::null_mut();
        let hwnd = crate::modules::lifecycle::CreateWindowExW(
            0,
            w!("STATIC"),
            w!("Keyboard smoke"),
            0,
            0,
            0,
            32,
            32,
            null,
            null,
            null,
            std::ptr::null(),
        );
        assert!(!hwnd.is_null());
        let mode = || profile_hint::window_profile(hwnd);
        let send = |msg, lparam| unsafe { wm::SendMessageW(hwnd, msg, 0x75, lparam) };
        let down = || send(wm::WM_KEYDOWN, 1);
        let up = || send(wm::WM_KEYUP, (3isize << 30) | 1);
        assert_eq!(mode(), Some(false));
        let mut layout = Layout::default();
        layout.read_profile(&keys(&[b'W']), true, false, mode());
        down();
        up(); // Entire tap before the next keyboard sample.
        assert_eq!(mode(), Some(true));
        let held = layout.read_profile(&keys(&[b'W']), true, false, mode());
        assert_eq!(
            (held.buttons, held.left_y, held.right_trigger),
            (0, 0.0, 0.0)
        );
        layout.read_profile(&[], false, false, mode());
        assert_eq!(
            layout
                .read_profile(&keys(&[b'W']), false, false, mode())
                .right_trigger,
            1.0
        );
        // A sample can arrive before its window message, or several during a held key.
        layout.read_profile(&keys(&[0x75]), false, false, mode());
        assert!(layout.driving);
        down();
        send(wm::WM_KEYDOWN, (1 << 30) | 1);
        down(); // A duplicate from an input sender with no repeat metadata.
        assert_eq!(mode(), Some(false));
        layout.read_profile(&keys(&[0x75]), false, false, mode());
        assert!(!layout.driving);
        unsafe { wm::SendMessageW(hwnd, wm::WM_KILLFOCUS, 0, 0) };
        send(wm::WM_KEYDOWN, (1 << 30) | 1);
        assert_eq!(mode(), Some(false));
        up();
        for _ in 0..2 {
            down();
            up();
        }
        assert_eq!(mode(), Some(false));
        unsafe { wm::DestroyWindow(hwnd) };
    }

    #[test]
    fn held_sticks_keep_their_direction_and_magnitude() {
        let mut layout = Layout::default();
        for _ in 0..30 {
            let pad = layout.read(&keys(&[b'W', b'D', b'I', b'J']), false, false);
            assert!((pad.left_x.hypot(pad.left_y) - 1.0).abs() < 1e-6);
            assert!((pad.right_x.hypot(pad.right_y) - 1.0).abs() < 1e-6);
            assert!(pad.left_y > 0.0 && pad.left_x == pad.left_y);
            assert!(pad.right_y > 0.0 && -pad.right_x == pad.right_y);
        }
        let pad = layout.read(&keys(&[b'W', b'S', b'J', b'L']), false, false);
        assert_eq!(
            (pad.left_x, pad.left_y, pad.right_x, pad.right_y),
            (0.0, 0.0, 0.0, 0.0)
        );
    }

    #[test]
    fn mouse_is_radial_independent_of_read_frequency_and_returns_to_center() {
        let now = Instant::now();
        let mut m = Motion::default();
        m.push(now, [2.0, -1.0]);
        let t = now + Duration::from_millis(30);
        let first = m.stick(t, 1.0);
        assert_eq!(m.stick(t, 1.0), first);
        assert!(first[0] > 0.0 && first[1] > 0.0 && (first[0] / first[1] - 2.0).abs() < 1e-6);
        assert!(first[0].hypot(first[1]) > 8689.0 / 32767.0);
        assert_eq!(m.stick(now + MOUSE_WINDOW, 1.0), [0.0; 2]);
        m.absolute_position(t, 1, false, [10000, 12000], [1920, 1080]);
        assert_eq!(m.stick(t, 1.0), [0.0; 2]);
        m.absolute_position(t, 1, false, [11000, 10000], [1920, 1080]);
        let stick = m.stick(t, 1.0);
        assert!(stick[0] > 0.0 && stick[1] > 0.0 && stick[0].hypot(stick[1]) <= 1.000001);
    }
}

#[derive(Default)]
pub(super) struct Layout {
    driving: bool,
    switch_down: bool,
    previous_inputs: [bool; 10],
    await_release: [bool; 10],
}

impl Layout {
    pub(super) fn suspend(&mut self) {
        self.switch_down = true;
        for (blocked, held) in self.await_release.iter_mut().zip(self.previous_inputs) {
            *blocked |= held;
        }
        self.previous_inputs.fill(false);
    }

    fn mouse() -> [f32; 2] {
        static SPEED: OnceLock<f32> = OnceLock::new();
        let speed = *SPEED.get_or_init(|| {
            std::env::var("VICTORY_MOUSE_SPEED")
                .ok()
                .and_then(|v| v.parse::<f32>().ok())
                .filter(|v| v.is_finite())
                .unwrap_or(1.0)
                .clamp(0.1, 8.0)
        });
        motion().stick(Instant::now(), speed)
    }

    pub(super) fn read_host(
        &mut self,
        keys: &[KeyState],
        left_mouse: bool,
        right_mouse: bool,
    ) -> PadState {
        let window = profile_hint::window_profile(unsafe { wm::GetForegroundWindow() });
        let previous = self.driving;
        let pad = self.read_profile(keys, left_mouse, right_mouse, window);
        if window.is_none() && previous != self.driving {
            profile_hint::fallback_changed(self.driving);
        }
        pad
    }

    #[cfg(test)]
    pub(super) fn read(
        &mut self,
        keys: &[KeyState],
        left_mouse: bool,
        right_mouse: bool,
    ) -> PadState {
        self.read_profile(keys, left_mouse, right_mouse, None)
    }

    fn read_profile(
        &mut self,
        keys: &[KeyState],
        left_mouse: bool,
        right_mouse: bool,
        window_profile: Option<bool>,
    ) -> PadState {
        let down = |vk: u8| keys.iter().any(|key| key.virtual_key == vk);
        // The title does not expose occupancy through GameInput. F/Y might also
        // open a door or use a bed. Only F6 changes the keyboard profile.
        let held = [
            down(b'W'),
            down(b'S'),
            left_mouse,
            right_mouse,
            down(0xa0),
            down(0xa2),
            down(b'Q'),
            down(b'V'),
            down(0x09),
            down(b'H'),
        ];
        let switch = down(0x75);
        // Exactly one source owns the switch in a given window. For hooked windows
        // all physical down/up edges are handled even between two GameInput polls.
        let driving = window_profile.unwrap_or(self.driving ^ (switch && !self.switch_down));
        let changed = self.driving != driving;
        self.driving = driving;
        let [w, s, fire, aim, shift, ctrl, q, v, tab, horn] = std::array::from_fn(|i| {
            self.await_release[i] =
                held[i] && (self.await_release[i] || changed && self.previous_inputs[i]);
            held[i] && !self.await_release[i]
        });
        self.switch_down = switch;
        self.previous_inputs = held;
        let buttons = [
            (down(0x1b), MENU),
            (down(b'M'), VIEW),
            (down(0x20) || down(0x0d), A),
            (down(0x08) || down(b'B'), B),
            (down(b'F') || down(b'Y'), Y),
            (down(b'X') || (!self.driving && v), X),
            (down(0x27), RIGHT),
            (down(0x25), LEFT),
            (down(0x26), UP),
            (down(0x28), DOWN),
            (down(b'C'), RIGHT_STICK),
            (!self.driving && ctrl, B),
            (!self.driving && tab, LEFT_SHOULDER),
            (if self.driving { fire } else { q }, RIGHT_SHOULDER),
            (if self.driving { horn } else { shift }, LEFT_STICK),
        ]
        .into_iter()
        .filter(|&(on, _)| on)
        .fold(0, |bits, (_, button)| bits | button);
        let axis = |pos, neg| u8::from(down(pos)) as f32 - u8::from(down(neg)) as f32;
        let forward = if self.driving {
            0.0
        } else {
            u8::from(w) as f32 - u8::from(s) as f32
        };
        let [left_x, left_y] = digital_stick(axis(b'D', b'A'), forward);
        let [right_x, right_y] = if [b'I', b'J', b'K', b'L'].into_iter().any(down) {
            digital_stick(axis(b'L', b'J'), axis(b'I', b'K'))
        } else {
            Self::mouse()
        };
        PadState {
            buttons,
            right_x,
            right_y,
            left_x,
            left_y,
            left_trigger: u8::from(if self.driving { s } else { aim }) as f32,
            right_trigger: u8::from(if self.driving { w } else { fire }) as f32,
        }
    }
}
