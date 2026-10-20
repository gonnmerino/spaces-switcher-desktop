use std::io::{BufWriter, Write};
use std::path::PathBuf;
use std::process::{Child, ChildStdin, Command, Stdio};
use std::sync::{Arc, RwLock};
use rdev::Key;
use tauri::{
    menu::{Menu, MenuItem},
    tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent},
    AppHandle, Manager, State, WindowEvent, Wry,
};
use tauri_plugin_autostart::ManagerExt;

const DEFAULT_TRIGGER: &str = "MetaLeft";

fn settings_file(app: &AppHandle) -> Option<PathBuf> {
    let dir = app.path().app_config_dir().ok()?;
    Some(dir.join("settings.json"))
}

fn load_trigger(app: &AppHandle) -> (String, bool) {
    let Some(path) = settings_file(app) else {
        return (DEFAULT_TRIGGER.to_string(), true);
    };
    match std::fs::read_to_string(&path) {
        Ok(content) => {
            let parsed: serde_json::Value = serde_json::from_str(&content).unwrap_or_default();
            let trigger = parsed
                .get("trigger")
                .and_then(|v| v.as_str())
                .filter(|s| key_from_str(s).is_some())
                .unwrap_or(DEFAULT_TRIGGER)
                .to_string();
            (trigger, false)
        }
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
            (DEFAULT_TRIGGER.to_string(), true)
        }
        Err(_) => (DEFAULT_TRIGGER.to_string(), false),
    }
}

fn save_trigger(app: &AppHandle, trigger: &str) {
    if let Some(path) = settings_file(app) {
        if let Some(parent) = path.parent() {
            let _ = std::fs::create_dir_all(parent);
        }
        let json = serde_json::json!({ "trigger": trigger });
        let _ = std::fs::write(path, json.to_string());
    }
}

type SharedKey = Arc<RwLock<Key>>;

struct Helper {
    #[allow(dead_code)]
    child: Child,
    stdin: BufWriter<ChildStdin>,
}

type HelperState = Arc<RwLock<Option<Helper>>>;

struct TrayState {
    open: MenuItem<Wry>,
    quit: MenuItem<Wry>,
}

fn tray_labels(lang: &str) -> (&'static str, &'static str) {
    match lang {
        "en" => ("Settings", "Quit"),
        "pt" => ("Configurações", "Sair"),
        "fr" => ("Paramètres", "Quitter"),
        "de" => ("Einstellungen", "Beenden"),
        "it" => ("Impostazioni", "Esci"),
        "nl" => ("Instellingen", "Afsluiten"),
        "pl" => ("Ustawienia", "Zakończ"),
        "ru" => ("Настройки", "Выход"),
        "ja" => ("設定", "終了"),
        "zh" => ("设置", "退出"),
        "ko" => ("설정", "종료"),
        _ => ("Configuración", "Salir"),
    }
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

fn key_to_str(k: Key) -> String {
    format!("{:?}", k)
}

fn helper_exe() -> Option<PathBuf> {
    let exe = std::env::current_exe().ok()?;
    Some(exe.parent()?.join("hook_helper.exe"))
}

fn spawn_helper(initial_trigger: &str) -> Option<Helper> {
    let path = helper_exe()?;
    let mut child = Command::new(path)
        .stdin(Stdio::piped())
        .stdout(Stdio::null())
        .stderr(Stdio::inherit())
        .spawn()
        .ok()?;
    let stdin = child.stdin.take()?;
    let mut helper = Helper {
        child,
        stdin: BufWriter::new(stdin),
    };
    let _ = writeln!(helper.stdin, "trigger {initial_trigger}");
    let _ = helper.stdin.flush();
    Some(helper)
}

fn send_trigger(helper: &mut Helper, key: &str) {
    let _ = writeln!(helper.stdin, "trigger {key}");
    let _ = helper.stdin.flush();
}

fn update_helper(state: &HelperState, key_str: &str) {
    let mut guard = state.write().unwrap();
    match guard.as_mut() {
        Some(helper) => send_trigger(helper, key_str),
        None => *guard = spawn_helper(key_str),
    }
}

#[tauri::command]
fn get_trigger(state: State<SharedKey>) -> String {
    key_to_str(*state.read().unwrap())
}

#[tauri::command]
fn set_trigger(
    key: String,
    app: AppHandle,
    state: State<SharedKey>,
    helper: State<HelperState>,
) -> Result<(), String> {
    let key_obj = key_from_str(&key).ok_or_else(|| format!("Tecla no soportada: {key}"))?;
    *state.write().unwrap() = key_obj;
    update_helper(&helper, &key);
    save_trigger(&app, &key);
    Ok(())
}

#[tauri::command]
fn set_lang(lang: String, tray: State<TrayState>) -> Result<(), String> {
    let (open, quit) = tray_labels(&lang);
    tray.open.set_text(open).map_err(|e| e.to_string())?;
    tray.quit.set_text(quit).map_err(|e| e.to_string())?;
    Ok(())
}

fn show_window(app: &AppHandle) {
    if let Some(w) = app.get_webview_window("main") {
        let _ = w.show();
        let _ = w.unminimize();
        let _ = w.set_focus();
    }
}

pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_autostart::init(
            tauri_plugin_autostart::MacosLauncher::LaunchAgent,
            None,
        ))
        .setup(|app| {
            let (trigger_str, first_run) = load_trigger(app.handle());
            let key_obj = key_from_str(&trigger_str).unwrap_or(Key::MetaLeft);

            let trigger: SharedKey = Arc::new(RwLock::new(key_obj));
            app.manage(Arc::clone(&trigger));
            app.manage(Arc::new(RwLock::new(spawn_helper(&trigger_str))));

            if first_run {
                let _ = app.handle().autolaunch().enable();
            }

            let open = MenuItem::with_id(app, "open", "Settings", true, None::<&str>)?;
            let quit = MenuItem::with_id(app, "quit", "Quit", true, None::<&str>)?;
            let menu = Menu::with_items(app, &[&open, &quit])?;

            let _tray = TrayIconBuilder::new()
                .icon(
                    tauri::image::Image::from_bytes(include_bytes!("../icons/tray-32.png"))
                        .expect("icono de bandeja inválido"),
                )
                .tooltip("spaces")
                .menu(&menu)
                .show_menu_on_left_click(false)
                .on_menu_event(|app, event| match event.id.as_ref() {
                    "open" => show_window(app),
                    "quit" => app.exit(0),
                    _ => {}
                })
                .on_tray_icon_event(|tray, event| {
                    if let TrayIconEvent::Click {
                        button: MouseButton::Left,
                        button_state: MouseButtonState::Up,
                        ..
                    } = event
                    {
                        show_window(tray.app_handle());
                    }
                })
                .build(app)?;

            app.manage(TrayState { open, quit });

            Ok(())
        })
        .on_window_event(|window, event| {
            if let WindowEvent::CloseRequested { api, .. } = event {
                api.prevent_close();
                let _ = window.hide();
            }
        })
        .invoke_handler(tauri::generate_handler![get_trigger, set_trigger, set_lang])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
