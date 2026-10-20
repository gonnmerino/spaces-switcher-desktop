#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use std::io::{self, BufRead};
use std::sync::atomic::{AtomicBool, AtomicU8, Ordering};
use std::sync::{mpsc, Arc, RwLock};
use std::thread;
use std::time::Duration;
use rdev::{grab, simulate, Event, EventType, Key};
use winvd::{get_current_desktop, get_desktops, switch_desktop};

const DEBUG: bool = false;

type SharedKey = Arc<RwLock<Key>>;

static TRIGGER_DOWN: AtomicBool = AtomicBool::new(false);
static WHEEL_USED: AtomicBool = AtomicBool::new(false);
static PENDING: AtomicU8 = AtomicU8::new(0);

enum Msg {
    Switch(bool),
    ReleaseTrigger(Key),
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

fn switch_relative(scroll_up: bool) -> Result<(), String> {
    let desktops = get_desktops().map_err(|e| format!("{:?}", e))?;
    let count = desktops.len();
    if count <= 1 {
        return Ok(());
    }
    let current = get_current_desktop()
        .and_then(|d| d.get_index())
        .map_err(|e| format!("{:?}", e))? as usize;

    let target = if scroll_up { (current + count - 1) % count } else { (current + 1) % count };
    switch_desktop(desktops[target].clone()).map_err(|e| format!("{:?}", e))
}

fn listener_keyboard(trigger: SharedKey) {
    let (tx, rx) = mpsc::channel::<Msg>();

    thread::spawn(move || {
        while let Ok(msg) = rx.recv() {
            match msg {
                Msg::Switch(scroll_up) => match switch_relative(scroll_up) {
                    Ok(()) => {
                        if DEBUG {
                            eprintln!("[helper] cambio de escritorio ok (arriba: {scroll_up})");
                        }
                    }
                    Err(e) => eprintln!("[helper] error cambiando de escritorio: {e}"),
                },
                Msg::ReleaseTrigger(key) => {
                    PENDING.store(3, Ordering::SeqCst);
                    let _ = simulate(&EventType::KeyPress(Key::ControlLeft));
                    let _ = simulate(&EventType::KeyRelease(Key::ControlLeft));
                    let _ = simulate(&EventType::KeyRelease(key));
                    thread::sleep(Duration::from_millis(50));
                    PENDING.store(0, Ordering::SeqCst);
                }
            }
        }
    });

    thread::spawn(move || {
        let callback = move |event: Event| -> Option<Event> {
            if DEBUG
                && matches!(
                    event.event_type,
                    EventType::KeyPress(_) | EventType::KeyRelease(_) | EventType::Wheel { .. }
                )
            {
                eprintln!("[helper][hook] {:?}", event.event_type);
            }

            if matches!(event.event_type, EventType::KeyPress(_) | EventType::KeyRelease(_))
                && PENDING.load(Ordering::SeqCst) > 0
            {
                PENDING.fetch_sub(1, Ordering::SeqCst);
                return Some(event);
            }

            match event.event_type {
                EventType::KeyPress(k) if same_key(*trigger.read().unwrap(), k) => {
                    if !TRIGGER_DOWN.swap(true, Ordering::SeqCst) {
                        WHEEL_USED.store(false, Ordering::SeqCst);
                    }
                    Some(event)
                }
                EventType::KeyRelease(k) if same_key(*trigger.read().unwrap(), k) => {
                    TRIGGER_DOWN.store(false, Ordering::SeqCst);
                    if WHEEL_USED.swap(false, Ordering::SeqCst) {
                        let _ = tx.send(Msg::ReleaseTrigger(k));
                        None
                    } else {
                        Some(event)
                    }
                }
                EventType::Wheel { delta_y, .. } if TRIGGER_DOWN.load(Ordering::SeqCst) => {
                    WHEEL_USED.store(true, Ordering::SeqCst);
                    let _ = tx.send(Msg::Switch(delta_y > 0));
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
    let trigger: SharedKey = Arc::new(RwLock::new(Key::MetaLeft));
    listener_keyboard(Arc::clone(&trigger));

    let stdin = io::stdin();
    for line in stdin.lock().lines() {
        let Ok(line) = line else { break };
        let line = line.trim();
        if let Some(rest) = line.strip_prefix("trigger ") {
            if let Some(key) = key_from_str(rest.trim()) {
                *trigger.write().unwrap() = key;
                TRIGGER_DOWN.store(false, Ordering::SeqCst);
                WHEEL_USED.store(false, Ordering::SeqCst);
                PENDING.store(0, Ordering::SeqCst);
                if DEBUG {
                    eprintln!("[helper] trigger actualizado a {rest}");
                }
            } else if DEBUG {
                eprintln!("[helper] tecla no soportada: {rest}");
            }
        }
    }

    std::process::exit(0);
}
