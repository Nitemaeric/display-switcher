use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::thread;
use std::time::{Duration, Instant};

use parking_lot::RwLock;

use crate::config::{is_group_activatable, AppConfig, GamepadChord};

#[derive(Clone)]
pub struct GamepadManager {
    running: Arc<AtomicBool>,
    config: Arc<RwLock<AppConfig>>,
    on_activate: Arc<dyn Fn(String) + Send + Sync>,
}

const XUSER_MAX_COUNT: u32 = 4;

// XInput button flags
const XINPUT_GAMEPAD_START: u16 = 0x0010;
const XINPUT_GAMEPAD_BACK: u16 = 0x0020;
const XINPUT_GAMEPAD_LEFT_SHOULDER: u16 = 0x0100;
const XINPUT_GAMEPAD_RIGHT_SHOULDER: u16 = 0x0200;
const XINPUT_GAMEPAD_A: u16 = 0x1000;
const XINPUT_GAMEPAD_B: u16 = 0x2000;
const XINPUT_GAMEPAD_X: u16 = 0x4000;
const XINPUT_GAMEPAD_Y: u16 = 0x8000;

#[repr(C)]
struct XInputGamepad {
    buttons: u16,
    left_trigger: u8,
    right_trigger: u8,
    thumb_lx: i16,
    thumb_ly: i16,
    thumb_rx: i16,
    thumb_ry: i16,
}

#[repr(C)]
struct XInputState {
    packet_number: u32,
    gamepad: XInputGamepad,
}

type XInputGetStateFn = unsafe extern "system" fn(u32, *mut XInputState) -> u32;

impl GamepadManager {
    pub fn new<F>(on_activate: F) -> Self
    where
        F: Fn(String) + Send + Sync + 'static,
    {
        Self {
            running: Arc::new(AtomicBool::new(false)),
            config: Arc::new(RwLock::new(AppConfig::default())),
            on_activate: Arc::new(on_activate),
        }
    }

    pub fn update_config(&self, config: AppConfig) {
        *self.config.write() = config;
    }

    pub fn start(&self) {
        if self.running.swap(true, Ordering::SeqCst) {
            return;
        }

        let running = self.running.clone();
        let config = self.config.clone();
        let on_activate = self.on_activate.clone();

        thread::spawn(move || {
            let xinput = load_xinput();
            let mut chord_start: Option<Instant> = None;
            let mut active_group: Option<String> = None;

            while running.load(Ordering::SeqCst) {
                thread::sleep(Duration::from_millis(50));

                let Some(get_state) = xinput else {
                    continue;
                };

                let cfg = config.read().clone();
                let mut state = XInputState {
                    packet_number: 0,
                    gamepad: XInputGamepad {
                        buttons: 0,
                        left_trigger: 0,
                        right_trigger: 0,
                        thumb_lx: 0,
                        thumb_ly: 0,
                        thumb_rx: 0,
                        thumb_ry: 0,
                    },
                };

                let mut connected = false;
                for i in 0..XUSER_MAX_COUNT {
                    if unsafe { get_state(i, &mut state) } == 0 {
                        connected = true;
                        break;
                    }
                }

                if !connected {
                    chord_start = None;
                    active_group = None;
                    continue;
                }

                let pressed = state.gamepad.buttons;
                let mut matched_group: Option<(String, u64)> = None;

                for group in &cfg.groups {
                    if !is_group_activatable(group) {
                        continue;
                    }
                    let Some(chord) = &group.gamepad_chord else {
                        continue;
                    };
                    if chord_matches(pressed, chord) {
                        matched_group = Some((group.id.clone(), chord.hold_ms));
                        break;
                    }
                }

                match matched_group {
                    Some((group_id, hold_ms)) if active_group.as_ref() == Some(&group_id) => {
                        if let Some(start) = chord_start {
                            if start.elapsed() >= Duration::from_millis(hold_ms) {
                                on_activate(group_id.clone());
                                chord_start = None;
                                active_group = None;
                                thread::sleep(Duration::from_millis(500));
                            }
                        }
                    }
                    Some((group_id, _)) => {
                        chord_start = Some(Instant::now());
                        active_group = Some(group_id);
                    }
                    None => {
                        chord_start = None;
                        active_group = None;
                    }
                }
            }
        });
    }

    pub fn stop(&self) {
        self.running.store(false, Ordering::SeqCst);
    }
}

fn chord_matches(pressed: u16, chord: &GamepadChord) -> bool {
    let required: u16 = chord
        .buttons
        .iter()
        .filter_map(|b| button_name_to_flag(b))
        .fold(0, |acc, flag| acc | flag);

    if required == 0 {
        return false;
    }
    (pressed & required) == required
}

fn button_name_to_flag(name: &str) -> Option<u16> {
    match name.to_uppercase().as_str() {
        "START" => Some(XINPUT_GAMEPAD_START),
        "BACK" | "SELECT" => Some(XINPUT_GAMEPAD_BACK),
        "LB" | "LEFT_SHOULDER" => Some(XINPUT_GAMEPAD_LEFT_SHOULDER),
        "RB" | "RIGHT_SHOULDER" => Some(XINPUT_GAMEPAD_RIGHT_SHOULDER),
        "A" => Some(XINPUT_GAMEPAD_A),
        "B" => Some(XINPUT_GAMEPAD_B),
        "X" => Some(XINPUT_GAMEPAD_X),
        "Y" => Some(XINPUT_GAMEPAD_Y),
        _ => None,
    }
}

fn load_xinput() -> Option<XInputGetStateFn> {
    unsafe {
        let lib = match windows::Win32::System::LibraryLoader::LoadLibraryW(windows::core::w!(
            "XInput1_4.dll"
        )) {
            Ok(lib) => lib,
            Err(_) => windows::Win32::System::LibraryLoader::LoadLibraryW(windows::core::w!(
                "XInput9_1_0.dll"
            ))
            .ok()?,
        };

        let proc = windows::Win32::System::LibraryLoader::GetProcAddress(
            lib,
            windows::core::PCSTR(b"XInputGetState\0".as_ptr()),
        )?;
        Some(std::mem::transmute(proc))
    }
}

pub fn list_gamepad_buttons() -> Vec<&'static str> {
    vec!["Start", "Back", "LB", "RB", "A", "B", "X", "Y"]
}