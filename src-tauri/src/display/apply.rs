use windows::Win32::Devices::Display::{
    SetDisplayConfig, DISPLAYCONFIG_MODE_INFO, DISPLAYCONFIG_MODE_INFO_TYPE_SOURCE,
    DISPLAYCONFIG_MODE_INFO_TYPE_TARGET, DISPLAYCONFIG_PATH_INFO, DISPLAYCONFIG_PIXELFORMAT_32BPP,
    DISPLAYCONFIG_SOURCE_MODE, SDC_ALLOW_CHANGES, SDC_APPLY, SDC_USE_SUPPLIED_DISPLAY_CONFIG,
    SDC_VALIDATE, SDC_VIRTUAL_MODE_AWARE,
};
use windows::Win32::Foundation::POINTL;

use super::capture::get_target_preferred_mode;
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
        "None of this group's displays are available. \
         Make sure {} are connected, then click Save layout again.",
        assigned_labels.join(", ")
    ))
}

/// Turns on assigned displays that are connected but disabled in Windows so a
/// group can be saved without activating its displays first. SetDisplayConfig
/// refuses supplied paths without modes, so each activated path gets modes
/// synthesized from the display's preferred (native) mode, placed to the right
/// of the sources that are already on.
pub fn activate_assigned_displays(
    profile: &mut DisplayProfile,
    group_display_ids: &[String],
) -> Result<(), String> {
    if group_display_ids.is_empty() {
        return Ok(());
    }

    let mut paths: Vec<DISPLAYCONFIG_PATH_INFO> = decode_structs(&profile.paths_b64)?;
    let mut modes: Vec<DISPLAYCONFIG_MODE_INFO> = decode_structs(&profile.modes_b64)?;
    let assigned_labels = resolve_assigned_labels(group_display_ids);

    let mut used_sources: Vec<((i32, u32), u32)> = Vec::new();
    let mut used_targets: Vec<((i32, u32), u32)> = Vec::new();
    for path in paths.iter().filter(|p| p.flags & PATH_ACTIVE != 0) {
        used_sources.push(source_key(path));
        used_targets.push(target_key(path));
    }

    let mut changed = false;

    for (display_id, display_name) in group_display_ids.iter().zip(&assigned_labels) {
        let already_active = paths.iter().enumerate().any(|(idx, path)| {
            path.flags & PATH_ACTIVE != 0
                && profile
                    .path_labels
                    .get(idx)
                    .map_or(false, |label| candidate_score(label, display_id, display_name) > 0)
        });
        if already_active {
            continue;
        }

        let Some(idx) = best_inactive_candidate(
            &paths,
            &profile.path_labels,
            display_id,
            display_name,
            &used_sources,
            &used_targets,
        ) else {
            continue;
        };

        activate_path_with_preferred_mode(&mut paths, idx, &mut modes).map_err(|e| {
            format!("Could not turn on {display_name}: {e}")
        })?;
        used_sources.push(source_key(&paths[idx]));
        used_targets.push(target_key(&paths[idx]));
        changed = true;
    }

    if changed {
        profile.paths_b64 = encode_structs(&paths);
        profile.modes_b64 = encode_structs(&modes);
    }
    Ok(())
}

fn activate_path_with_preferred_mode(
    paths: &mut [DISPLAYCONFIG_PATH_INFO],
    idx: usize,
    modes: &mut Vec<DISPLAYCONFIG_MODE_INFO>,
) -> Result<(), String> {
    let path = paths[idx];
    let preferred = get_target_preferred_mode(path.targetInfo.adapterId, path.targetInfo.id)
        .ok_or_else(|| "the display did not report a preferred mode".to_string())?;

    let next_x = active_sources_right_edge(paths, modes);

    let target_mode_idx = modes.len() as u32;
    let mut target_mode = DISPLAYCONFIG_MODE_INFO::default();
    target_mode.infoType = DISPLAYCONFIG_MODE_INFO_TYPE_TARGET;
    target_mode.id = path.targetInfo.id;
    target_mode.adapterId = path.targetInfo.adapterId;
    target_mode.Anonymous.targetMode = preferred.targetMode;
    modes.push(target_mode);

    let source_mode_idx = modes.len() as u32;
    let mut source_mode = DISPLAYCONFIG_MODE_INFO::default();
    source_mode.infoType = DISPLAYCONFIG_MODE_INFO_TYPE_SOURCE;
    source_mode.id = path.sourceInfo.id;
    source_mode.adapterId = path.sourceInfo.adapterId;
    source_mode.Anonymous.sourceMode = DISPLAYCONFIG_SOURCE_MODE {
        width: preferred.width,
        height: preferred.height,
        pixelFormat: DISPLAYCONFIG_PIXELFORMAT_32BPP,
        position: POINTL { x: next_x, y: 0 },
    };
    modes.push(source_mode);

    // Virtual-mode-aware index layout: source mode index in the high word over
    // an invalid clone group id; target mode index in the high word over an
    // invalid desktop image index.
    let path = &mut paths[idx];
    path.flags |= PATH_ACTIVE;
    path.sourceInfo.Anonymous.modeInfoIdx = (source_mode_idx << 16) | 0xffff;
    path.targetInfo.Anonymous.modeInfoIdx = (target_mode_idx << 16) | 0xffff;
    path.targetInfo.refreshRate = preferred.targetMode.targetVideoSignalInfo.vSyncFreq;
    path.targetInfo.scanLineOrdering = preferred.targetMode.targetVideoSignalInfo.scanLineOrdering;
    Ok(())
}

