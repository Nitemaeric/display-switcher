#[cfg(target_os = "windows")]
mod apply;
#[cfg(target_os = "windows")]
mod capture;
#[cfg(target_os = "windows")]
mod enumerate;
#[cfg(target_os = "windows")]
mod remap;

pub mod types;

pub use types::{DisplayInfo, DisplayProfile};

#[cfg(target_os = "windows")]
pub use apply::{
    activate_assigned_displays, apply_profile, displays_all_active, profile_covers_displays,
    sanitize_profile_for_group, validate_profile_safe, validate_profile_with_windows,
};
#[cfg(target_os = "windows")]
pub use capture::capture_current_profile;
#[cfg(target_os = "windows")]
pub use enumerate::list_displays;

#[cfg(not(target_os = "windows"))]
pub fn list_displays() -> Result<Vec<DisplayInfo>, String> {
    Err("Display switching is only supported on Windows".into())
}

#[cfg(not(target_os = "windows"))]
pub fn capture_current_profile() -> Result<DisplayProfile, String> {
    Err("Display capture is only supported on Windows".into())
}

#[cfg(not(target_os = "windows"))]
pub fn apply_profile(_profile: &DisplayProfile) -> Result<(), String> {
    Err("Display switching is only supported on Windows".into())
}

#[cfg(not(target_os = "windows"))]
pub fn validate_profile_safe(_profile: &DisplayProfile, _group_display_ids: &[String]) -> Result<(), String> {
    Err("Display switching is only supported on Windows".into())
}

#[cfg(not(target_os = "windows"))]
pub fn activate_assigned_displays(_profile: &mut DisplayProfile, _group_display_ids: &[String]) -> Result<(), String> {
    Err("Display switching is only supported on Windows".into())
}

#[cfg(not(target_os = "windows"))]
pub fn validate_profile_with_windows(_profile: &DisplayProfile) -> Result<(), String> {
    Err("Display switching is only supported on Windows".into())
}

#[cfg(not(target_os = "windows"))]
pub fn profile_covers_displays(_profile: &DisplayProfile, _group_display_ids: &[String]) -> bool {
    false
}

#[cfg(not(target_os = "windows"))]
pub fn displays_all_active(_group_display_ids: &[String]) -> bool {
    false
}