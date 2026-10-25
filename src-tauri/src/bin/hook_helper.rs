#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use std::io::{self, BufRead};
use std::sync::atomic::{AtomicBool, AtomicU8, AtomicU64, Ordering};
use std::sync::{mpsc, Arc, Condvar, Mutex, RwLock};
use std::thread;
use std::time::{Duration, Instant};
use rdev::{grab, simulate, Event, EventType, Key};
use winvd::{get_current_desktop, get_desktops, pin_window, switch_desktop, unpin_window};
use std::cell::RefCell;
use windows::core::{w, Interface, PCWSTR};
use windows::Foundation::Numerics::Matrix3x2;
use windows::Win32::Foundation::{BOOL, HINSTANCE, HMODULE, HWND, LPARAM, LRESULT, POINT, RECT, WPARAM};
use windows::Win32::Graphics::Direct3D::{D3D_DRIVER_TYPE_HARDWARE, D3D_DRIVER_TYPE_WARP};
use windows::Win32::Graphics::Direct3D11::{
    D3D11CreateDevice, ID3D11Device, D3D11_CREATE_DEVICE_BGRA_SUPPORT, D3D11_SDK_VERSION,
};
use windows::Win32::Graphics::DirectComposition::{
    DCompositionCreateDevice, IDCompositionDevice, IDCompositionEffectGroup, IDCompositionSurface,
    IDCompositionTarget, IDCompositionVisual, IDCompositionVisual3,
    DCOMPOSITION_BITMAP_INTERPOLATION_MODE_LINEAR,
};
use windows::Win32::Graphics::Dwm::DwmFlush;
use windows::Win32::Graphics::Dxgi::Common::{
    DXGI_ALPHA_MODE_IGNORE, DXGI_FORMAT_B8G8R8A8_UNORM,
};
use windows::Win32::Graphics::Dxgi::{IDXGIAdapter, IDXGIDevice, IDXGISurface1};
use windows::Win32::Graphics::Gdi::{
    BitBlt, CreateCompatibleBitmap, CreateCompatibleDC, DeleteDC, DeleteObject, GetDC,
    ReleaseDC, SelectObject, HBITMAP, HBRUSH, SRCCOPY,
};
use windows::Win32::System::Com::{CoInitializeEx, COINIT_MULTITHREADED};
use windows::Win32::UI::HiDpi::{
    SetProcessDpiAwarenessContext, DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2,
};
use windows::Win32::UI::WindowsAndMessaging::{
    CreateWindowExW, DefWindowProcW, DestroyWindow, DispatchMessageW,
    LoadCursorW, MSG, PM_REMOVE, PeekMessageW, RegisterClassExW, SetProcessDPIAware, SetWindowDisplayAffinity,
    SetWindowPos, ShowWindow, SystemParametersInfoW, TranslateMessage, CS_HREDRAW, CS_VREDRAW,
    HCURSOR, HICON, HMENU, IDC_ARROW, SPI_GETWORKAREA, SWP_NOACTIVATE, SWP_NOMOVE, SWP_NOSIZE,
    SW_SHOWNOACTIVATE, HWND_TOP, WDA_EXCLUDEFROMCAPTURE, WDA_NONE,
    SYSTEM_PARAMETERS_INFO_UPDATE_FLAGS,
    WS_EX_NOREDIRECTIONBITMAP, WS_EX_NOACTIVATE, WS_EX_TOOLWINDOW, WS_EX_TOPMOST, WS_EX_TRANSPARENT,
    WS_POPUP, WNDCLASSEXW,
};

const DOUBLE_TAP_MS: Duration = Duration::from_millis(350);
const TAP_UP_DELAY_MS: Duration = Duration::from_millis(180);
const COALESCE_QUIET: Duration = Duration::from_millis(18);
const COALESCE_MAX: Duration = Duration::from_millis(220);
const GRACE_MS: Duration = Duration::from_millis(12);
const SLIDE_MS: Duration = Duration::from_millis(180);
const SCALE_MS: Duration = Duration::from_millis(150);
const CLASS_NAME: PCWSTR = w!("SpacesCurtainClass");

type SharedKey = Arc<RwLock<Key>>;

