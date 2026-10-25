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

fn default_trigger() -> String {
    DEFAULT_TRIGGER.to_string()
}

fn default_true() -> bool {
    true
}

fn default_animation() -> String {
    "macos".to_string()
}

fn normalize_animation(s: &mut String) {
    if s != "macos" && s != "windows" && s != "scale" {
        *s = default_animation();
    }
}

#[derive(Clone, serde::Serialize, serde::Deserialize)]
struct Settings {
    #[serde(default = "default_trigger")]
    trigger: String,
    #[serde(default = "default_true")]
    animations: bool,
    #[serde(default = "default_true")]
    taskview: bool,
    #[serde(default = "default_animation")]
    animation: String,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            trigger: default_trigger(),
            animations: true,
            taskview: true,
            animation: default_animation(),
        }
    }
}

fn settings_file(app: &AppHandle) -> Option<PathBuf> {
    let dir = app.path().app_config_dir().ok()?;
    Some(dir.join("settings.json"))
}

fn load_settings(app: &AppHandle) -> (Settings, bool) {
    let Some(path) = settings_file(app) else {
        return (Settings::default(), true);
    };
    match std::fs::read_to_string(&path) {
        Ok(content) => {
            let mut parsed: Settings = serde_json::from_str(&content).unwrap_or_default();
            if key_from_str(&parsed.trigger).is_none() {
                parsed.trigger = default_trigger();
            }
            normalize_animation(&mut parsed.animation);
            (parsed, false)
        }
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => (Settings::default(), true),
        Err(_) => (Settings::default(), false),
    }
}

fn save_settings(app: &AppHandle, settings: &Settings) {
    if let Some(path) = settings_file(app) {
        if let Some(parent) = path.parent() {
            let _ = std::fs::create_dir_all(parent);
        }
        let json = serde_json::to_string(settings).unwrap_or_default();
        let _ = std::fs::write(path, json);
    }
}

struct Helper {
    #[allow(dead_code)]
    child: Child,
    stdin: BufWriter<ChildStdin>,
}

type HelperState = Arc<RwLock<Option<Helper>>>;
type SettingsState = Arc<RwLock<Settings>>;

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

fn helper_exe() -> Option<PathBuf> {
    let exe = std::env::current_exe().ok()?;
    Some(exe.parent()?.join("hook_helper.exe"))
}

fn spawn_helper(settings: &Settings) -> Option<Helper> {
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
    let _ = writeln!(helper.stdin, "trigger {}", settings.trigger);
    let _ = writeln!(helper.stdin, "animations {}", if settings.animations { "1" } else { "0" });
    let _ = writeln!(helper.stdin, "anim {}", settings.animation);
    let _ = writeln!(helper.stdin, "taskview {}", if settings.taskview { "1" } else { "0" });
    let _ = helper.stdin.flush();
    Some(helper)
}

fn push_helper(state: &HelperState, settings: &Settings, line: &str) {
    let mut guard = state.write().unwrap();
    match guard.as_mut() {
        Some(helper) => {
            let _ = writeln!(helper.stdin, "{line}");
            let _ = helper.stdin.flush();
        }
        None => *guard = spawn_helper(settings),
    }
}

#[tauri::command]
fn get_settings(settings: State<SettingsState>) -> Settings {
    settings.read().unwrap().clone()
}

#[tauri::command]
fn set_trigger(
    key: String,
    app: AppHandle,
    settings: State<SettingsState>,
    helper: State<HelperState>,
) -> Result<(), String> {
    key_from_str(&key).ok_or_else(|| format!("Tecla no soportada: {key}"))?;
    let snapshot = {
        let mut guard = settings.write().unwrap();
        guard.trigger = key.clone();
        save_settings(&app, &guard);
        guard.clone()
    };
    push_helper(&helper, &snapshot, &format!("trigger {key}"));
    Ok(())
}

#[tauri::command]
fn set_animations(
    enabled: bool,
    app: AppHandle,
    settings: State<SettingsState>,
    helper: State<HelperState>,
) {
    let snapshot = {
        let mut guard = settings.write().unwrap();
        guard.animations = enabled;
        save_settings(&app, &guard);
        guard.clone()
    };
    push_helper(&helper, &snapshot, &format!("animations {}", if enabled { "1" } else { "0" }));
}

#[tauri::command]
fn set_animation(
    kind: String,
    app: AppHandle,
    settings: State<SettingsState>,
    helper: State<HelperState>,
) -> Result<(), String> {
    let kind = if kind == "macos" || kind == "windows" || kind == "scale" {
        kind
    } else {
        return Err(format!("Animación no soportada: {kind}"));
    };
    let snapshot = {
        let mut guard = settings.write().unwrap();
        guard.animation = kind.clone();
        save_settings(&app, &guard);
        guard.clone()
    };
    push_helper(&helper, &snapshot, &format!("anim {kind}"));
    Ok(())
}

#[tauri::command]
fn set_taskview(
    enabled: bool,
    app: AppHandle,
    settings: State<SettingsState>,
    helper: State<HelperState>,
) {
    let snapshot = {
        let mut guard = settings.write().unwrap();
        guard.taskview = enabled;
        save_settings(&app, &guard);
        guard.clone()
    };
    push_helper(&helper, &snapshot, &format!("taskview {}", if enabled { "1" } else { "0" }));
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
            let (settings, _) = load_settings(app.handle());

            app.manage(Arc::new(RwLock::new(settings.clone())));
            app.manage(Arc::new(RwLock::new(spawn_helper(&settings))));

            if cfg!(not(debug_assertions)) {
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
                .tooltip("Spaces")
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
        .invoke_handler(tauri::generate_handler![
            get_settings,
            set_trigger,
            set_animations,
            set_animation,
            set_taskview,
            set_lang
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
