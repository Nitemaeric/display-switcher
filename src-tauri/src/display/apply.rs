use windows::Win32::Devices::Display::{
    SetDisplayConfig, DISPLAYCONFIG_MODE_INFO, DISPLAYCONFIG_MODE_INFO_TYPE_SOURCE,
    DISPLAYCONFIG_PATH_INFO, SDC_ALLOW_CHANGES, SDC_APPLY, SDC_USE_SUPPLIED_DISPLAY_CONFIG,
    SDC_VIRTUAL_MODE_AWARE,
};

use super::enumerate::list_displays;
use super::remap::remap_profile;
use super::types::{decode_structs, encode_structs, DisplayProfile, PathLabel};

/// DISPLAYCONFIG_PATH_ACTIVE
const PATH_ACTIVE: u32 = 0x00000001;

pub fn validate_profile_safe(profile: &DisplayProfile, group_display_ids: &[String]) -> Result<(), String> {
    validate_at_least_one_active(profile)?;

    if group_display_ids.is_empty() {
        return Ok(());
    }

    let assigned_labels = resolve_assigned_labels(group_display_ids);

    if profile_activates_any_assigned(profile, group_display_ids, &assigned_labels) {
        return Ok(());
    }

    Err(format!(
        "None of this group's displays are turned on in the current Windows layout. \
         In Windows Display Settings, enable {}, arrange them, then click Save layout again.",
        assigned_labels.join(", ")
    ))
}

fn resolve_assigned_labels(group_display_ids: &[String]) -> Vec<String> {
    let displays = list_displays().unwrap_or_default();
    group_display_ids
        .iter()
        .map(|id| {
            displays
                .iter()
                .find(|display| &display.id == id)
                .map(|display| display.name.clone())
                .unwrap_or_else(|| id.clone())
        })
        .collect()
}

fn validate_at_least_one_active(profile: &DisplayProfile) -> Result<(), String> {
    let paths: Vec<DISPLAYCONFIG_PATH_INFO> = decode_structs(&profile.paths_b64)?;
    let any_active = paths.iter().any(|path| path.flags & PATH_ACTIVE != 0);
    if !any_active {
        return Err(
            "This layout would disable all displays. Save a layout that keeps at least one monitor on."
                .into(),
        );
    }
    Ok(())
}

fn profile_activates_any_assigned(
    profile: &DisplayProfile,
    group_display_ids: &[String],
    assigned_labels: &[String],
) -> bool {
    let paths: Vec<DISPLAYCONFIG_PATH_INFO> =
        decode_structs(&profile.paths_b64).unwrap_or_default();

    paths.iter().enumerate().any(|(idx, path)| {
        if path.flags & PATH_ACTIVE == 0 {
            return false;
        }
        profile
            .path_labels
            .get(idx)
            .map(|label| label_matches_assigned(label, group_display_ids, assigned_labels))
            .unwrap_or(false)
    })
}

fn label_matches_assigned(
    label: &PathLabel,
    group_display_ids: &[String],
    assigned_labels: &[String],
) -> bool {
    if group_display_ids
        .iter()
        .any(|id| id == &label.gdi_device_name)
    {
        return true;
    }

    assigned_labels
        .iter()
        .any(|name| names_match(&label.target_device_name, name))
}

fn names_match(target: &str, assigned: &str) -> bool {
    let target = normalize_device_name(target);
    let assigned = normalize_device_name(assigned);
    if target.is_empty() || assigned.is_empty() {
        return false;
    }
    target == assigned || target.contains(&assigned) || assigned.contains(&target)
}

