use std::collections::HashMap;
use std::ffi::OsString;
use std::os::windows::ffi::OsStringExt;

use windows::core::BOOL;
use windows::Win32::Devices::Display::{
    DISPLAYCONFIG_MODE_INFO, DISPLAYCONFIG_MODE_INFO_TYPE_SOURCE, DISPLAYCONFIG_PATH_INFO,
};
use windows::Win32::Foundation::LPARAM;
use windows::Win32::Graphics::Gdi::{
    EnumDisplayDevicesW, EnumDisplayMonitors, GetMonitorInfoW, DISPLAY_DEVICEW,
    DISPLAY_DEVICE_ATTACHED_TO_DESKTOP, DISPLAY_DEVICE_MIRRORING_DRIVER,
    DISPLAY_DEVICE_PRIMARY_DEVICE, HDC, HMONITOR, MONITORINFOEXW,
};
use windows::Win32::UI::WindowsAndMessaging::MONITORINFOF_PRIMARY;

use super::capture::{get_source_name_for_path, get_target_name_for_path, query_raw_config};
use super::types::DisplayInfo;

/// DISPLAYCONFIG_PATH_ACTIVE
const PATH_ACTIVE: u32 = 0x00000001;
const MODE_IDX_INVALID: u32 = u32::MAX;

#[derive(Clone)]
struct MonitorGeometry {
    width: u32,
    height: u32,
    x: i32,
    y: i32,
    is_primary: bool,
}

#[derive(Clone)]
struct PathCandidate {
    gdi_name: String,
    friendly_name: String,
    is_active: bool,
    width: u32,
    height: u32,
    x: i32,
    y: i32,
    is_primary: bool,
}

struct GeometryContext {
    monitors: HashMap<String, MonitorGeometry>,
    error: Option<String>,
}

pub fn list_displays() -> Result<Vec<DisplayInfo>, String> {
    let active_geometry = collect_active_monitor_geometry()?;
    let mut by_id: HashMap<String, DisplayInfo> = HashMap::new();

    if let Ok((paths, modes)) = query_raw_config() {
        let mut by_target: HashMap<String, PathCandidate> = HashMap::new();

        for path in &paths {
            if let Some(candidate) = path_candidate(path, &modes, &active_geometry) {
                let key = target_key(path);
                by_target
                    .entry(key)
                    .and_modify(|existing| merge_candidate(existing, &candidate))
                    .or_insert(candidate);
            }
        }

        for candidate in by_target.into_values() {
            insert_display(&mut by_id, DisplayInfo::from(candidate));
        }
    }

    for gdi_device in enumerate_gdi_devices()? {
        insert_display(&mut by_id, gdi_device);
    }

    if by_id.is_empty() {
        return list_displays_from_monitors_only(active_geometry);
    }

    let mut displays: Vec<_> = by_id.into_values().collect();
    for display in &mut displays {
        if display.width == 0
            && display.height == 0
            && !active_geometry.contains_key(&display.id)
        {
            display.is_active = false;
        }
    }

    displays = finalize_displays(displays);
    displays.sort_by(|a, b| a.id.cmp(&b.id));
    Ok(displays)
}

fn path_candidate(
    path: &DISPLAYCONFIG_PATH_INFO,
    modes: &[DISPLAYCONFIG_MODE_INFO],
    active_geometry: &HashMap<String, MonitorGeometry>,
) -> Option<PathCandidate> {
    let gdi_name = get_source_name_for_path(path).unwrap_or_default();
    let friendly_name = get_target_name_for_path(path)
        .filter(|name| !name.is_empty())
        .unwrap_or_else(|| {
            if gdi_name.is_empty() {
                target_display_id(path)
            } else {
                gdi_name.clone()
            }
        });

    if friendly_name.is_empty() && gdi_name.is_empty() {
        return None;
    }

    let is_active = path.flags & PATH_ACTIVE != 0;
    let geometry = if !gdi_name.is_empty() {
        active_geometry.get(&gdi_name)
    } else {
        None
    };
    let (width, height, x, y) = geometry
        .map(|g| (g.width, g.height, g.x, g.y))
        .or_else(|| source_mode_geometry(path, modes))
        .unwrap_or((0, 0, 0, 0));
    let is_primary = geometry.map(|g| g.is_primary).unwrap_or(false);

    Some(PathCandidate {
        gdi_name: if gdi_name.is_empty() {
            target_display_id(path)
        } else {
            gdi_name
        },
        friendly_name,
        is_active,
        width,
        height,
        x,
        y,
        is_primary,
    })
}

