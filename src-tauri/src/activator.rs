use std::fs;
use std::path::Path;
use std::time::Instant;

use crate::actions;
use crate::config::{is_group_activatable, resolve_profile_path, AppConfig, DisplayGroup};
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
    let display_result = validate_and_apply_group(group, &profile_path);
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

fn validate_and_apply_group(group: &DisplayGroup, profile_path: &Path) -> Result<(), String> {
    if !is_group_activatable(group) {
        return Err("Group has no displays assigned. Add at least one display before activating.".into());
    }

    if !profile_path.exists() {
        return Err(
            "No layout saved for this group. Open the group, arrange displays in Windows Settings, then click Save layout."
                .into(),
        );
    }

    let content = fs::read_to_string(profile_path).map_err(|e| e.to_string())?;
    let mut profile: DisplayProfile =
        serde_json::from_str(&content).map_err(|e| format!("Invalid profile: {e}"))?;

    display::sanitize_profile_for_group(&mut profile, &group.display_ids)?;
    display::validate_profile_safe(&profile, &group.display_ids)?;
    display::apply_profile(&profile)
}

pub fn save_group_layout(group: &DisplayGroup) -> Result<(), String> {
    let path = resolve_profile_path(&group.profile_file);
    if should_keep_existing_layout(&path, &group.display_ids) {
        return Ok(());
    }

    let mut profile = display::capture_current_profile()?;
    display::sanitize_profile_for_group(&mut profile, &group.display_ids)?;
    display::activate_assigned_displays(&mut profile, &group.display_ids)?;
    display::validate_profile_safe(&profile, &group.display_ids)?;
    display::validate_profile_with_windows(&profile)?;
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }
    let content = serde_json::to_string_pretty(&profile).map_err(|e| e.to_string())?;
    fs::write(path, content).map_err(|e| e.to_string())?;
    Ok(())
}

/// Recapturing while some assigned displays are off would replace a real,
/// user-arranged layout with synthesized native-mode defaults. Keep the saved
/// layout unless the live state can fully describe the group (all displays on)
/// or the saved layout no longer covers the assigned displays.
fn should_keep_existing_layout(profile_path: &Path, display_ids: &[String]) -> bool {
    if display::displays_all_active(display_ids) {
        return false;
    }
    let Ok(content) = fs::read_to_string(profile_path) else {
        return false;
    };
    let Ok(profile) = serde_json::from_str::<DisplayProfile>(&content) else {
        return false;
    };
    display::profile_covers_displays(&profile, display_ids)
}