use crate::{Project, ProjectError};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct Appearance {
    pub speed: f32,
    pub brightness: f32,
    pub detail: f32,
    pub audio_amount: f32,
    pub parallax: f32,
    pub palette: u32,
}
impl Default for Appearance {
    fn default() -> Self {
        Self {
            speed: 1.,
            brightness: 1.,
            detail: 1.,
            audio_amount: 1.,
            parallax: 0.035,
            palette: 0,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ImageLayer {
    pub path: PathBuf,
    pub depth_path: Option<PathBuf>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum DisplayMode {
    #[default]
    Primary,
    EveryDisplay,
    Span,
    Selected,
}
impl DisplayMode {
    pub const ALL: [Self; 4] = [
        Self::Primary,
        Self::EveryDisplay,
        Self::Span,
        Self::Selected,
    ];
}
impl std::fmt::Display for DisplayMode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            Self::Primary => "Primary display",
            Self::EveryDisplay => "Every display · duplicate",
            Self::Span => "Span all displays",
            Self::Selected => "Selected display",
        })
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PlayerSession {
    #[serde(default)]
    pub token: u64,
    pub project: Project,
    pub device_id: Option<String>,
    pub display_mode: DisplayMode,
    pub monitor_name: Option<String>,
    pub paused: bool,
    pub pause_fullscreen: bool,
    pub pause_battery: bool,
}
impl PlayerSession {
    pub fn validate(&self) -> Result<(), ProjectError> {
        self.project.validate()
    }
    pub fn load(path: &std::path::Path) -> Result<Self, String> {
        use std::io::Read;
        let mut bytes = Vec::new();
        std::fs::File::open(path)
            .map_err(|e| e.to_string())?
            .take(crate::MAX_PROJECT_BYTES + 1)
            .read_to_end(&mut bytes)
            .map_err(|e| e.to_string())?;
        if bytes.len() as u64 > crate::MAX_PROJECT_BYTES {
            return Err("Wallpaper session exceeds its size limit".into());
        }
        let session: Self = serde_json::from_slice(&bytes).map_err(|e| e.to_string())?;
        session.validate().map_err(|e| e.to_string())?;
        Ok(session)
    }
    pub fn save(&self, path: &std::path::Path) -> Result<(), String> {
        self.validate().map_err(|e| e.to_string())?;
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
        }
        let temp = path.with_extension("pending");
        std::fs::write(&temp, serde_json::to_vec(self).map_err(|e| e.to_string())?)
            .map_err(|e| e.to_string())?;
        std::fs::rename(temp, path).map_err(|e| e.to_string())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlayerStatus {
    pub token: u64,
    pub process_id: u32,
    pub running: bool,
    pub paused: bool,
    pub frames: u64,
    pub message: String,
}

pub fn data_directory() -> PathBuf {
    std::env::var_os("LOCALAPPDATA")
        .map(PathBuf::from)
        .unwrap_or_else(std::env::temp_dir)
        .join("Serein")
}

#[cfg(test)]
mod tests {
    #[test]
    fn session_round_trip_preserves_device_display_and_playback_choices() {
        let session = super::PlayerSession {
            token: 47,
            project: crate::Project::default(),
            device_id: Some("selected-endpoint".into()),
            display_mode: super::DisplayMode::Span,
            monitor_name: None,
            paused: false,
            pause_fullscreen: true,
            pause_battery: false,
        };
        let path =
            std::env::temp_dir().join(format!("serein-session-test-{}.json", std::process::id()));
        session.save(&path).expect("save session");
        assert_eq!(
            super::PlayerSession::load(&path).expect("load session"),
            session
        );
        std::fs::remove_file(path).expect("remove test file");
    }
}
