use tauri::{Theme, WebviewWindow};

const LIGHT_BACKGROUND: u32 = colorref(0xfa, 0xfa, 0xfa);
const LIGHT_FOREGROUND: u32 = colorref(0x0a, 0x0a, 0x0a);
const DARK_BACKGROUND: u32 = colorref(0x0a, 0x0a, 0x0a);
const DARK_FOREGROUND: u32 = colorref(0xed, 0xed, 0xed);

pub fn resolve_theme(theme: &str) -> &'static str {
    match theme {
        "dark" => "dark",
        "light" => "light",
        _ => {
            #[cfg(windows)]
            {
                if system_prefers_dark() {
                    "dark"
                } else {
                    "light"
                }
            }
            #[cfg(not(windows))]
            {
                "light"
            }
        }
    }
}

pub fn apply_theme(window: &WebviewWindow, theme: &str) -> Result<(), String> {
    let resolved = resolve_theme(theme);
    let dark = resolved == "dark";

    let tauri_theme = if dark { Theme::Dark } else { Theme::Light };
    window
        .set_theme(Some(tauri_theme))
        .map_err(|e| e.to_string())?;

    #[cfg(windows)]
    apply_windows_title_bar(window, dark)?;

    Ok(())
}

#[cfg(windows)]
fn apply_windows_title_bar(window: &WebviewWindow, dark: bool) -> Result<(), String> {
    use raw_window_handle::{HasWindowHandle, RawWindowHandle};
    use windows::Win32::Foundation::HWND;
    use windows::Win32::Graphics::Dwm::{
        DwmSetWindowAttribute, DWMWA_BORDER_COLOR, DWMWA_CAPTION_COLOR, DWMWA_TEXT_COLOR,
        DWMWA_USE_IMMERSIVE_DARK_MODE,
    };

    let handle = window.window_handle().map_err(|e| e.to_string())?;
    let hwnd = match handle.as_raw() {
        RawWindowHandle::Win32(handle) => HWND(handle.hwnd.get() as _),
        _ => return Ok(()),
    };

    let (background, foreground) = if dark {
        (DARK_BACKGROUND, DARK_FOREGROUND)
    } else {
        (LIGHT_BACKGROUND, LIGHT_FOREGROUND)
    };

    let dark_mode = u32::from(dark);
    unsafe {
        let _ = DwmSetWindowAttribute(
            hwnd,
            DWMWA_USE_IMMERSIVE_DARK_MODE,
            &dark_mode as *const _ as _,
            std::mem::size_of::<u32>() as u32,
        );
        for (attribute, color) in [
            (DWMWA_CAPTION_COLOR, background),
            (DWMWA_BORDER_COLOR, background),
            (DWMWA_TEXT_COLOR, foreground),
        ] {
            let _ = DwmSetWindowAttribute(
                hwnd,
                attribute,
                &color as *const _ as _,
                std::mem::size_of::<u32>() as u32,
            );
        }
    }

    Ok(())
}

#[cfg(windows)]
fn system_prefers_dark() -> bool {
    use windows::core::w;
    use windows::Win32::System::Registry::{
        RegCloseKey, RegOpenKeyExW, RegQueryValueExW, HKEY_CURRENT_USER, KEY_READ,
    };

    unsafe {
        let mut key = Default::default();
        if RegOpenKeyExW(
            HKEY_CURRENT_USER,
            w!(r"Software\Microsoft\Windows\CurrentVersion\Themes\Personalize"),
            Some(0),
            KEY_READ,
            &mut key,
        )
        .is_err()
        {
            return false;
        }

        let mut value: u32 = 1;
        let mut size = std::mem::size_of::<u32>() as u32;
        let result = RegQueryValueExW(
            key,
            w!("AppsUseLightTheme"),
            None,
            None,
            Some(&mut value as *mut _ as *mut u8),
            Some(&mut size),
        );
        let _ = RegCloseKey(key);

        result.is_ok() && value == 0
    }
}

const fn colorref(r: u8, g: u8, b: u8) -> u32 {
    (r as u32) | ((g as u32) << 8) | ((b as u32) << 16)
}