fn normalize_device_name(name: &str) -> String {
    name.split('(')
        .next()
        .unwrap_or(name)
        .trim()
        .to_lowercase()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    const PATH_ACTIVE: u32 = 0x00000001;

    fn dump_active_paths(path: &str) {
        let content = fs::read_to_string(path).expect("read profile");
        let profile: DisplayProfile = serde_json::from_str(&content).expect("parse profile");
        let paths: Vec<DISPLAYCONFIG_PATH_INFO> =
            decode_structs(&profile.paths_b64).expect("decode paths");

        println!("=== {path} ===");
        for (idx, path_info) in paths.iter().enumerate() {
            if path_info.flags & PATH_ACTIVE == 0 {
                continue;
            }
            let label = profile.path_labels.get(idx);
            let gdi = label.map(|l| l.gdi_device_name.as_str()).unwrap_or("?");
            let target = label.map(|l| l.target_device_name.as_str()).unwrap_or("?");
            println!("  ACTIVE: {gdi} -> {target}");
        }
    }

    #[test]
    fn dump_saved_profiles() {
        let base = std::env::var("APPDATA").expect("APPDATA");
        let dir = format!("{base}\\display-switcher\\profiles");
        dump_active_paths(&format!("{dir}\\3ef0f17c-7a54-4f28-905b-8651d1414e20.json"));
        dump_active_paths(&format!("{dir}\\cd1a3f53-7d16-430f-9d8c-17ef5c959647.json"));
    }

    #[test]
    fn desktop_profile_targets_two_monitors() {
        let base = std::env::var("APPDATA").expect("APPDATA");
        let path = format!("{base}\\display-switcher\\profiles\\3ef0f17c-7a54-4f28-905b-8651d1414e20.json");
        let content = fs::read_to_string(&path).expect("read profile");
        let profile: DisplayProfile = serde_json::from_str(&content).expect("parse profile");
        let selections = active_path_selections(&profile).expect("selections");
        let gdis: Vec<_> = selections
            .iter()
            .map(|label| label.gdi_device_name.as_str())
            .collect();
        assert!(gdis.contains(&"\\\\.\\DISPLAY2"));
        assert!(gdis.contains(&"\\\\.\\DISPLAY3"));
    }

    #[test]
    fn sanitize_tv_profile_disables_desktop_paths() {
        let base = std::env::var("APPDATA").expect("APPDATA");
        let path = format!("{base}\\display-switcher\\profiles\\cd1a3f53-7d16-430f-9d8c-17ef5c959647.json");
        let content = fs::read_to_string(&path).expect("read profile");
        let mut profile: DisplayProfile = serde_json::from_str(&content).expect("parse profile");
        let tv_display_ids = vec!["\\\\.\\DISPLAY1".to_string()];

        sanitize_profile_for_group(&mut profile, &tv_display_ids).expect("sanitize");

        let paths: Vec<DISPLAYCONFIG_PATH_INFO> =
            decode_structs(&profile.paths_b64).expect("decode paths");
        let mut active: Vec<(String, String)> = Vec::new();
        for (idx, path_info) in paths.iter().enumerate() {
            if path_info.flags & PATH_ACTIVE == 0 {
                continue;
            }
            let label = profile.path_labels.get(idx).expect("label");
            active.push((label.gdi_device_name.clone(), label.target_device_name.clone()));
        }

        assert!(
            active.iter().all(|(gdi, _)| gdi == "\\\\.\\DISPLAY1"),
            "expected only DISPLAY1 active, got {active:?}"
        );
    }

    fn validate_config(paths: &[DISPLAYCONFIG_PATH_INFO], modes: &[DISPLAYCONFIG_MODE_INFO]) -> i32 {
        use windows::Win32::Devices::Display::SDC_VALIDATE;
        let flags = SDC_VALIDATE
            | SDC_USE_SUPPLIED_DISPLAY_CONFIG
            | SDC_ALLOW_CHANGES
            | SDC_VIRTUAL_MODE_AWARE;
        unsafe { SetDisplayConfig(Some(paths), Some(modes), flags) }
    }

    #[test]
    fn sanitized_tv_profile_validates_after_reorigin() {
        let base = std::env::var("APPDATA").expect("APPDATA");
        let path = format!("{base}\\display-switcher\\profiles\\cd1a3f53-7d16-430f-9d8c-17ef5c959647.json");
        let content = fs::read_to_string(&path).expect("read profile");
        let mut profile: DisplayProfile = serde_json::from_str(&content).expect("parse profile");
        sanitize_profile_for_group(&mut profile, &["\\\\.\\DISPLAY1".to_string()]).expect("sanitize");

        let mut paths = decode_structs(&profile.paths_b64).expect("decode paths");
        let mut modes = decode_structs(&profile.modes_b64).expect("decode modes");
        remap_profile(&mut paths, &mut modes).expect("remap");
        assert_eq!(validate_config(&paths, &modes), 87, "expected raw profile to fail");

        let (paths, modes) = prepare_for_apply(&profile).expect("prepare");
        assert_eq!(validate_config(&paths, &modes), 0, "expected re-origined profile to pass");
    }

    #[test]
    fn desktop_profile_still_validates() {
        let base = std::env::var("APPDATA").expect("APPDATA");
        let path = format!("{base}\\display-switcher\\profiles\\3ef0f17c-7a54-4f28-905b-8651d1414e20.json");
        let content = fs::read_to_string(&path).expect("read profile");
        let profile: DisplayProfile = serde_json::from_str(&content).expect("parse profile");
        let (paths, modes) = prepare_for_apply(&profile).expect("prepare");
        assert_eq!(validate_config(&paths, &modes), 0);
    }
}