static TRIGGER_DOWN: AtomicBool = AtomicBool::new(false);
static TRIGGER_KEY: Mutex<Key> = Mutex::new(Key::MetaLeft);
static WHEEL_USED: AtomicBool = AtomicBool::new(false);
static OTHER_KEY_USED: AtomicBool = AtomicBool::new(false);
static PENDING: AtomicU8 = AtomicU8::new(0);
static WIN_UP_GEN: AtomicU64 = AtomicU64::new(0);
static DOUBLE_ARMED: AtomicBool = AtomicBool::new(false);
static LAST_PRESS: Mutex<Option<Instant>> = Mutex::new(None);
static ANIMATIONS: AtomicBool = AtomicBool::new(true);
static TASKVIEW: AtomicBool = AtomicBool::new(true);
static ANIM_KIND: AtomicU8 = AtomicU8::new(0);

const ANIM_MACOS: u8 = 0;
const ANIM_WINDOWS: u8 = 1;
const ANIM_SCALE: u8 = 2;

static CLASS_OK: AtomicBool = AtomicBool::new(false);

struct Capture {
    bmp: isize,
    x: i32,
    y: i32,
    w: i32,
    h: i32,
}

struct AnimState {
    slot: Option<(Option<Capture>, i32)>,
    running: bool,
    interrupt: bool,
}

static ANIM_M: Mutex<AnimState> = Mutex::new(AnimState {
    slot: None,
    running: false,
    interrupt: false,
});
static ANIM_CV: Condvar = Condvar::new();

enum Msg {
    Switch { up: bool },
    ReleaseTrigger(Key),
    TaskView,
}

fn same_key(a: Key, b: Key) -> bool {
    fn norm(k: Key) -> Key {
        match k {
            Key::MetaRight => Key::MetaLeft,
            Key::ShiftRight => Key::ShiftLeft,
            Key::AltGr => Key::Alt,
            other => other,
        }
    }
    norm(a) == norm(b)
}

fn is_win_key(k: Key) -> bool {
    matches!(k, Key::MetaLeft | Key::MetaRight)
}

fn key_from_str(s: &str) -> Option<Key> {
    Some(match s {
        "MetaLeft" => Key::MetaLeft,
        "MetaRight" => Key::MetaRight,
        "Alt" => Key::Alt,
        "AltGr" => Key::AltGr,
        "ShiftLeft" => Key::ShiftLeft,
        "ShiftRight" => Key::ShiftRight,
        "CapsLock" => Key::CapsLock,
        "F1" => Key::F1,
        "F2" => Key::F2,
        "F3" => Key::F3,
        "F4" => Key::F4,
        "F5" => Key::F5,
        "F6" => Key::F6,
        "F7" => Key::F7,
        "F8" => Key::F8,
        "F9" => Key::F9,
        "F10" => Key::F10,
        "F11" => Key::F11,
        "F12" => Key::F12,
        _ => return None,
    })
}

fn switch_offset(offset: i32) -> Result<(), String> {
    let desktops = get_desktops().map_err(|e| format!("{:?}", e))?;
    let count = desktops.len() as i64;
    if count <= 1 {
        return Ok(());
    }
    let current = get_current_desktop()
        .and_then(|d| d.get_index())
        .map_err(|e| format!("{:?}", e))? as i64;
    let target = (current - offset as i64).rem_euclid(count) as usize;
    switch_desktop(desktops[target].clone()).map_err(|e| format!("{:?}", e))
}

unsafe fn pump() {
    unsafe {
        let mut msg = MSG::default();
        while PeekMessageW(&mut msg, None::<&HWND>, 0, 0, PM_REMOVE).as_bool() {
            let _ = TranslateMessage(&msg);
            let _ = DispatchMessageW(&msg);
        }
    }
}

thread_local! {
    static DC: RefCell<Option<IDCompositionDevice>> = const { RefCell::new(None) };
}

unsafe fn create_dcomp_device() -> Option<IDCompositionDevice> {
    unsafe {
        let _ = CoInitializeEx(None, COINIT_MULTITHREADED);
        for kind in [D3D_DRIVER_TYPE_HARDWARE, D3D_DRIVER_TYPE_WARP] {
            let mut d3d: Option<ID3D11Device> = None;
            if let Err(_) = D3D11CreateDevice(
                None::<&IDXGIAdapter>,
                kind,
                HMODULE(std::ptr::null_mut()),
                D3D11_CREATE_DEVICE_BGRA_SUPPORT,
                None,
                D3D11_SDK_VERSION,
                Some(&mut d3d),
                None,
                None,
            ) {
                continue;
            }
            if let Some(d3d) = d3d {
                if let Ok(dxgi) = d3d.cast::<IDXGIDevice>() {
                    return DCompositionCreateDevice::<_, IDCompositionDevice>(&dxgi).ok();
                }
            }
        }
        None
    }
}

