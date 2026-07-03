use std::ffi::OsString;
use std::os::windows::ffi::OsStringExt;

use windows::core::BOOL;
use windows::Win32::Devices::Display::{
    DisplayConfigGetDeviceInfo, DISPLAYCONFIG_DEVICE_INFO_GET_TARGET_NAME,
    DISPLAYCONFIG_DEVICE_INFO_HEADER, DISPLAYCONFIG_TARGET_DEVICE_NAME,
};
use windows::Win32::Foundation::LPARAM;
use windows::Win32::Graphics::Gdi::{
    EnumDisplayMonitors, GetMonitorInfoW, HDC, HMONITOR, MONITORINFOEXW,
};
use windows::Win32::UI::WindowsAndMessaging::MONITORINFOF_PRIMARY;

use super::types::DisplayInfo;

struct EnumContext {
    displays: Vec<DisplayInfo>,
    error: Option<String>,
}

unsafe extern "system" fn monitor_enum_proc(
    h_monitor: HMONITOR,
    _hdc: HDC,
    _rect: *mut windows::Win32::Foundation::RECT,
    lparam: LPARAM,
) -> BOOL {
    let ctx = &mut *(lparam.0 as *mut EnumContext);
    match collect_monitor(h_monitor) {
        Ok(info) => ctx.displays.push(info),
        Err(e) => ctx.error = Some(e),
    }
    BOOL(1)
}

pub fn list_displays() -> Result<Vec<DisplayInfo>, String> {
    let mut ctx = EnumContext {
        displays: Vec::new(),
        error: None,
    };

    unsafe {
        let ok = EnumDisplayMonitors(
            None,
            None,
            Some(monitor_enum_proc),
            LPARAM(&mut ctx as *mut _ as isize),
        );
        if !ok.as_bool() {
            return Err("EnumDisplayMonitors failed".into());
        }
    }

    if let Some(err) = ctx.error {
        return Err(err);
    }

    ctx.displays.sort_by(|a, b| a.id.cmp(&b.id));
    Ok(ctx.displays)
}

fn collect_monitor(h_monitor: HMONITOR) -> Result<DisplayInfo, String> {
    let mut info = MONITORINFOEXW::default();
    info.monitorInfo.cbSize = std::mem::size_of::<MONITORINFOEXW>() as u32;

    unsafe {
        let ok = GetMonitorInfoW(h_monitor, &mut info.monitorInfo as *mut _ as *mut _);
        if !ok.as_bool() {
            return Err("GetMonitorInfoW failed".into());
        }
    }

    let gdi_name = wide_to_string(&info.szDevice);
    let rect = info.monitorInfo.rcMonitor;
    let width = (rect.right - rect.left) as u32;
    let height = (rect.bottom - rect.top) as u32;
    let is_primary = (info.monitorInfo.dwFlags & MONITORINFOF_PRIMARY) != 0;
    let friendly_name = query_target_name_for_gdi(&gdi_name).unwrap_or_else(|| gdi_name.clone());

    Ok(DisplayInfo {
        id: gdi_name,
        name: friendly_name,
        is_active: true,
        is_primary,
        width,
        height,
        x: rect.left,
        y: rect.top,
    })
}

fn query_target_name_for_gdi(gdi_name: &str) -> Option<String> {
    let (paths, _) = super::capture::query_raw_config().ok()?;
    for path in paths {
        let source_name = get_source_name(&path)?;
        if source_name != gdi_name {
            continue;
        }
        return get_target_name(&path);
    }
    None
}

fn get_source_name(path: &windows::Win32::Devices::Display::DISPLAYCONFIG_PATH_INFO) -> Option<String> {
    use windows::Win32::Devices::Display::{
        DisplayConfigGetDeviceInfo, DISPLAYCONFIG_DEVICE_INFO_GET_SOURCE_NAME,
        DISPLAYCONFIG_DEVICE_INFO_HEADER, DISPLAYCONFIG_SOURCE_DEVICE_NAME,
    };

    let mut source = DISPLAYCONFIG_SOURCE_DEVICE_NAME::default();
    source.header = DISPLAYCONFIG_DEVICE_INFO_HEADER {
        r#type: DISPLAYCONFIG_DEVICE_INFO_GET_SOURCE_NAME,
        size: std::mem::size_of::<DISPLAYCONFIG_SOURCE_DEVICE_NAME>() as u32,
        adapterId: path.sourceInfo.adapterId,
        id: path.sourceInfo.id,
    };
    if unsafe { DisplayConfigGetDeviceInfo(&mut source.header) } != 0 {
        return None;
    }
    Some(wide_to_string(&source.viewGdiDeviceName))
}

fn get_target_name(path: &windows::Win32::Devices::Display::DISPLAYCONFIG_PATH_INFO) -> Option<String> {
    let mut target = DISPLAYCONFIG_TARGET_DEVICE_NAME::default();
    target.header = DISPLAYCONFIG_DEVICE_INFO_HEADER {
        r#type: DISPLAYCONFIG_DEVICE_INFO_GET_TARGET_NAME,
        size: std::mem::size_of::<DISPLAYCONFIG_TARGET_DEVICE_NAME>() as u32,
        adapterId: path.targetInfo.adapterId,
        id: path.targetInfo.id,
    };
    if unsafe { DisplayConfigGetDeviceInfo(&mut target.header) } != 0 {
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