fn active_sources_right_edge(
    paths: &[DISPLAYCONFIG_PATH_INFO],
    modes: &[DISPLAYCONFIG_MODE_INFO],
) -> i32 {
    paths
        .iter()
        .filter(|path| path.flags & PATH_ACTIVE != 0)
        .filter_map(source_mode_index)
        .filter_map(|idx| {
            let mode = modes.get(idx)?;
            if mode.infoType != DISPLAYCONFIG_MODE_INFO_TYPE_SOURCE {
                return None;
            }
            let source = unsafe { mode.Anonymous.sourceMode };
            Some(source.position.x + source.width as i32)
        })
        .max()
        .unwrap_or(0)
}

fn source_key(path: &DISPLAYCONFIG_PATH_INFO) -> ((i32, u32), u32) {
    let luid = path.sourceInfo.adapterId;
    ((luid.HighPart, luid.LowPart), path.sourceInfo.id)
}

fn target_key(path: &DISPLAYCONFIG_PATH_INFO) -> ((i32, u32), u32) {
    let luid = path.targetInfo.adapterId;
    ((luid.HighPart, luid.LowPart), path.targetInfo.id)
}

/// A monitor-name match outranks a GDI source-name match: in QDC_ALL_PATHS
/// every source pairs with every target, so a GDI name alone can point at the
/// wrong monitor.
fn candidate_score(label: &PathLabel, display_id: &str, display_name: &str) -> u32 {
    let gdi = u32::from(label.gdi_device_name == display_id);
    let name = u32::from(names_match(&label.target_device_name, display_name));
    name * 2 + gdi
}

fn best_inactive_candidate(
    paths: &[DISPLAYCONFIG_PATH_INFO],
    labels: &[PathLabel],
    display_id: &str,
    display_name: &str,
    used_sources: &[((i32, u32), u32)],
    used_targets: &[((i32, u32), u32)],
) -> Option<usize> {
    let mut best: Option<(u32, usize)> = None;

    for (idx, path) in paths.iter().enumerate() {
        if path.flags & PATH_ACTIVE != 0 || path.targetInfo.targetAvailable.0 == 0 {
            continue;
        }
        if used_sources.contains(&source_key(path)) || used_targets.contains(&target_key(path)) {
            continue;
        }
        let Some(label) = labels.get(idx) else {
            continue;
        };
        let score = candidate_score(label, display_id, display_name);
        if score > 0 && best.map_or(true, |(best_score, _)| score > best_score) {
            best = Some((score, idx));
        }
    }

    best.map(|(_, idx)| idx)
}

/// True when this profile turns on every assigned display.
pub fn profile_covers_displays(profile: &DisplayProfile, group_display_ids: &[String]) -> bool {
    if group_display_ids.is_empty() {
        return false;
    }
    let Ok(paths) = decode_structs::<DISPLAYCONFIG_PATH_INFO>(&profile.paths_b64) else {
        return false;
    };
    let assigned_labels = resolve_assigned_labels(group_display_ids);
    group_display_ids
        .iter()
        .zip(&assigned_labels)
        .all(|(id, name)| {
            paths.iter().enumerate().any(|(idx, path)| {
                path.flags & PATH_ACTIVE != 0
                    && profile
                        .path_labels
                        .get(idx)
                        .map_or(false, |label| candidate_score(label, id, name) > 0)
            })
        })
}

/// True when every assigned display is currently turned on in Windows.
pub fn displays_all_active(group_display_ids: &[String]) -> bool {
    if group_display_ids.is_empty() {
        return false;
    }
    let displays = list_displays().unwrap_or_default();
    group_display_ids.iter().all(|id| {
        displays
            .iter()
            .any(|display| &display.id == id && display.is_active)
    })
}