fn with_dcomp<T>(f: impl FnOnce(&IDCompositionDevice) -> Option<T>) -> Option<T> {
    DC.with(|dc| {
        let mut g = dc.borrow_mut();
        if g.is_none() {
            *g = unsafe { create_dcomp_device() };
        }
        let dev = g.as_ref()?;
        f(dev)
    })
}

unsafe fn capture_screen() -> Option<Capture> {
    unsafe {
        let mut wa = RECT::default();
        if SystemParametersInfoW(
            SPI_GETWORKAREA,
            0,
            Some(&mut wa as *mut RECT as *mut core::ffi::c_void),
            SYSTEM_PARAMETERS_INFO_UPDATE_FLAGS(0),
        )
        .is_err()
        {
            return None;
        }
        let x = wa.left;
        let y = wa.top;
        let w = wa.right - wa.left;
        let h = wa.bottom - wa.top;
        if w <= 0 || h <= 0 {
            return None;
        }
        let screen = GetDC(None::<&HWND>);
        if screen.0.is_null() {
            return None;
        }
        let mem = CreateCompatibleDC(screen);
        let bmp = CreateCompatibleBitmap(screen, w, h);
        let mut ok = !bmp.0.is_null();
        if ok {
            let old = SelectObject(mem, bmp);
            ok = BitBlt(mem, 0, 0, w, h, screen, x, y, SRCCOPY).is_ok();
            SelectObject(mem, old);
        }
        let _ = DeleteDC(mem);
        ReleaseDC(None::<&HWND>, screen);
        if !ok {
            if !bmp.0.is_null() {
                let _ = DeleteObject(bmp);
            }
            return None;
        }
        Some(Capture {
            bmp: bmp.0 as isize,
            x,
            y,
            w,
            h,
        })
    }
}

unsafe fn free_capture(c: Capture) {
    unsafe {
        let _ = DeleteObject(HBITMAP(c.bmp as *mut core::ffi::c_void));
    }
}

unsafe extern "system" fn wnd_proc(
    hwnd: HWND,
    msg: u32,
    wparam: WPARAM,
    lparam: LPARAM,
) -> LRESULT {
    unsafe { DefWindowProcW(hwnd, msg, wparam, lparam) }
}

unsafe fn ensure_class() -> bool {
    unsafe {
        if CLASS_OK.load(Ordering::SeqCst) {
            return true;
        }
        let wc = WNDCLASSEXW {
            cbSize: std::mem::size_of::<WNDCLASSEXW>() as u32,
            style: CS_HREDRAW | CS_VREDRAW,
            lpfnWndProc: Some(wnd_proc),
            cbClsExtra: 0,
            cbWndExtra: 0,
            hInstance: HINSTANCE(std::ptr::null_mut()),
            hIcon: HICON(std::ptr::null_mut()),
            hCursor: LoadCursorW(None::<&HINSTANCE>, IDC_ARROW)
                .unwrap_or(HCURSOR(std::ptr::null_mut())),
            hbrBackground: HBRUSH(std::ptr::null_mut()),
            lpszMenuName: PCWSTR(std::ptr::null()),
            lpszClassName: CLASS_NAME,
            hIconSm: HICON(std::ptr::null_mut()),
        };
        if RegisterClassExW(&wc) == 0 {
            return false;
        }
        CLASS_OK.store(true, Ordering::SeqCst);
        true
    }
}

enum OpacityKind {
    V3(IDCompositionVisual3),
    Group(IDCompositionEffectGroup),
    None,
}

struct Overlay {
    hwnd: HWND,
    target: IDCompositionTarget,
    visual: IDCompositionVisual,
    opacity: OpacityKind,
    surface: IDCompositionSurface,
    device: IDCompositionDevice,
}