fn target_key(path: &DISPLAYCONFIG_PATH_INFO) -> String {
    let adapter = path.targetInfo.adapterId;
    format!(
        "{}:{}:{}",
        adapter.HighPart, adapter.LowPart, path.targetInfo.id
    )
}

fn target_display_id(path: &DISPLAYCONFIG_PATH_INFO) -> String {
    format!("target:{}", target_key(path))
}

fn merge_candidate(existing: &mut PathCandidate, incoming: &PathCandidate) {
    if incoming.is_active && !existing.is_active {
        *existing = incoming.clone();
        return;
    }

    if existing.is_active && !incoming.is_active {
        if existing.width == 0
            && existing.height == 0
            && (incoming.width > 0 || incoming.height > 0)
        {
            existing.width = incoming.width;
            existing.height = incoming.height;
            existing.x = incoming.x;
            existing.y = incoming.y;
        }
        if existing.name_is_fallback() && !incoming.name_is_fallback() {
            existing.friendly_name = incoming.friendly_name.clone();
        }
        return;
    }

    if existing.gdi_name.starts_with("target:") && !incoming.gdi_name.starts_with("target:") {
        existing.gdi_name = incoming.gdi_name.clone();
    }
    if existing.name_is_fallback() && !incoming.name_is_fallback() {
        existing.friendly_name = incoming.friendly_name.clone();
    }
    if existing.width == 0 && incoming.width > 0 {
        existing.width = incoming.width;
        existing.height = incoming.height;
        existing.x = incoming.x;
        existing.y = incoming.y;
    }
    existing.is_active |= incoming.is_active;
    existing.is_primary |= incoming.is_primary;
}

impl PathCandidate {
    fn name_is_fallback(&self) -> bool {
        self.friendly_name == self.gdi_name || self.friendly_name.starts_with("target:")
    }
}

impl From<PathCandidate> for DisplayInfo {
    fn from(candidate: PathCandidate) -> Self {
        DisplayInfo {
            id: candidate.gdi_name,
            name: candidate.friendly_name,
            is_active: candidate.is_active,
            is_primary: candidate.is_primary,
            width: candidate.width,
            height: candidate.height,
            x: candidate.x,
            y: candidate.y,
        }
    }
}

fn insert_display(by_id: &mut HashMap<String, DisplayInfo>, display: DisplayInfo) {
    if let Some(existing_key) = find_existing_key(by_id, &display) {
        let mut existing = by_id
            .remove(&existing_key)
            .expect("display entry missing from map");
        merge_display(&mut existing, &display);
        by_id.insert(existing.id.clone(), existing);
        return;
    }

    by_id.insert(display.id.clone(), display);
}

fn find_existing_key(by_id: &HashMap<String, DisplayInfo>, display: &DisplayInfo) -> Option<String> {
    if by_id.contains_key(&display.id) {
        return Some(display.id.clone());
    }

    let normalized_name = normalize_name(&display.name);
    if normalized_name.is_empty() {
        return None;
    }

    by_id
        .iter()
        .find(|(_, existing)| {
            normalize_name(&existing.name) == normalized_name
                && (existing.is_active == display.is_active
                    || !existing.is_active
                    || !display.is_active)
        })
        .map(|(key, _)| key.clone())
}

