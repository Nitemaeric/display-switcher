use std::fs;
use std::path::Path;
use std::time::Instant;

use crate::actions;
use crate::config::{resolve_profile_path, AppConfig, DisplayGroup};
use crate::display::{self, DisplayProfile};
use crate::telemetry::{self, SwitchRecord};

pub struct ActivationResult {
    pub record: SwitchRecord,
}

pub fn activate_group(
    config: &AppConfig,
    group: &DisplayGroup,
    trigger: &str,
) -> ActivationResult {
    let total_start = Instant::now();
    let profile_path = resolve_profile_path(&group.profile_file);

    let display_start = Instant::now();
    let display_result = apply_group_profile(&profile_path);
    let display_apply_ms = display_start.elapsed().as_millis() as u64;

    let post_start = Instant::now();
    let post_result = if display_result.is_ok() {
        actions::run_post_action(&group.post_action, &config.settings.steam_path)
    } else {
        Ok(())
    };
    let post_action_ms = post_start.elapsed().as_millis() as u64;

    let success = display_result.is_ok() && post_result.is_ok();
    let error = if !display_result.is_ok() {
        display_result.err()
    } else {
        post_result.err()
    };

    let total_ms = total_start.elapsed().as_millis() as u64;
    let record = telemetry::make_record(
        &group.id,
        &group.name,
        trigger,
        display_apply_ms,
        post_action_ms,
        total_ms,
        success,
        error,
    );

    let _ = telemetry::append_record(&record, config.settings.telemetry_retention);

    ActivationResult { record }
}

fn apply_group_profile(path: &Path) -> Result<(), String> {
    if !path.exists() {
        return Err(format!(
            "Profile not found: {}. Save the group layout first.",
            path.display()
        ));
    }

    let content = fs::read_to_string(path).map_err(|e| e.to_string())?;
    let profile: DisplayProfile =
        serde_json::from_str(&content).map_err(|e| format!("Invalid profile: {e}"))?;

    display::apply_profile(&profile)
}

pub fn save_group_layout(group: &DisplayGroup) -> Result<(), String> {
    let profile = display::capture_current_profile()?;
    let path = resolve_profile_path(&group.profile_file);
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }
    let content = serde_json::to_string_pretty(&profile).map_err(|e| e.to_string())?;
    fs::write(path, content).map_err(|e| e.to_string())?;
    Ok(())
}