unsafe fn blit_into(surface: &IDCompositionSurface, bmp: HBITMAP, w: i32, h: i32) -> Option<()> {
    unsafe {
        let mut off = POINT::default();
        let surf1: IDXGISurface1 = surface.BeginDraw(None, &mut off).ok()?;
        let hdc = surf1.GetDC(BOOL(1)).ok()?;
        let mut ok = false;
        let mem = CreateCompatibleDC(hdc);
        if !mem.0.is_null() {
            let old = SelectObject(mem, bmp);
            ok = BitBlt(hdc, off.x, off.y, w, h, mem, 0, 0, SRCCOPY).is_ok();
            SelectObject(mem, old);
            let _ = DeleteDC(mem);
        }
        let dirty = RECT {
            left: 0,
            top: 0,
            right: w,
            bottom: h,
        };
        let _ = surf1.ReleaseDC(Some(&dirty));
        surface.EndDraw().ok()?;
        if ok {
            Some(())
        } else {
            None
        }
    }
}

unsafe fn create_overlay(c: &Capture, show: bool) -> Option<Overlay> {
    unsafe {
        if !ensure_class() {
            return None;
        }
        let hwnd = CreateWindowExW(
            WS_EX_TOPMOST
                | WS_EX_TOOLWINDOW
                | WS_EX_NOACTIVATE
                | WS_EX_TRANSPARENT
                | WS_EX_NOREDIRECTIONBITMAP,
            CLASS_NAME,
            w!(""),
            WS_POPUP,
            c.x,
            c.y,
            c.w,
            c.h,
            None::<&HWND>,
            None::<&HMENU>,
            HINSTANCE(std::ptr::null_mut()),
            None,
        )
        .ok()?;
        let bmp = HBITMAP(c.bmp as *mut core::ffi::c_void);
        let ov = with_dcomp(|device| {
            let target = match device.CreateTargetForHwnd(hwnd, BOOL(1)) {
                Ok(t) => t,
                Err(_) => return None,
            };
            let visual = match device.CreateVisual() {
                Ok(v) => v,
                Err(_) => return None,
            };
            let opacity = match visual.cast::<IDCompositionVisual3>() {
                Ok(v3) => OpacityKind::V3(v3),
                Err(_) => match device.CreateEffectGroup() {
                    Ok(g) => match visual.SetEffect(&g) {
                        Ok(()) => OpacityKind::Group(g),
                        Err(_) => OpacityKind::None,
                    },
                    Err(_) => OpacityKind::None,
                },
            };
            let surface = match device.CreateSurface(
                c.w as u32,
                c.h as u32,
                DXGI_FORMAT_B8G8R8A8_UNORM,
                DXGI_ALPHA_MODE_IGNORE,
            ) {
                Ok(s) => s,
                Err(_) => return None,
            };
            if blit_into(&surface, bmp, c.w, c.h).is_none() {
                return None;
            }
            if let Err(_) = visual.SetContent(&surface) {
                return None;
            }
            let _ = visual.SetBitmapInterpolationMode(DCOMPOSITION_BITMAP_INTERPOLATION_MODE_LINEAR);
            if let Err(_) = target.SetRoot(&visual) {
                return None;
            }
            if let Err(_) = device.Commit() {
                return None;
            }
            Some(Overlay {
                hwnd,
                target,
                visual,
                opacity,
                surface,
                device: device.clone(),
            })
        });
        if ov.is_none() {
            let _ = DestroyWindow(hwnd);
            return None;
        }
        if show {
            let _ = ShowWindow(hwnd, SW_SHOWNOACTIVATE);
            pump();
        }
        ov
    }
}