/// Asks Windows to validate the layout without applying it, so a bad save
/// fails at save time instead of on the first activation.
pub fn validate_profile_with_windows(profile: &DisplayProfile) -> Result<(), String> {
    let (paths, modes) = prepare_for_apply(profile)?;
    let flags = SDC_VALIDATE
        | SDC_USE_SUPPLIED_DISPLAY_CONFIG
        | SDC_ALLOW_CHANGES
        | SDC_VIRTUAL_MODE_AWARE;
    let result = unsafe { SetDisplayConfig(Some(&paths), Some(&modes), flags) };
    if result != 0 {
        return Err(format!(
            "Windows rejected this layout (code {result}). \
             Turn the group's displays on, arrange them in Windows Display Settings, and save again."
        ));
    }
    Ok(())
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

    /// Recreates the original error-87 shape — an active desktop with no
    /// source at (0,0) — by shifting the sanitized TV profile off origin, then
    /// checks that re-origining restores a primary and Windows accepts it.
    #[test]
    fn sanitized_tv_profile_validates_after_reorigin() {
        let base = std::env::var("APPDATA").expect("APPDATA");
        let path = format!("{base}\\display-switcher\\profiles\\cd1a3f53-7d16-430f-9d8c-17ef5c959647.json");
        let content = fs::read_to_string(&path).expect("read profile");
        let mut profile: DisplayProfile = serde_json::from_str(&content).expect("parse profile");
        sanitize_profile_for_group(&mut profile, &["\\\\.\\DISPLAY1".to_string()]).expect("sanitize");

        let mut paths: Vec<DISPLAYCONFIG_PATH_INFO> =
            decode_structs(&profile.paths_b64).expect("decode paths");
        let mut modes: Vec<DISPLAYCONFIG_MODE_INFO> =
            decode_structs(&profile.modes_b64).expect("decode modes");
        remap_profile(&mut paths, &mut modes).expect("remap");

        for mode in modes.iter_mut() {
            if mode.infoType == DISPLAYCONFIG_MODE_INFO_TYPE_SOURCE {
                let pos = unsafe { &mut mode.Anonymous.sourceMode.position };
                pos.x -= 3840;
                pos.y += 500;
            }
        }

        reorigin_active_sources(&paths, &mut modes);

        let origin_restored = paths
            .iter()
            .filter(|path| path.flags & PATH_ACTIVE != 0)
            .filter_map(source_mode_index)
            .any(|idx| {
                let pos = unsafe { modes[idx].Anonymous.sourceMode.position };
                pos.x == 0 && pos.y == 0
            });
        assert!(origin_restored, "expected an active source back at (0,0)");

        assert_eq!(validate_config(&paths, &modes), 0, "expected re-origined profile to pass");
    }

    /// Simulates saving the TV group from the live Windows state, whatever it
    /// currently is: sanitize deactivates non-group paths, activation turns the
    /// TV back on (mode-less if it is currently disabled), and Windows must
    /// accept the result.
    #[test]
    fn save_pipeline_handles_inactive_group_displays() {
        use super::super::capture::capture_current_profile;

        let tv_display_ids = vec!["\\\\.\\DISPLAY1".to_string()];
        let mut profile = capture_current_profile().expect("capture");
        sanitize_profile_for_group(&mut profile, &tv_display_ids).expect("sanitize");
        activate_assigned_displays(&mut profile, &tv_display_ids).expect("activate");

        let paths: Vec<DISPLAYCONFIG_PATH_INFO> =
            decode_structs(&profile.paths_b64).expect("decode paths");
        let active_targets: Vec<&str> = paths
            .iter()
            .enumerate()
            .filter(|(_, path)| path.flags & PATH_ACTIVE != 0)
            .filter_map(|(idx, _)| profile.path_labels.get(idx))
            .map(|label| label.target_device_name.as_str())
            .collect();
        assert!(
            active_targets.iter().any(|name| name.contains("SAMSUNG")),
            "expected the TV to be activated, got {active_targets:?}"
        );

        validate_profile_with_windows(&profile).expect("windows validate");
    }

    #[test]
    fn profile_coverage_matches_active_paths() {
        let base = std::env::var("APPDATA").expect("APPDATA");
        let path = format!("{base}\\display-switcher\\profiles\\3ef0f17c-7a54-4f28-905b-8651d1414e20.json");
        let content = fs::read_to_string(&path).expect("read profile");
        let profile: DisplayProfile = serde_json::from_str(&content).expect("parse profile");

        let desktop_ids = vec!["\\\\.\\DISPLAY2".to_string(), "\\\\.\\DISPLAY3".to_string()];
        assert!(profile_covers_displays(&profile, &desktop_ids));

        let with_tv = vec![
            "\\\\.\\DISPLAY1".to_string(),
            "\\\\.\\DISPLAY2".to_string(),
            "\\\\.\\DISPLAY3".to_string(),
        ];
        assert!(
            !profile_covers_displays(&profile, &with_tv),
            "desktop profile should not cover the inactive TV"
        );
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