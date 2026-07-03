use windows::Win32::Devices::Display::{
    DISPLAYCONFIG_MODE_INFO, DISPLAYCONFIG_PATH_INFO,
};

use super::capture::{get_target_name_for_path, query_raw_config};
use super::types::PathLabel;

pub fn remap_profile(
    labels: &[PathLabel],
    paths: &mut [DISPLAYCONFIG_PATH_INFO],
    modes: &mut [DISPLAYCONFIG_MODE_INFO],
) -> Result<(), String> {
    let (current_paths, current_modes) = query_raw_config()?;

    for (idx, saved_path) in paths.iter_mut().enumerate() {
        let label = labels.get(idx);
        let target_key = label
            .map(|l| l.target_device_name.to_lowercase())
            .unwrap_or_default();

        let matching = if !target_key.is_empty() {
            current_paths.iter().find(|current| {
                get_target_name_for_path(current)
                    .map(|n| n.to_lowercase() == target_key)
                    .unwrap_or(false)
            })
        } else {
            None
        };

        if let Some(current) = matching {
            saved_path.sourceInfo.adapterId = current.sourceInfo.adapterId;
            saved_path.targetInfo.adapterId = current.targetInfo.adapterId;
            saved_path.sourceInfo.id = current.sourceInfo.id;
            saved_path.targetInfo.id = current.targetInfo.id;
        }
    }

    for saved_mode in modes.iter_mut() {
        if let Some(current_mode) = current_modes.iter().find(|m| m.id == saved_mode.id) {
            saved_mode.adapterId = current_mode.adapterId;
        }
    }

    Ok(())
}