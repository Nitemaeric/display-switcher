use tauri::AppHandle;

#[cfg(desktop)]
pub fn sync_launch_on_startup(app: &AppHandle, enabled: bool) -> Result<(), String> {
    use tauri_plugin_autostart::ManagerExt;

    let manager = app.autolaunch();
    if enabled {
        manager.enable().map_err(|e| e.to_string())?;
    } else if manager.is_enabled().unwrap_or(false) {
        // Only remove an existing Run key; deleting a missing value errors on Windows.
        manager.disable().map_err(|e| e.to_string())?;
    }
    Ok(())
}

#[cfg(not(desktop))]
pub fn sync_launch_on_startup(_app: &AppHandle, _enabled: bool) -> Result<(), String> {
    Ok(())
}