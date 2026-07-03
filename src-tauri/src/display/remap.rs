use std::collections::HashMap;

use windows::Win32::Devices::Display::{
    DISPLAYCONFIG_MODE_INFO, DISPLAYCONFIG_OUTPUT_TECHNOLOGY_DISPLAYPORT_EMBEDDED,
    DISPLAYCONFIG_OUTPUT_TECHNOLOGY_DISPLAYPORT_EXTERNAL, DISPLAYCONFIG_OUTPUT_TECHNOLOGY_DVI,
    DISPLAYCONFIG_OUTPUT_TECHNOLOGY_HDMI, DISPLAYCONFIG_OUTPUT_TECHNOLOGY_INTERNAL,
    DISPLAYCONFIG_PATH_INFO, DISPLAYCONFIG_VIDEO_OUTPUT_TECHNOLOGY,
};
use windows::Win32::Foundation::LUID;

use super::capture::{get_target_name_for_path, query_raw_config};

/// Remap adapter LUIDs in a saved profile to match the current topology, mirroring
/// DisplayConfig's `UpdateAdapterIds` behavior before applying an imported layout.
pub fn remap_profile(
    paths: &mut [DISPLAYCONFIG_PATH_INFO],
    modes: &mut [DISPLAYCONFIG_MODE_INFO],
) -> Result<(), String> {
    let (current_paths, _) = query_raw_config()?;
    update_adapter_ids(paths, modes, &current_paths)
}

fn luid_key(luid: LUID) -> (i32, u32) {
    (luid.HighPart, luid.LowPart)
}

fn target_available(path: &DISPLAYCONFIG_PATH_INFO) -> bool {
    path.targetInfo.targetAvailable.0 != 0
}

fn available_path_indexes(paths: &[DISPLAYCONFIG_PATH_INFO]) -> Vec<usize> {
    let mut indexes = Vec::new();
    let mut seen = HashMap::new();

    for (idx, path) in paths.iter().enumerate() {
        if !target_available(path) {
            continue;
        }
        let key = (luid_key(path.targetInfo.adapterId), path.targetInfo.id);
        if seen.insert(key, ()).is_none() {
            indexes.push(idx);
        }
    }

    indexes
}

fn output_technology_priority(tech: DISPLAYCONFIG_VIDEO_OUTPUT_TECHNOLOGY) -> u32 {
    if tech == DISPLAYCONFIG_OUTPUT_TECHNOLOGY_INTERNAL {
        50
    } else if tech == DISPLAYCONFIG_OUTPUT_TECHNOLOGY_DISPLAYPORT_EMBEDDED {
        100
    } else if tech == DISPLAYCONFIG_OUTPUT_TECHNOLOGY_DVI {
        150
    } else if tech == DISPLAYCONFIG_OUTPUT_TECHNOLOGY_DISPLAYPORT_EXTERNAL {
        200
    } else if tech == DISPLAYCONFIG_OUTPUT_TECHNOLOGY_HDMI {
        250
    } else {
        300
    }
}

fn normalize_monitor_name(name: &str) -> String {
    name.split('(')
        .next()
        .unwrap_or(name)
        .trim()
        .to_lowercase()
}

fn monitor_names_match(left: &str, right: &str) -> bool {
    let left = normalize_monitor_name(left);
    let right = normalize_monitor_name(right);
    if left.is_empty() || right.is_empty() {
        return false;
    }
    left == right || left.contains(&right) || right.contains(&left)
}

fn sort_available_indexes(paths: &[DISPLAYCONFIG_PATH_INFO], indexes: &[usize]) -> Vec<usize> {
    let mut sorted = indexes.to_vec();
    sorted.sort_by_key(|&idx| {
        let path = &paths[idx];
        let name = get_target_name_for_path(path).unwrap_or_default();
        (
            normalize_monitor_name(&name),
            output_technology_priority(path.targetInfo.outputTechnology),
            path.targetInfo.id,
        )
    });
    sorted
}

fn build_adapter_luid_map(
    saved_paths: &[DISPLAYCONFIG_PATH_INFO],
    current_paths: &[DISPLAYCONFIG_PATH_INFO],
) -> Result<HashMap<(i32, u32), LUID>, String> {
    let saved_indexes = sort_available_indexes(saved_paths, &available_path_indexes(saved_paths));
    let current_indexes =
        sort_available_indexes(current_paths, &available_path_indexes(current_paths));

    if saved_indexes.len() != current_indexes.len() {
        return Err(format!(
            "Cannot apply layout: connected display count changed (saved {}, current {}).",
            saved_indexes.len(),
            current_indexes.len()
        ));
    }

    let mut map = HashMap::new();

    for saved_idx in saved_indexes {
        let saved_path = &saved_paths[saved_idx];
        let saved_name = get_target_name_for_path(saved_path).unwrap_or_default();
        let saved_adapter = saved_path.targetInfo.adapterId;

        let current_idx = current_indexes.iter().copied().find(|&idx| {
            let current_name = get_target_name_for_path(&current_paths[idx]).unwrap_or_default();
            monitor_names_match(&saved_name, &current_name)
        });

        let Some(current_idx) = current_idx else {
            return Err(format!(
                "Cannot apply layout: monitor \"{saved_name}\" is not connected."
            ));
        };

        let current_adapter = current_paths[current_idx].targetInfo.adapterId;
        let key = luid_key(saved_adapter);

        if let Some(existing) = map.get(&key) {
            if *existing != current_adapter {
                return Err("Cannot apply layout: adapter layout has changed.".into());
            }
        } else {
            map.insert(key, current_adapter);
        }
    }

    Ok(map)
}

fn apply_adapter_luid_map(
    paths: &mut [DISPLAYCONFIG_PATH_INFO],
    modes: &mut [DISPLAYCONFIG_MODE_INFO],
    map: &HashMap<(i32, u32), LUID>,
) {
    for path in paths.iter_mut() {
        if let Some(adapter) = map.get(&luid_key(path.sourceInfo.adapterId)) {
            path.sourceInfo.adapterId = *adapter;
        }
        if let Some(adapter) = map.get(&luid_key(path.targetInfo.adapterId)) {
            path.targetInfo.adapterId = *adapter;
        }
    }

    for mode in modes.iter_mut() {
        if let Some(adapter) = map.get(&luid_key(mode.adapterId)) {
            mode.adapterId = *adapter;
        }
    }
}

fn update_adapter_ids(
    paths: &mut [DISPLAYCONFIG_PATH_INFO],
    modes: &mut [DISPLAYCONFIG_MODE_INFO],
    current_paths: &[DISPLAYCONFIG_PATH_INFO],
) -> Result<(), String> {
    let map = build_adapter_luid_map(paths, current_paths)?;
    apply_adapter_luid_map(paths, modes, &map);
    Ok(())
}