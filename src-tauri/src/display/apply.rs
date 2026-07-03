use windows::Win32::Devices::Display::{
    SetDisplayConfig, DISPLAYCONFIG_MODE_INFO, DISPLAYCONFIG_PATH_INFO, SDC_ALLOW_CHANGES,
    SDC_APPLY, SDC_USE_SUPPLIED_DISPLAY_CONFIG,
};

use super::remap::remap_profile;
use super::types::{decode_structs, DisplayProfile};

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