fn merge_display(existing: &mut DisplayInfo, incoming: &DisplayInfo) {
    let incoming_name = incoming.name.clone();
    let incoming_id = incoming.id.clone();

    if incoming.is_active && !existing.is_active {
        existing.id = incoming_id;
        existing.is_active = true;
        existing.width = incoming.width;
        existing.height = incoming.height;
        existing.x = incoming.x;
        existing.y = incoming.y;
        existing.is_primary = incoming.is_primary;
    } else if existing.is_active && !incoming.is_active {
        if existing.width == 0 && incoming.width > 0 {
            existing.width = incoming.width;
            existing.height = incoming.height;
            existing.x = incoming.x;
            existing.y = incoming.y;
        }
    } else if incoming.width > existing.width {
        existing.width = incoming.width;
        existing.height = incoming.height;
        existing.x = incoming.x;
        existing.y = incoming.y;
    }

    if name_is_generic(existing) && !name_is_generic_name(&incoming_name) {
        existing.name = incoming_name;
    }
    if incoming.is_primary {
        existing.is_primary = true;
    }
}

fn name_is_generic(display: &DisplayInfo) -> bool {
    name_is_generic_name(&display.name) || display.name == display.id
}

fn name_is_generic_name(name: &str) -> bool {
    name.starts_with("target:") || name.starts_with("\\\\.\\DISPLAY")
}

fn normalize_name(name: &str) -> String {
    display_identity_key(name)
}

fn display_identity_key(name: &str) -> String {
    let base = name
        .split('(')
        .next()
        .unwrap_or(name)
        .trim()
        .to_lowercase();

    base.replace("display ", "")
        .replace("alienware ", "")
        .replace("generic pnp monitor", "generic pnp")
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
}

fn display_rank(display: &DisplayInfo) -> u8 {
    let has_resolution = display.width > 0 && display.height > 0;
    match (display.is_active, has_resolution) {
        (true, true) => 4,
        (false, true) => 3,
        (true, false) => 2,
        (false, false) => 1,
    }
}

fn is_junk_display(display: &DisplayInfo) -> bool {
    let name = display.name.to_lowercase();
    (name.contains("generic pnp") && display.width == 0 && display.height == 0)
        || (name.starts_with("target:"))
}

fn finalize_displays(displays: Vec<DisplayInfo>) -> Vec<DisplayInfo> {
    let mut best_by_identity: HashMap<String, DisplayInfo> = HashMap::new();

    for display in displays {
        if is_junk_display(&display) {
            continue;
        }

        let identity = display_identity_key(&display.name);
        best_by_identity
            .entry(identity)
            .and_modify(|best| {
                if display_rank(&display) > display_rank(best) {
                    *best = display.clone();
                } else if display_rank(&display) == display_rank(best) {
                    merge_display(best, &display);
                }
            })
            .or_insert(display);
    }

    best_by_identity.into_values().collect()
}

fn enumerate_gdi_devices() -> Result<Vec<DisplayInfo>, String> {
    let mut displays = Vec::new();
    let mut adapter_index = 0u32;

    loop {
        let mut adapter = DISPLAY_DEVICEW::default();
        adapter.cb = std::mem::size_of::<DISPLAY_DEVICEW>() as u32;

        let ok = unsafe { EnumDisplayDevicesW(None, adapter_index, &mut adapter, 0) };
        if !ok.as_bool() {
            break;
        }

        if (adapter.StateFlags & DISPLAY_DEVICE_MIRRORING_DRIVER).0 != 0 {
            adapter_index += 1;
            continue;
        }

        let mut monitor_index = 0u32;
        loop {
            let mut monitor = DISPLAY_DEVICEW::default();
            monitor.cb = std::mem::size_of::<DISPLAY_DEVICEW>() as u32;

            let ok = unsafe {
                EnumDisplayDevicesW(
                    windows::core::PCWSTR(adapter.DeviceName.as_ptr()),
                    monitor_index,
                    &mut monitor,
                    0,
                )
            };
            if !ok.as_bool() {
                break;
            }

            if (monitor.StateFlags & DISPLAY_DEVICE_MIRRORING_DRIVER).0 != 0 {
                monitor_index += 1;
                continue;
            }

            let gdi_name = wide_to_string(&monitor.DeviceName);
            let device_string = wide_to_string(&monitor.DeviceString);
            let flags = monitor.StateFlags;
            let is_active = (flags & DISPLAY_DEVICE_ATTACHED_TO_DESKTOP).0 != 0;
            let is_primary = (flags & DISPLAY_DEVICE_PRIMARY_DEVICE).0 != 0;
            let friendly_name = if device_string.is_empty() {
                gdi_name.clone()
            } else {
                device_string
            };

            displays.push(DisplayInfo {
                id: gdi_name,
                name: friendly_name,
                is_active,
                is_primary,
                width: 0,
                height: 0,
                x: 0,
                y: 0,
            });

            monitor_index += 1;
        }

        adapter_index += 1;
    }

    Ok(displays)
}