fn animate_macos(old: &Overlay, new: Option<&Overlay>, c: &Capture, net: i32) {
    let w = c.w as f32;
    let dir: f32 = if net > 0 { 1.0 } else { -1.0 };
    let _ = unsafe {
        match &old.opacity {
            OpacityKind::V3(v) => v.SetOpacity2(1.0),
            OpacityKind::Group(g) => g.SetOpacity2(1.0),
            OpacityKind::None => Ok(()),
        }
    };
    let start = Instant::now();
    let total = SLIDE_MS.as_secs_f64();
    loop {
        if ANIM_M.lock().unwrap().interrupt {
            break;
        }
        let t = (start.elapsed().as_secs_f64() / total).min(1.0);
        let e = if t < 0.5 {
            4.0 * t * t * t
        } else {
            1.0 - (-2.0 * t + 2.0).powi(3) / 2.0
        };
        let off = dir * w * e as f32;
        let m_old = Matrix3x2 {
            M11: 1.0,
            M12: 0.0,
            M21: 0.0,
            M22: 1.0,
            M31: off,
            M32: 0.0,
        };
        let m_new = Matrix3x2 {
            M11: 1.0,
            M12: 0.0,
            M21: 0.0,
            M22: 1.0,
            M31: off - dir * w,
            M32: 0.0,
        };
        let ok = unsafe {
            let a = old
                .visual
                .SetTransform2(&m_old)
                .and_then(|_| old.device.Commit());
            let b = match new {
                Some(n) => n
                    .visual
                    .SetTransform2(&m_new)
                    .and_then(|_| n.device.Commit()),
                None => Ok(()),
            };
            a.and(b).is_ok()
        };
        if !ok {
            break;
        }
        unsafe {
            if DwmFlush().is_err() {
                thread::sleep(Duration::from_millis(6));
            }
        }
        if t >= 1.0 {
            break;
        }
    }
}

fn set_overlay_opacity(o: &Overlay, alpha: f32) {
    let _ = unsafe {
        match &o.opacity {
            OpacityKind::V3(v) => v.SetOpacity2(alpha),
            OpacityKind::Group(g) => g.SetOpacity2(alpha),
            OpacityKind::None => Ok(()),
        }
    };
}

fn center_scale_matrix(s: f32, w: f32, h: f32) -> Matrix3x2 {
    Matrix3x2 {
        M11: s,
        M12: 0.0,
        M21: 0.0,
        M22: s,
        M31: (1.0 - s) * w * 0.5,
        M32: (1.0 - s) * h * 0.5,
    }
}

fn animate_scale(old: &Overlay, new: Option<&Overlay>, c: &Capture) {
    let (w, h) = (c.w as f32, c.h as f32);
    set_overlay_opacity(old, 1.0);
    if let Some(n) = new {
        set_overlay_opacity(n, 0.0);
    }
    let start = Instant::now();
    let total = SCALE_MS.as_secs_f64();
    loop {
        if ANIM_M.lock().unwrap().interrupt {
            break;
        }
        let t = (start.elapsed().as_secs_f64() / total).min(1.0);
        let e = if t < 0.5 {
            4.0 * t * t * t
        } else {
            1.0 - (-2.0 * t + 2.0).powi(3) / 2.0
        };
        let e = e as f32;
        let s_old = 1.0 + 0.004 * e;
        let s_new = 0.996 + 0.004 * e;
        let m_old = center_scale_matrix(s_old, w, h);
        let m_new = center_scale_matrix(s_new, w, h);
        let fade_t = (t / 0.5).min(1.0);
        let fade = if fade_t < 0.5 {
            4.0 * fade_t * fade_t * fade_t
        } else {
            1.0 - (-2.0 * fade_t + 2.0).powi(3) / 2.0
        };
        let fade = fade as f32;
        set_overlay_opacity(old, 1.0 - fade);
        if let Some(n) = new {
            set_overlay_opacity(n, fade);
        }
        let ok = unsafe {
            let a = old
                .visual
                .SetTransform2(&m_old)
                .and_then(|_| old.device.Commit());
            let b = match new {
                Some(n) => n
                    .visual
                    .SetTransform2(&m_new)
                    .and_then(|_| n.device.Commit()),
                None => Ok(()),
            };
            a.and(b).is_ok()
        };
        if !ok {
            break;
        }
        unsafe {
            if DwmFlush().is_err() {
                thread::sleep(Duration::from_millis(6));
            }
        }
        if t >= 1.0 {
            break;
        }
    }
    set_overlay_opacity(old, 0.0);
    if let Some(n) = new {
        set_overlay_opacity(n, 1.0);
    }
}

