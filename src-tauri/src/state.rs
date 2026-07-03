use parking_lot::RwLock;
use std::sync::Arc;

use crate::config::AppConfig;
use crate::gamepad::GamepadManager;

pub struct AppState {
    pub config: RwLock<AppConfig>,
    pub gamepad: GamepadManager,
}

impl AppState {
    pub fn new(gamepad: GamepadManager) -> Arc<Self> {
        Arc::new(Self {
            config: RwLock::new(AppConfig::default()),
            gamepad,
        })
    }

    pub fn get_config(&self) -> AppConfig {
        self.config.read().clone()
    }

    pub fn set_config(&self, config: AppConfig) {
        *self.config.write() = config.clone();
        self.gamepad.update_config(config);
    }
}