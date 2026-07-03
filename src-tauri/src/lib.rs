mod actions;
mod activator;
mod config;
mod display;
mod gamepad;
mod hotkeys;
mod state;
mod startup;
mod steam;
mod telemetry;
mod window_chrome;

use std::collections::HashMap;
use std::fs;
use std::sync::Arc;

use parking_lot::Mutex;
use tauri::{
    menu::{MenuBuilder, MenuItemBuilder},
    tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent},
    Emitter, Listener, Manager,
};
use crate::activator::{activate_group, save_group_layout};
use crate::config::{
    group_has_layout, is_group_activatable, load_config, new_group_id, save_config, AppConfig,
    DisplayGroup, PostAction,
};
use crate::gamepad::GamepadManager;
use crate::state::AppState;

#[tauri::command]
fn get_config(state: tauri::State<'_, Arc<AppState>>) -> AppConfig {
    state.get_config()
}

#[tauri::command]
fn sync_window_chrome(app: AppHandle, theme: String) -> Result<(), String> {
    if let Some(window) = app.get_webview_window("main") {
        window_chrome::apply_theme(&window, &theme)?;
    }
    Ok(())
}

#[tauri::command]
fn resolve_theme_setting(theme: String) -> String {
    window_chrome::resolve_theme(&theme).to_string()
}

#[tauri::command]
fn save_app_config(
    config: AppConfig,
    app: AppHandle,
    state: tauri::State<'_, Arc<AppState>>,
) -> Result<(), String> {
    save_config(&config)?;
    state.set_config(config.clone());
    refresh_hotkeys_and_tray(&app, &config)?;
    startup::sync_launch_on_startup(&app, config.settings.launch_on_startup)?;
    Ok(())
}

#[tauri::command]
fn list_displays() -> Result<Vec<display::DisplayInfo>, String> {
    display::list_displays()
}

#[tauri::command]
fn list_group_layout_status(
    state: tauri::State<'_, Arc<AppState>>,
) -> Result<HashMap<String, bool>, String> {
    let config = state.get_config();
    Ok(config
        .groups
        .iter()
        .map(|group| (group.id.clone(), group_has_layout(group)))
        .collect())
}

#[tauri::command]
fn save_group_layout_cmd(
    group_id: String,
    state: tauri::State<'_, Arc<AppState>>,
) -> Result<(), String> {
    let config = state.get_config();
    let group = config
        .groups
        .iter()
        .find(|g| g.id == group_id)
        .ok_or_else(|| "Group not found".to_string())?
        .clone();
    save_group_layout(&group)
}

#[tauri::command]
fn activate_group_cmd(
    group_id: String,
    trigger: String,
    app: AppHandle,
    state: tauri::State<'_, Arc<AppState>>,
) -> Result<telemetry::SwitchRecord, String> {
    let config = state.get_config();
    let group = config
        .groups
        .iter()
        .find(|g| g.id == group_id)
        .ok_or_else(|| "Group not found".to_string())?
        .clone();

    let result = activate_group(&config, &group, &trigger);
    let _ = app.emit("activation-complete", &result.record);
    if !result.record.success {
        return Err(result.record.error.unwrap_or_else(|| "Activation failed".into()));
    }
    Ok(result.record)
}

#[tauri::command]
fn create_group(
    name: String,
    state: tauri::State<'_, Arc<AppState>>,
    app: AppHandle,
) -> Result<DisplayGroup, String> {
    let mut config = state.get_config();
    let id = new_group_id();
    let group = DisplayGroup {
        id: id.clone(),
        name,
        display_ids: vec![],
        profile_file: format!("profiles/{}.json", id),
        hotkey: None,
        gamepad_chord: None,
        post_action: PostAction::Builtin {
            action: "none".into(),
        },
    };
    config.groups.push(group.clone());
    save_config(&config)?;
    state.set_config(config.clone());
    refresh_hotkeys_and_tray(&app, &config)?;
    Ok(group)
}

#[tauri::command]
fn delete_group(
    group_id: String,
    state: tauri::State<'_, Arc<AppState>>,
    app: AppHandle,
) -> Result<(), String> {
    let mut config = state.get_config();
    if let Some(group) = config.groups.iter().find(|g| g.id == group_id) {
        let path = config::resolve_profile_path(&group.profile_file);
        let _ = fs::remove_file(path);
    }
    config.groups.retain(|g| g.id != group_id);
    save_config(&config)?;
    state.set_config(config.clone());
    refresh_hotkeys_and_tray(&app, &config)?;
    Ok(())
}