fn run_sequence(cap: Option<Capture>, net: i32) {
    unsafe {
        let ov_old = match &cap {
            Some(c) => create_overlay(c, true),
            None => None,
        };
        if let Some(o) = &ov_old {
            thread::sleep(Duration::from_millis(8));
            if pin_window(o.hwnd).is_err() {
                thread::sleep(Duration::from_millis(8));
                let _ = pin_window(o.hwnd);
            }
            let _ = SetWindowDisplayAffinity(o.hwnd, WDA_EXCLUDEFROMCAPTURE);
        }
        if let Err(e) = switch_offset(net) {
            eprintln!("[helper] error cambiando de escritorio: {e}");
        }
        let ov_new = if ov_old.is_some() {
            thread::sleep(GRACE_MS);
            pump();
            let nc = capture_screen();
            let on = match &nc {
                Some(c) => create_overlay(c, false),
                None => None,
            };
            if let Some(c) = nc {
                free_capture(c);
            }
            if let Some(o) = &ov_old {
                let _ = SetWindowDisplayAffinity(o.hwnd, WDA_NONE);
            }
            on
        } else {
            None
        };
        if let (Some(oold), Some(onew)) = (&ov_old, &ov_new) {
            let _ = SetWindowPos(
                oold.hwnd,
                HWND_TOP,
                0,
                0,
                0,
                0,
                SWP_NOMOVE | SWP_NOSIZE | SWP_NOACTIVATE,
            );
            let _ = ShowWindow(onew.hwnd, SW_SHOWNOACTIVATE);
            pump();
            thread::sleep(Duration::from_millis(8));
            if pin_window(onew.hwnd).is_err() {
                thread::sleep(Duration::from_millis(8));
                let _ = pin_window(onew.hwnd);
            }
            let _ = SetWindowPos(
                oold.hwnd,
                HWND_TOP,
                0,
                0,
                0,
                0,
                SWP_NOMOVE | SWP_NOSIZE | SWP_NOACTIVATE,
            );
        }
        if let (Some(oold), Some(c)) = (&ov_old, &cap) {
            if ANIM_KIND.load(Ordering::SeqCst) == ANIM_SCALE {
                animate_scale(oold, ov_new.as_ref(), c);
            } else {
                animate_macos(oold, ov_new.as_ref(), c, net);
            }
        }
        for o in [ov_old, ov_new].into_iter().flatten() {
            let Overlay {
                hwnd,
                target,
                visual,
                opacity,
                surface,
                device,
            } = o;
            drop((target, visual, opacity, surface, device));
            let _ = unpin_window(hwnd);
            let _ = DestroyWindow(hwnd);
        }
        if let Some(c) = cap {
            free_capture(c);
        }
    }
}

fn anim_thread() {
    loop {
        let (cap, net) = {
            let mut g = ANIM_M.lock().unwrap();
            loop {
                if let Some(s) = g.slot.take() {
                    g.running = true;
                    g.interrupt = false;
                    break s;
                }
                g = ANIM_CV.wait(g).unwrap();
            }
        };
        run_sequence(cap, net);
        let mut g = ANIM_M.lock().unwrap();
        g.running = false;
        ANIM_CV.notify_all();
    }
}

fn native_switch(net: i32) {
    if net == 0 {
        return;
    }
    let trigger = *TRIGGER_KEY.lock().unwrap();
    let lift = matches!(
        trigger,
        Key::Alt | Key::AltGr | Key::ShiftLeft | Key::ShiftRight
    );
    let arrow = if net > 0 { Key::LeftArrow } else { Key::RightArrow };
    let steps = net.unsigned_abs() as usize;
    for i in 0..steps {
        PENDING.store(if lift { 8 } else { 6 }, Ordering::SeqCst);
        let _ = simulate(&EventType::KeyPress(Key::ControlLeft));
        if lift {
            let _ = simulate(&EventType::KeyRelease(trigger));
        }
        let _ = simulate(&EventType::KeyPress(Key::MetaLeft));
        let _ = simulate(&EventType::KeyPress(arrow));
        let _ = simulate(&EventType::KeyRelease(arrow));
        let _ = simulate(&EventType::KeyRelease(Key::MetaLeft));
        let _ = simulate(&EventType::KeyRelease(Key::ControlLeft));
        if lift {
            let _ = simulate(&EventType::KeyPress(trigger));
        }
        if i + 1 < steps {
            thread::sleep(Duration::from_millis(60));
        }
    }
}

