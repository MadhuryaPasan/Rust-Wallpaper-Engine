use serde::{Deserialize, Serialize};
use serein_domain::{DisplayMode, data_directory};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct Preferences {
    pub selected_preset: Option<usize>,
    pub selected_local: Option<String>,
    pub output_id: Option<String>,
    pub display_mode: DisplayMode,
    pub monitor_name: Option<String>,
    pub pause_fullscreen: bool,
    pub pause_battery: bool,
}
impl Default for Preferences {
    fn default() -> Self {
        Self {
            selected_preset: None,
            selected_local: None,
            output_id: None,
            display_mode: DisplayMode::Primary,
            monitor_name: None,
            pause_fullscreen: true,
            pause_battery: true,
        }
    }
}
impl Preferences {
    pub fn load() -> Self {
        std::fs::read(data_directory().join("preferences.json"))
            .ok()
            .and_then(|bytes| serde_json::from_slice(&bytes).ok())
            .unwrap_or_default()
    }
    pub fn save(&self) -> Result<(), String> {
        let path = data_directory();
        std::fs::create_dir_all(&path).map_err(|e| e.to_string())?;
        let temp = path.join("preferences.pending");
        std::fs::write(
            &temp,
            serde_json::to_vec_pretty(self).map_err(|e| e.to_string())?,
        )
        .map_err(|e| e.to_string())?;
        std::fs::rename(temp, path.join("preferences.json")).map_err(|e| e.to_string())
    }
}
