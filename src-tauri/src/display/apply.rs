use windows::Win32::Devices::Display::{
    SetDisplayConfig, DISPLAYCONFIG_MODE_INFO, DISPLAYCONFIG_PATH_INFO, SDC_ALLOW_CHANGES,
    SDC_APPLY, SDC_USE_SUPPLIED_DISPLAY_CONFIG,
};

use super::remap::remap_profile;
use super::types::{decode_structs, DisplayProfile};

/// DISPLAYCONFIG_PATH_ACTIVE
const PATH_ACTIVE: u32 = 0x00000001;

pub fn validate_profile_safe(profile: &DisplayProfile, group_display_ids: &[String]) -> Result<(), String> {
    validate_at_least_one_active(profile)?;

    if group_display_ids.is_empty() {
        return Ok(());
    }

    if !profile_activates_any_assigned(profile, group_display_ids) {
        return Err(
            "This layout does not activate any of the displays assigned to this group.".into(),
        );
    }

    Ok(())
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
            .map(|label| group_display_ids.iter().any(|id| id == &label.gdi_device_name))
            .unwrap_or(false)
    })
}

pub fn apply_profile(profile: &DisplayProfile) -> Result<(), String> {
    let mut paths: Vec<DISPLAYCONFIG_PATH_INFO> = decode_structs(&profile.paths_b64)?;
    let mut modes: Vec<DISPLAYCONFIG_MODE_INFO> = decode_structs(&profile.modes_b64)?;

    remap_profile(&profile.path_labels, &mut paths, &mut modes)?;

    let flags = SDC_APPLY | SDC_USE_SUPPLIED_DISPLAY_CONFIG | SDC_ALLOW_CHANGES;
    let result = unsafe { SetDisplayConfig(Some(&paths), Some(&modes), flags) };

    if result != 0 {
        return Err(format!("SetDisplayConfig failed with code {result}"));
    }

    Ok(())
}