fn submit_switch(net: i32) {
    if net == 0 {
        return;
    }
    if get_desktops().map(|d| d.len()).unwrap_or(0) <= 1 {
        return;
    }

    if ANIMATIONS.load(Ordering::SeqCst) && ANIM_KIND.load(Ordering::SeqCst) == ANIM_WINDOWS {
        native_switch(net);
        return;
    }

    let mut g = ANIM_M.lock().unwrap();
    g.interrupt = true;
    while g.running {
        let (ng, timeout) = ANIM_CV
            .wait_timeout(g, Duration::from_millis(500))
            .unwrap();
        g = ng;
        if timeout.timed_out() && g.running {
            g.running = false;
            break;
        }
    }
    if let Some((old, _)) = g.slot.take() {
        if let Some(c) = old {
            unsafe { free_capture(c) };
        }
    }
    drop(g);

    let cap = if ANIMATIONS.load(Ordering::SeqCst) {
        unsafe { capture_screen() }
    } else {
        None
    };

    let mut g = ANIM_M.lock().unwrap();
    g.slot = Some((cap, net));
    ANIM_CV.notify_all();
}

fn cancel_win_up() {
    WIN_UP_GEN.fetch_add(1, Ordering::SeqCst);
}

fn schedule_win_up(key: Key) {
    let generation = WIN_UP_GEN.fetch_add(1, Ordering::SeqCst) + 1;
    thread::spawn(move || {
        thread::sleep(TAP_UP_DELAY_MS);
        if WIN_UP_GEN.load(Ordering::SeqCst) != generation || TRIGGER_DOWN.load(Ordering::SeqCst) {
            return;
        }
        PENDING.store(1, Ordering::SeqCst);
        let _ = simulate(&EventType::KeyRelease(key));
    });
}

fn reset_trigger_state() {
    cancel_win_up();
    TRIGGER_DOWN.store(false, Ordering::SeqCst);
    WHEEL_USED.store(false, Ordering::SeqCst);
    OTHER_KEY_USED.store(false, Ordering::SeqCst);
    PENDING.store(0, Ordering::SeqCst);
    DOUBLE_ARMED.store(false, Ordering::SeqCst);
    *LAST_PRESS.lock().unwrap() = None;
}

fn open_task_view() {
    PENDING.store(4, Ordering::SeqCst);
    let _ = simulate(&EventType::KeyPress(Key::MetaLeft));
    let _ = simulate(&EventType::KeyPress(Key::Tab));
    let _ = simulate(&EventType::KeyRelease(Key::Tab));
    let _ = simulate(&EventType::KeyRelease(Key::MetaLeft));
    thread::sleep(Duration::from_millis(50));
    PENDING.store(0, Ordering::SeqCst);
}

fn release_dance(key: Key) {
    PENDING.store(3, Ordering::SeqCst);
    let _ = simulate(&EventType::KeyPress(Key::ControlLeft));
    let _ = simulate(&EventType::KeyRelease(Key::ControlLeft));
    let _ = simulate(&EventType::KeyRelease(key));
    thread::sleep(Duration::from_millis(50));
    PENDING.store(0, Ordering::SeqCst);
}

