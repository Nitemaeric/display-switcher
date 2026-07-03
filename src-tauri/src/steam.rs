use std::path::PathBuf;
use std::process::Command;
use std::thread;
use std::time::Duration;

use windows::core::w;
use windows::Win32::System::Registry::{
    RegCloseKey, RegOpenKeyExW, RegQueryValueExW, HKEY_LOCAL_MACHINE, KEY_READ,
};
use windows::Win32::UI::Input::KeyboardAndMouse::{
    SendInput, INPUT, INPUT_0, INPUT_KEYBOARD, KEYBDINPUT, KEYEVENTF_KEYUP, VIRTUAL_KEY,
};

pub fn find_steam_exe(configured: &str) -> Option<PathBuf> {
    if configured != "auto" && !configured.is_empty() {
        let path = PathBuf::from(configured);
        if path.exists() {
            return Some(path);
        }
    }

    for candidate in [
        r"C:\Program Files (x86)\Steam\steam.exe",
        r"C:\Program Files\Steam\steam.exe",
    ] {
        let p = PathBuf::from(candidate);
        if p.exists() {
            return Some(p);
        }
    }

    read_steam_from_registry()
}

fn read_steam_from_registry() -> Option<PathBuf> {
    unsafe {
        let mut key = Default::default();
        if RegOpenKeyExW(
            HKEY_LOCAL_MACHINE,
            w!("SOFTWARE\\WOW6432Node\\Valve\\Steam"),
            Some(0),
            KEY_READ,
            &mut key,
        )
        .is_err()
        {
            return None;
        }

        let mut buf = [0u16; 512];
        let mut size = (buf.len() * 2) as u32;
        let result = RegQueryValueExW(
            key,
            w!("InstallPath"),
            None,
            None,
            Some(buf.as_mut_ptr() as *mut u8),
            Some(&mut size),
        );
        let _ = RegCloseKey(key);

        if result.is_err() {
            return None;
        }

        let len = buf.iter().position(|&c| c == 0).unwrap_or(buf.len());
        let install = String::from_utf16_lossy(&buf[..len]);
        let path = PathBuf::from(install).join("steam.exe");
        if path.exists() {
            Some(path)
        } else {
            None
        }
    }
}

pub fn is_steam_running() -> bool {
    Command::new("tasklist")
        .args(["/FI", "IMAGENAME eq steam.exe"])
        .output()
        .map(|o| String::from_utf8_lossy(&o.stdout).contains("steam.exe"))
        .unwrap_or(false)
}

pub fn launch_big_picture(steam_path: &PathBuf) -> Result<(), String> {
    if !is_steam_running() {
        Command::new(steam_path)
            .spawn()
            .map_err(|e| format!("Failed to start Steam: {e}"))?;
        thread::sleep(Duration::from_secs(2));
    }

    Command::new(steam_path)
        .arg("-bigpicture")
        .spawn()
        .map_err(|e| format!("Failed to launch Big Picture: {e}"))?;
    Ok(())
}

pub fn exit_big_picture() -> Result<(), String> {
    thread::sleep(Duration::from_millis(3500));

    let _ = Command::new("cmd")
        .args(["/C", "start", "", "steam://close/bigpicture"])
        .spawn();

    thread::sleep(Duration::from_millis(800));
    send_alt_enter();
    Ok(())
}

fn send_alt_enter() {
    unsafe {
        let mut inputs = [
            make_key_input(VIRTUAL_KEY(0x12), false),
            make_key_input(VIRTUAL_KEY(0x0D), false),
            make_key_input(VIRTUAL_KEY(0x0D), true),
            make_key_input(VIRTUAL_KEY(0x12), true),
        ];
        SendInput(&mut inputs, std::mem::size_of::<INPUT>() as i32);
    }
}

unsafe fn make_key_input(vk: VIRTUAL_KEY, key_up: bool) -> INPUT {
    INPUT {
        r#type: INPUT_KEYBOARD,
        Anonymous: INPUT_0 {
            ki: KEYBDINPUT {
                wVk: vk,
                wScan: 0,
                dwFlags: if key_up { KEYEVENTF_KEYUP } else { Default::default() },
                time: 0,
                dwExtraInfo: 0,
            },
        },
    }
}