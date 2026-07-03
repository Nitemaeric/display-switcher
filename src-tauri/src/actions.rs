use std::process::Command;

use crate::config::PostAction;
use crate::steam;

pub fn run_post_action(action: &PostAction, steam_path_setting: &str) -> Result<(), String> {
    match action {
        PostAction::Builtin { action } => match action.as_str() {
            "none" => Ok(()),
            "launch-steam-big-picture" => {
                let steam = steam::find_steam_exe(steam_path_setting)
                    .ok_or_else(|| "Steam executable not found".to_string())?;
                steam::launch_big_picture(&steam)
            }
            "exit-steam-big-picture" => steam::exit_big_picture(),
            other => Err(format!("Unknown builtin action: {other}")),
        },
        PostAction::LaunchProgram { path, args } => {
            let mut cmd = Command::new(path);
            if let Some(a) = args {
                for part in a.split_whitespace() {
                    cmd.arg(part);
                }
            }
            cmd.spawn()
                .map_err(|e| format!("Failed to launch program: {e}"))?;
            Ok(())
        }
        PostAction::RunCommand { command } => {
            Command::new("cmd")
                .args(["/C", "start", "", "cmd", "/C", command])
                .spawn()
                .map_err(|e| format!("Failed to run command: {e}"))?;
            Ok(())
        }
    }
}

pub const BUILTIN_ACTIONS: &[(&str, &str)] = &[
    ("none", "Switch displays only"),
    ("launch-steam-big-picture", "Launch Steam Big Picture"),
    ("exit-steam-big-picture", "Exit Steam Big Picture"),
];