#[tauri::command]
fn update_group(
    group: DisplayGroup,
    state: tauri::State<'_, Arc<AppState>>,
    app: AppHandle,
) -> Result<(), String> {
    let mut config = state.get_config();
    if let Some(existing) = config.groups.iter_mut().find(|g| g.id == group.id) {
        *existing = group;
    } else {
        return Err("Group not found".to_string());
    }
    save_config(&config)?;
    state.set_config(config.clone());
    refresh_hotkeys_and_tray(&app, &config)?;
    Ok(())
}

#[tauri::command]
fn get_builtin_actions() -> Vec<(String, String)> {
    actions::BUILTIN_ACTIONS
        .iter()
        .map(|(id, label)| (id.to_string(), label.to_string()))
        .collect()
}

#[tauri::command]
fn get_gamepad_buttons() -> Vec<String> {
    gamepad::list_gamepad_buttons()
        .iter()
        .map(|s| s.to_string())
        .collect()
}

#[tauri::command]
fn get_telemetry_stats() -> Result<telemetry::TelemetryStats, String> {
    telemetry::compute_stats()
}

#[tauri::command]
fn get_telemetry_recent(limit: usize) -> Result<Vec<telemetry::SwitchRecord>, String> {
    telemetry::recent_records(limit)
}

#[tauri::command]
fn clear_telemetry() -> Result<(), String> {
    telemetry::clear()
}

#[tauri::command]
fn export_telemetry(path: String) -> Result<(), String> {
    telemetry::export_to(&path)
}

#[tauri::command]
fn complete_onboarding(state: tauri::State<'_, Arc<AppState>>) -> Result<(), String> {
    let mut config = state.get_config();
    config.onboarding_complete = true;
    save_config(&config)?;
    state.set_config(config);
    Ok(())
}

fn refresh_hotkeys_and_tray(app: &AppHandle, config: &AppConfig) -> Result<(), String> {
    let hotkeys: Vec<(String, String)> = config
        .groups
        .iter()
        .filter(|g| is_group_activatable(g))
        .filter_map(|g| g.hotkey.as_ref().map(|h| (g.id.clone(), h.clone())))
        .collect();
    hotkeys::register_group_hotkeys(app, hotkeys)?;
    rebuild_tray(app, config)?;
    Ok(())
}

fn rebuild_tray(app: &AppHandle, config: &AppConfig) -> Result<(), String> {
    let configure = MenuItemBuilder::with_id("configure", "Configure")
        .build(app)
        .map_err(|e| e.to_string())?;
    let exit = MenuItemBuilder::with_id("exit", "Exit")
        .build(app)
        .map_err(|e| e.to_string())?;

    let mut menu_builder = MenuBuilder::new(app);
    for group in config.groups.iter().filter(|g| is_group_activatable(g)) {
        let item = MenuItemBuilder::with_id(format!("group-{}", group.id), &group.name)
            .build(app)
            .map_err(|e| e.to_string())?;
        menu_builder = menu_builder.item(&item);
    }
    menu_builder = menu_builder.separator().item(&configure).item(&exit);
    let menu = menu_builder.build().map_err(|e| e.to_string())?;

    if let Some(tray) = app.tray_by_id("main") {
        tray.set_menu(Some(menu)).map_err(|e| e.to_string())?;
    }

    Ok(())
}

fn setup_tray(app: &AppHandle, config: &AppConfig) -> Result<(), String> {
    let configure = MenuItemBuilder::with_id("configure", "Configure")
        .build(app)
        .map_err(|e| e.to_string())?;
    let exit = MenuItemBuilder::with_id("exit", "Exit")
        .build(app)
        .map_err(|e| e.to_string())?;

    let mut menu_builder = MenuBuilder::new(app);
    for group in config.groups.iter().filter(|g| is_group_activatable(g)) {
        let item = MenuItemBuilder::with_id(format!("group-{}", group.id), &group.name)
            .build(app)
            .map_err(|e| e.to_string())?;
        menu_builder = menu_builder.item(&item);
    }
    menu_builder = menu_builder.separator().item(&configure).item(&exit);
    let menu = menu_builder.build().map_err(|e| e.to_string())?;

    let _tray = TrayIconBuilder::with_id("main")
        .icon(
            app.default_window_icon()
                .ok_or_else(|| "Missing app icon".to_string())?
                .clone(),
        )
        .tooltip("Display Switcher")
        .menu(&menu)
        .on_menu_event(|app, event| {
            let id = event.id().0.as_str();
            if id == "configure" {
                show_main_window(app);
            } else if id == "exit" {
                app.exit(0);
            } else if let Some(group_id) = id.strip_prefix("group-") {
                let _ = app.emit("activate-group", group_id.to_string());
            }
        })
        .on_tray_icon_event(|tray, event| {
            if let TrayIconEvent::Click {
                button: MouseButton::Left,
                button_state: MouseButtonState::Up,
                ..
            } = event
            {
                show_main_window(tray.app_handle());
            }
        })
        .build(app)
        .map_err(|e| e.to_string())?;

    Ok(())
}