/// Deactivates display paths that are not assigned to this group so a saved
/// layout cannot accidentally keep extra monitors on (e.g. TV Mode with desktops).
pub fn sanitize_profile_for_group(
    profile: &mut DisplayProfile,
    group_display_ids: &[String],
) -> Result<(), String> {
    if group_display_ids.is_empty() {
        return Ok(());
    }

    let mut paths: Vec<DISPLAYCONFIG_PATH_INFO> = decode_structs(&profile.paths_b64)?;

    for (idx, path) in paths.iter_mut().enumerate() {
        let Some(label) = profile.path_labels.get(idx) else {
            continue;
        };
        if group_display_ids
            .iter()
            .any(|id| id == &label.gdi_device_name)
        {
            continue;
        }
        path.flags &= !PATH_ACTIVE;
    }

    profile.paths_b64 = encode_structs(&paths);
    Ok(())
}

fn labels_match_for_apply(saved: &PathLabel, current: &PathLabel) -> bool {
    if saved.gdi_device_name.to_lowercase() != current.gdi_device_name.to_lowercase() {
        return false;
    }
    if saved.target_device_name.is_empty() || current.target_device_name.is_empty() {
        return true;
    }
    names_match(&saved.target_device_name, &current.target_device_name)
}

fn active_path_selections(profile: &DisplayProfile) -> Result<Vec<PathLabel>, String> {
    let saved_paths: Vec<DISPLAYCONFIG_PATH_INFO> = decode_structs(&profile.paths_b64)?;
    let mut selections = Vec::new();

    for (idx, path) in saved_paths.iter().enumerate() {
        if path.flags & PATH_ACTIVE == 0 {
            continue;
        }
        let Some(label) = profile.path_labels.get(idx) else {
            continue;
        };
        if label.gdi_device_name.is_empty() {
            continue;
        }
        if selections.iter().any(|existing| labels_match_for_apply(existing, label)) {
            continue;
        }
        selections.push(label.clone());
    }

    Ok(selections)
}

const SOURCE_MODE_IDX_INVALID: u32 = 0xffff;

fn source_mode_index(path: &DISPLAYCONFIG_PATH_INFO) -> Option<usize> {
    let idx = unsafe { path.sourceInfo.Anonymous.modeInfoIdx } >> 16;
    (idx != SOURCE_MODE_IDX_INVALID).then_some(idx as usize)
}

fn reorigin_active_sources(
    paths: &[DISPLAYCONFIG_PATH_INFO],
    modes: &mut [DISPLAYCONFIG_MODE_INFO],
) {
    let mut source_idxs: Vec<usize> = paths
        .iter()
        .filter(|path| path.flags & PATH_ACTIVE != 0)
        .filter_map(source_mode_index)
        .collect();
    source_idxs.sort_unstable();
    source_idxs.dedup();

    let positions: Vec<(i32, i32)> = source_idxs
        .iter()
        .filter_map(|&idx| {
            let mode = modes.get(idx)?;
            if mode.infoType != DISPLAYCONFIG_MODE_INFO_TYPE_SOURCE {
                return None;
            }
            let pos = unsafe { mode.Anonymous.sourceMode.position };
            Some((pos.x, pos.y))
        })
        .collect();

    if positions.is_empty() || positions.iter().any(|&(x, y)| x == 0 && y == 0) {
        return;
    }

    let (dx, dy) = positions
        .iter()
        .copied()
        .min_by_key(|&(x, y)| (x as i64).abs() + (y as i64).abs())
        .unwrap();

    for &idx in &source_idxs {
        let mode = &mut modes[idx];
        if mode.infoType != DISPLAYCONFIG_MODE_INFO_TYPE_SOURCE {
            continue;
        }
        let pos = unsafe { &mut mode.Anonymous.sourceMode.position };
        pos.x -= dx;
        pos.y -= dy;
    }
}

fn prepare_for_apply(
    profile: &DisplayProfile,
) -> Result<(Vec<DISPLAYCONFIG_PATH_INFO>, Vec<DISPLAYCONFIG_MODE_INFO>), String> {
    let mut paths = decode_structs(&profile.paths_b64)?;
    let mut modes = decode_structs(&profile.modes_b64)?;
    remap_profile(&mut paths, &mut modes)?;
    reorigin_active_sources(&paths, &mut modes);
    Ok((paths, modes))
}

pub fn apply_profile(profile: &DisplayProfile) -> Result<(), String> {
    validate_at_least_one_active(profile)?;

    let (paths, modes) = prepare_for_apply(profile)?;

    let flags = SDC_APPLY
        | SDC_USE_SUPPLIED_DISPLAY_CONFIG
        | SDC_ALLOW_CHANGES
        | SDC_VIRTUAL_MODE_AWARE;
    let result = unsafe { SetDisplayConfig(Some(&paths), Some(&modes), flags) };

    if result != 0 {
        return Err(format!("SetDisplayConfig failed with code {result}"));
    }

    Ok(())
}