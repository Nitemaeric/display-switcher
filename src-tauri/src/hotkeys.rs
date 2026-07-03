use tauri::{AppHandle, Emitter};
use tauri_plugin_global_shortcut::{GlobalShortcutExt, Shortcut, ShortcutState};

pub fn register_group_hotkeys(app: &AppHandle, hotkeys: Vec<(String, String)>) -> Result<(), String> {
    let gs = app.global_shortcut();
    gs.unregister_all()
        .map_err(|e| format!("Failed to clear hotkeys: {e}"))?;

    for (group_id, hotkey) in hotkeys {
        if hotkey.trim().is_empty() {
            continue;
        }
        let shortcut: Shortcut = hotkey
            .parse()
            .map_err(|e| format!("Invalid hotkey '{hotkey}': {e}"))?;

        let app_clone = app.clone();
        let gid = group_id.clone();
        gs.on_shortcut(shortcut, move |_app, _shortcut, event| {
            if event.state == ShortcutState::Pressed {
                let _ = app_clone.emit("activate-group", gid.clone());
            }
        })
        .map_err(|e| format!("Failed to bind hotkey '{hotkey}': {e}"))?;
    }

    Ok(())
}