fn list_displays_from_monitors_only(
    active_geometry: HashMap<String, MonitorGeometry>,
) -> Result<Vec<DisplayInfo>, String> {
    let mut displays = Vec::new();

    for (id, geometry) in active_geometry {
        displays.push(DisplayInfo {
            id: id.clone(),
            name: id,
            is_active: true,
            is_primary: geometry.is_primary,
            width: geometry.width,
            height: geometry.height,
            x: geometry.x,
            y: geometry.y,
        });
    }

    displays.sort_by(|a, b| a.id.cmp(&b.id));
    Ok(displays)
}

fn collect_active_monitor_geometry() -> Result<HashMap<String, MonitorGeometry>, String> {
    let mut ctx = GeometryContext {
        monitors: HashMap::new(),
        error: None,
    };

    unsafe {
        let ok = EnumDisplayMonitors(
            None,
            None,
            Some(monitor_geometry_proc),
            LPARAM(&mut ctx as *mut _ as isize),
        );
        if !ok.as_bool() {
            return Err("EnumDisplayMonitors failed".into());
        }
    }

    if let Some(err) = ctx.error {
        return Err(err);
    }

    Ok(ctx.monitors)
}

unsafe extern "system" fn monitor_geometry_proc(
    h_monitor: HMONITOR,
    _hdc: HDC,
    _rect: *mut windows::Win32::Foundation::RECT,
    lparam: LPARAM,
) -> BOOL {
    let ctx = &mut *(lparam.0 as *mut GeometryContext);
    match collect_monitor_geometry(h_monitor) {
        Ok((id, geometry)) => {
            ctx.monitors.insert(id, geometry);
        }
        Err(e) => ctx.error = Some(e),
    }
    BOOL(1)
}

fn collect_monitor_geometry(h_monitor: HMONITOR) -> Result<(String, MonitorGeometry), String> {
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

    Ok((
        gdi_name,
        MonitorGeometry {
            width: (rect.right - rect.left) as u32,
            height: (rect.bottom - rect.top) as u32,
            x: rect.left,
            y: rect.top,
            is_primary: (info.monitorInfo.dwFlags & MONITORINFOF_PRIMARY) != 0,
        },
    ))
}

fn source_mode_geometry(
    path: &DISPLAYCONFIG_PATH_INFO,
    modes: &[DISPLAYCONFIG_MODE_INFO],
) -> Option<(u32, u32, i32, i32)> {
    let mode_idx = unsafe { path.sourceInfo.Anonymous.modeInfoIdx };
    if mode_idx == MODE_IDX_INVALID {
        return None;
    }

    let mode = modes.get(mode_idx as usize)?;
    if mode.infoType != DISPLAYCONFIG_MODE_INFO_TYPE_SOURCE {
        return None;
    }

    let source_mode = unsafe { mode.Anonymous.sourceMode };
    Some((
        source_mode.width,
        source_mode.height,
        source_mode.position.x,
        source_mode.position.y,
    ))
}

fn wide_to_string(wide: &[u16]) -> String {
    let len = wide.iter().position(|&c| c == 0).unwrap_or(wide.len());
    OsString::from_wide(&wide[..len])
        .to_string_lossy()
        .into_owned()
}