fn listener_keyboard(trigger: SharedKey) {
    let (tx, rx) = mpsc::channel::<Msg>();

    thread::spawn(move || {
        use mpsc::RecvTimeoutError;
        let mut net: i32 = 0;

        'outer: loop {
            let mut msg = match rx.recv() {
                Ok(m) => m,
                Err(_) => break,
            };
            let start = Instant::now();

            loop {
                match msg {
                    Msg::Switch { up } => {
                        net += if up { 1 } else { -1 };

                        let timeout =
                            COALESCE_QUIET.min(COALESCE_MAX.saturating_sub(start.elapsed()));

                        if timeout.is_zero() {
                            submit_switch(net);
                            net = 0;
                            continue 'outer;
                        }
                        match rx.recv_timeout(timeout) {
                            Ok(m) => msg = m,
                            Err(RecvTimeoutError::Timeout) => {
                                submit_switch(net);
                                net = 0;
                                continue 'outer;
                            }
                            Err(RecvTimeoutError::Disconnected) => {
                                submit_switch(net);
                                break 'outer;
                            }
                        }
                    }
                    Msg::TaskView => {
                        submit_switch(net);
                        net = 0;
                        open_task_view();
                        continue 'outer;
                    }
                    Msg::ReleaseTrigger(key) => {
                        submit_switch(net);
                        net = 0;
                        release_dance(key);
                        continue 'outer;
                    }
                }
            }
        }
    });

    thread::spawn(move || {
        let callback = move |event: Event| -> Option<Event> {

            if matches!(event.event_type, EventType::KeyPress(_) | EventType::KeyRelease(_))
                && PENDING.load(Ordering::SeqCst) > 0
            {
                PENDING.fetch_sub(1, Ordering::SeqCst);
                return Some(event);
            }

            match event.event_type {
                EventType::KeyPress(k) if same_key(*trigger.read().unwrap(), k) => {
                    let first_down = !TRIGGER_DOWN.swap(true, Ordering::SeqCst);
                    if first_down {
                        cancel_win_up();
                        WHEEL_USED.store(false, Ordering::SeqCst);
                        OTHER_KEY_USED.store(false, Ordering::SeqCst);
                        let now = Instant::now();
                        let mut last = LAST_PRESS.lock().unwrap();
                        if let Some(prev) = *last {
                            if now.duration_since(prev) <= DOUBLE_TAP_MS {
                                DOUBLE_ARMED.store(true, Ordering::SeqCst);
                            }
                        }
                        *last = Some(now);
                    }
                    Some(event)
                }
                EventType::KeyRelease(k) if same_key(*trigger.read().unwrap(), k) => {
                    TRIGGER_DOWN.store(false, Ordering::SeqCst);
                    let wheel = WHEEL_USED.swap(false, Ordering::SeqCst);
                    let combo = OTHER_KEY_USED.swap(false, Ordering::SeqCst);
                    let double =
                        DOUBLE_ARMED.swap(false, Ordering::SeqCst) && TASKVIEW.load(Ordering::SeqCst);
                    let swallow_win = is_win_key(k) && !combo;

                    if wheel {
                        *LAST_PRESS.lock().unwrap() = None;
                        let _ = tx.send(Msg::ReleaseTrigger(k));
                        None
                    } else if double {
                        let _ = tx.send(Msg::TaskView);
                        if swallow_win { None } else { Some(event) }
                    } else if swallow_win {
                        if TASKVIEW.load(Ordering::SeqCst) {
                            schedule_win_up(k);
                            None
                        } else {
                            Some(event)
                        }
                    } else {
                        Some(event)
                    }
                }
                EventType::KeyPress(_) if TRIGGER_DOWN.load(Ordering::SeqCst) => {
                    OTHER_KEY_USED.store(true, Ordering::SeqCst);
                    Some(event)
                }
                EventType::Wheel { delta_y, .. } if TRIGGER_DOWN.load(Ordering::SeqCst) => {
                    WHEEL_USED.store(true, Ordering::SeqCst);
                    let _ = tx.send(Msg::Switch { up: delta_y > 0 });
                    None
                }
                _ => Some(event),
            }
        };

        if let Err(error) = grab(callback) {
            eprintln!("[helper] error instalando el hook: {:?}", error);
        }
    });
}

fn main() {
    unsafe {
        if SetProcessDpiAwarenessContext(DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2).is_err() {
            let _ = SetProcessDPIAware();
        }
    }

    let trigger: SharedKey = Arc::new(RwLock::new(Key::MetaLeft));
    thread::spawn(anim_thread);
    listener_keyboard(Arc::clone(&trigger));

    let stdin = io::stdin();
    for line in stdin.lock().lines() {
        let Ok(line) = line else { break };
        let line = line.trim();
        if let Some(rest) = line.strip_prefix("trigger ") {
            if let Some(key) = key_from_str(rest.trim()) {
                *trigger.write().unwrap() = key;
                *TRIGGER_KEY.lock().unwrap() = key;
                reset_trigger_state();
            }
        } else if let Some(rest) = line.strip_prefix("animations ") {
            ANIMATIONS.store(rest.trim() == "1", Ordering::SeqCst);
        } else if let Some(rest) = line.strip_prefix("anim ") {
            let kind = rest.trim();
            ANIM_KIND.store(
                if kind == "windows" {
                    ANIM_WINDOWS
                } else if kind == "scale" {
                    ANIM_SCALE
                } else {
                    ANIM_MACOS
                },
                Ordering::SeqCst,
            );
        } else if let Some(rest) = line.strip_prefix("taskview ") {
            TASKVIEW.store(rest.trim() == "1", Ordering::SeqCst);
        }
    }

    std::process::exit(0);
}
