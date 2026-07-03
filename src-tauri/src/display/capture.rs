use std::ffi::OsString;
use std::os::windows::ffi::OsStringExt;

use windows::Win32::Devices::Display::{
    DisplayConfigGetDeviceInfo, GetDisplayConfigBufferSizes, QueryDisplayConfig,
    DISPLAYCONFIG_DEVICE_INFO_GET_SOURCE_NAME, DISPLAYCONFIG_DEVICE_INFO_GET_TARGET_NAME,
    DISPLAYCONFIG_DEVICE_INFO_GET_TARGET_PREFERRED_MODE, DISPLAYCONFIG_DEVICE_INFO_HEADER,
    DISPLAYCONFIG_MODE_INFO, DISPLAYCONFIG_PATH_INFO, DISPLAYCONFIG_SOURCE_DEVICE_NAME,
    DISPLAYCONFIG_TARGET_DEVICE_NAME, DISPLAYCONFIG_TARGET_PREFERRED_MODE, QDC_ALL_PATHS,
    QDC_VIRTUAL_MODE_AWARE, QUERY_DISPLAY_CONFIG_FLAGS,
};
use windows::Win32::Foundation::LUID;

const QUERY_FLAGS: QUERY_DISPLAY_CONFIG_FLAGS =
    QUERY_DISPLAY_CONFIG_FLAGS(QDC_ALL_PATHS.0 | QDC_VIRTUAL_MODE_AWARE.0);
use windows::Win32::Foundation::WIN32_ERROR;

use super::types::{DisplayProfile, PathLabel, encode_structs};

pub fn capture_current_profile() -> Result<DisplayProfile, String> {
    let (paths, modes) = query_raw_config()?;
    let path_labels = paths
        .iter()
        .map(|p| PathLabel {
            gdi_device_name: get_source_name(p).unwrap_or_default(),
            target_device_name: get_target_name(p).unwrap_or_default(),
        })
        .collect();

    Ok(DisplayProfile {
        version: 1,
        paths_b64: encode_structs(&paths),
        modes_b64: encode_structs(&modes),
        path_labels,
    })
}

pub fn query_raw_config() -> Result<(Vec<DISPLAYCONFIG_PATH_INFO>, Vec<DISPLAYCONFIG_MODE_INFO>), String> {
    let mut path_count: u32 = 0;
    let mut mode_count: u32 = 0;

    let err = unsafe {
        GetDisplayConfigBufferSizes(QUERY_FLAGS, &mut path_count, &mut mode_count)
    };
    if err != WIN32_ERROR(0) {
        return Err(format!("GetDisplayConfigBufferSizes failed: {err:?}"));
    }

    let mut paths = vec![DISPLAYCONFIG_PATH_INFO::default(); path_count as usize];
    let mut modes = vec![DISPLAYCONFIG_MODE_INFO::default(); mode_count as usize];

    let err = unsafe {
        QueryDisplayConfig(
            QUERY_FLAGS,
            &mut path_count,
            paths.as_mut_ptr(),
            &mut mode_count,
            modes.as_mut_ptr(),
            None,
        )
    };
    if err != WIN32_ERROR(0) {
        return Err(format!("QueryDisplayConfig failed: {err:?}"));
    }

    paths.truncate(path_count as usize);
    modes.truncate(mode_count as usize);
    Ok((paths, modes))
}

/// Queries a target's native mode; works for connected displays even while
/// they are disabled in Windows.
pub fn get_target_preferred_mode(
    adapter_id: LUID,
    target_id: u32,
) -> Option<DISPLAYCONFIG_TARGET_PREFERRED_MODE> {
    let mut preferred = DISPLAYCONFIG_TARGET_PREFERRED_MODE::default();
    preferred.header = DISPLAYCONFIG_DEVICE_INFO_HEADER {
        r#type: DISPLAYCONFIG_DEVICE_INFO_GET_TARGET_PREFERRED_MODE,
        size: std::mem::size_of::<DISPLAYCONFIG_TARGET_PREFERRED_MODE>() as u32,
        adapterId: adapter_id,
        id: target_id,
    };
    let result = unsafe { DisplayConfigGetDeviceInfo(&mut preferred.header) };
    (result == 0).then_some(preferred)
}

pub fn get_source_name_for_path(path: &DISPLAYCONFIG_PATH_INFO) -> Option<String> {
    get_source_name(path)
}

pub fn get_target_name_for_path(path: &DISPLAYCONFIG_PATH_INFO) -> Option<String> {
    get_target_name(path)
}

fn get_source_name(path: &DISPLAYCONFIG_PATH_INFO) -> Option<String> {
    let mut source = DISPLAYCONFIG_SOURCE_DEVICE_NAME::default();
    source.header = DISPLAYCONFIG_DEVICE_INFO_HEADER {
        r#type: DISPLAYCONFIG_DEVICE_INFO_GET_SOURCE_NAME,
        size: std::mem::size_of::<DISPLAYCONFIG_SOURCE_DEVICE_NAME>() as u32,
        adapterId: path.sourceInfo.adapterId,
        id: path.sourceInfo.id,
    };
    let result = unsafe { DisplayConfigGetDeviceInfo(&mut source.header) };
    if result != 0 {
        return None;
    }
    Some(wide_to_string(&source.viewGdiDeviceName))
}

fn get_target_name(path: &DISPLAYCONFIG_PATH_INFO) -> Option<String> {
    let mut target = DISPLAYCONFIG_TARGET_DEVICE_NAME::default();
    target.header = DISPLAYCONFIG_DEVICE_INFO_HEADER {
        r#type: DISPLAYCONFIG_DEVICE_INFO_GET_TARGET_NAME,
        size: std::mem::size_of::<DISPLAYCONFIG_TARGET_DEVICE_NAME>() as u32,
        adapterId: path.targetInfo.adapterId,
        id: path.targetInfo.id,
    };
    let result = unsafe { DisplayConfigGetDeviceInfo(&mut target.header) };
    if result != 0 {
        return None;
    }
    Some(wide_to_string(&target.monitorFriendlyDeviceName))
}

fn wide_to_string(wide: &[u16]) -> String {
    let len = wide.iter().position(|&c| c == 0).unwrap_or(wide.len());
    OsString::from_wide(&wide[..len])
        .to_string_lossy()
        .into_owned()
}