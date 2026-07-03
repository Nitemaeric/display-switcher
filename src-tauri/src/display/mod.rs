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
pub use apply::apply_profile;
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