fn show_main_window(app: &AppHandle) {
    if let Some(window) = app.get_webview_window("main") {
        let _ = window.show();
        let _ = window.unminimize();
        let _ = window.set_focus();
    }
}

#[cfg(windows)]
fn ensure_single_instance() -> bool {
    use windows::core::w;
    use windows::Win32::Foundation::{GetLastError, ERROR_ALREADY_EXISTS};
    use windows::Win32::System::Threading::CreateMutexW;

    unsafe {
        let _ = CreateMutexW(None, true, w!("Global\\DisplaySwitcherSingleInstance"));
        GetLastError() != ERROR_ALREADY_EXISTS
    }
}

#[cfg(not(windows))]
fn ensure_single_instance() -> bool {
    true
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    if !ensure_single_instance() {
        eprintln!("Display Switcher is already running.");
        std::process::exit(0);
    }

    let activation_state: Arc<Mutex<Option<Arc<AppState>>>> = Arc::new(Mutex::new(None));

    let gamepad = GamepadManager::new({
        let activation_state = activation_state.clone();
        move |group_id| {
            if let Some(state) = activation_state.lock().as_ref() {
                let config = state.get_config();
                if let Some(group) = config.groups.iter().find(|g| g.id == group_id) {
                    let _ = activate_group(&config, group, "gamepad");
                }
            }
        }
    });

    let app_state = AppState::new(gamepad);
    let loaded = load_config();
    app_state.set_config(loaded);
    *activation_state.lock() = Some(app_state.clone());

    app_state.gamepad.start();

    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_global_shortcut::Builder::new().build())
        .plugin(tauri_plugin_autostart::init(
            tauri_plugin_autostart::MacosLauncher::LaunchAgent,
            Some(vec![startup::STARTUP_ARG]),
        ))
        .manage(app_state.clone())
        .invoke_handler(tauri::generate_handler![
            get_config,
            save_app_config,
            list_displays,
            list_group_layout_status,
            save_group_layout_cmd,
            activate_group_cmd,
            create_group,
            delete_group,
            update_group,
            get_builtin_actions,
            get_gamepad_buttons,
            get_telemetry_stats,
            get_telemetry_recent,
            clear_telemetry,
            export_telemetry,
            complete_onboarding,
            sync_window_chrome,
            resolve_theme_setting,
        ])
        .setup(move |app| {
            let config = app_state.get_config();
            setup_tray(app.handle(), &config)?;
            refresh_hotkeys_and_tray(app.handle(), &config)?;
            startup::sync_launch_on_startup(app.handle(), config.settings.launch_on_startup)?;

            let app_handle = app.handle().clone();
            let emit_handle = app_handle.clone();
            let state_clone = app_state.clone();
            let _ = app_handle.listen("activate-group", move |event| {
                if let Ok(group_id) = serde_json::from_str::<String>(event.payload()) {
                    let config = state_clone.get_config();
                    if let Some(group) = config.groups.iter().find(|g| g.id == group_id) {
                        if !is_group_activatable(group) {
                            return;
                        }
                        if !group_has_layout(group) {
                            let record = telemetry::make_record(
                                &group.id,
                                &group.name,
                                "hotkey",
                                0,
                                0,
                                0,
                                false,
                                Some(
                                    "No layout saved for this group. Open the group and click Save layout."
                                        .into(),
                                ),
                            );
                            let _ = emit_handle.emit("activation-complete", &record);
                            return;
                        }
                        let result = activate_group(&config, group, "hotkey");
                        let _ = emit_handle.emit("activation-complete", &result.record);
                    }
                }
            });

            if let Some(window) = app.get_webview_window("main") {
                let theme = config.settings.theme.clone();
                let _ = window_chrome::apply_theme(&window, &theme);

                let win = window.clone();
                window.on_window_event(move |event| {
                    if let tauri::WindowEvent::CloseRequested { api, .. } = event {
                        // Keep the daemon alive for hotkeys, gamepad, and tray.
                        api.prevent_close();
                        let _ = win.hide();
                    }
                });

                if startup::should_start_hidden(config.settings.minimize_to_tray) {
                    let _ = window.hide();
                }
            }

            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

use tauri::AppHandle;