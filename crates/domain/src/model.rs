use serde::{Deserialize, Serialize};
use std::{fs, io::Read, path::Path};
use thiserror::Error;

pub const SCHEMA_VERSION: u32 = 1;
pub const MAX_PROJECT_BYTES: u64 = 2 * 1024 * 1024;
pub const DEFAULT_SHADER: &str = include_str!("../../../examples/aurora/main.wgsl");

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FilterKind {
    Peak,
    LowPass,
    HighPass,
    LowShelf,
    HighShelf,
    Notch,
}

impl FilterKind {
    pub fn uses_gain(self) -> bool {
        matches!(self, Self::Peak | Self::LowShelf | Self::HighShelf)
    }
    pub const ALL: [Self; 6] = [
        Self::Peak,
        Self::LowPass,
        Self::HighPass,
        Self::LowShelf,
        Self::HighShelf,
        Self::Notch,
    ];
}

impl std::fmt::Display for FilterKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            Self::Peak => "Bell",
            Self::LowPass => "Low pass",
            Self::HighPass => "High pass",
            Self::LowShelf => "Low shelf",
            Self::HighShelf => "High shelf",
            Self::Notch => "Notch",
        })
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct EqBand {
    pub frequency_hz: f32,
    pub gain_db: f32,
    pub q: f32,
    pub kind: FilterKind,
    pub enabled: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ResponseBus {
    pub name: String,
    pub low_hz: f32,
    pub high_hz: f32,
    pub gain: f32,
    pub attack_ms: f32,
    pub release_ms: f32,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AudioSettings {
    pub bands: [EqBand; 7],
    pub bypass: bool,
    pub bus: ResponseBus,
}

impl Default for AudioSettings {
    fn default() -> Self {
        Self {
            bands: [50., 120., 350., 1000., 3000., 8000., 16000.].map(|frequency_hz| EqBand {
                frequency_hz,
                gain_db: 0.,
                q: 0.707,
                kind: FilterKind::Peak,
                enabled: true,
            }),
            bypass: false,
            bus: ResponseBus {
                name: "Bass pulse".into(),
                low_hz: 30.,
                high_hz: 220.,
                gain: 3.,
                attack_ms: 25.,
                release_ms: 240.,
            },
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Project {
    pub schema_version: u32,
    pub name: String,
    pub shader: String,
    pub audio: AudioSettings,
    pub preview_fps: u32,
    #[serde(default)]
    pub appearance: crate::Appearance,
    #[serde(default)]
    pub image: Option<crate::ImageLayer>,
}

impl Default for Project {
    fn default() -> Self {
        Self {
            schema_version: SCHEMA_VERSION,
            name: "Chromatic current".into(),
            shader: DEFAULT_SHADER.into(),
            audio: AudioSettings::default(),
            preview_fps: 30,
            appearance: crate::Appearance::default(),
            image: None,
        }
    }
}

#[derive(Debug, Error)]
pub enum ProjectError {
    #[error("{0}")]
    Invalid(String),
    #[error("Unable to access the project: {0}. Choose a writable local folder and try again.")]
    Io(#[from] std::io::Error),
    #[error(
        "Unable to read the project JSON: {0}. Open a Serein project exported by this version."
    )]
    Json(#[from] serde_json::Error),
}

fn in_range(value: f32, low: f32, high: f32) -> bool {
    value.is_finite() && (low..=high).contains(&value)
}

impl Project {
    pub fn validate(&self) -> Result<(), ProjectError> {
        let invalid = |s: &str| ProjectError::Invalid(s.into());
        if self.schema_version != SCHEMA_VERSION {
            return Err(invalid(
                "This project uses an unsupported schema. Use the matching Serein version; the file has not been changed.",
            ));
        }
        if self.name.trim().is_empty() || self.name.chars().count() > 120 {
            return Err(invalid("Use a project name between 1 and 120 characters."));
        }
        if self.shader.len() > 256 * 1024 {
            return Err(invalid(
                "The shader exceeds 256 KiB. Split the effect before importing it.",
            ));
        }
        if !(1..=120).contains(&self.preview_fps) {
            return Err(invalid("Choose a preview ceiling between 1 and 120 FPS."));
        }
        let look = &self.appearance;
        if let Some(image) = &self.image {
            for path in std::iter::once(&image.path).chain(image.depth_path.iter()) {
                let value = path.to_string_lossy();
                if value.starts_with("\\\\") || value.starts_with("//") || value.contains("://") {
                    return Err(invalid(
                        "Wallpaper images must be local files. Import an image from this computer.",
                    ));
                }
            }
        }
        if !in_range(look.speed, 0., 4.)
            || !in_range(look.brightness, 0.1, 2.)
            || !in_range(look.detail, 0.1, 3.)
            || !in_range(look.audio_amount, 0., 5.)
            || !in_range(look.parallax, 0., 0.12)
            || look.palette > 3
        {
            return Err(invalid(
                "Wallpaper controls are outside their supported ranges. Reset the wallpaper's appearance settings.",
            ));
        }
        for band in self.audio.bands {
            if !in_range(band.frequency_hz, 20., 20000.)
                || !in_range(band.gain_db, -24., 24.)
                || !in_range(band.q, 0.1, 18.)
            {
                return Err(invalid(
                    "An EQ band is outside its limits. Use 20–20000 Hz, ±24 dB and Q 0.1–18.",
                ));
            }
        }
        let bus = &self.audio.bus;
        if bus.name.trim().is_empty()
            || bus.name.chars().count() > 64
            || !in_range(bus.low_hz, 20., 19999.)
            || !in_range(bus.high_hz, 21., 20000.)
            || bus.low_hz >= bus.high_hz
            || !in_range(bus.gain, 0., 20.)
            || !in_range(bus.attack_ms, 1., 2000.)
            || !in_range(bus.release_ms, 1., 5000.)
        {
            return Err(invalid(
                "Check the response bus: choose an ordered 20–20000 Hz range, gain 0–20, attack 1–2000 ms and release 1–5000 ms.",
            ));
        }
        Ok(())
    }

    pub fn load(path: &Path) -> Result<Self, ProjectError> {
        // Bound the read itself: metadata can race a file that is still growing.
        let file = fs::File::open(path)?;
        let mut bytes = Vec::new();
        file.take(MAX_PROJECT_BYTES + 1).read_to_end(&mut bytes)?;
        if bytes.len() as u64 > MAX_PROJECT_BYTES {
            return Err(ProjectError::Invalid(
                "Project exceeds 2 MiB. Open a smaller Serein project file.".into(),
            ));
        }
        let project: Self = serde_json::from_slice(&bytes)?;
        project.validate()?;
        Ok(project)
    }

    pub fn save(&self, path: &Path) -> Result<(), ProjectError> {
        use std::io::Write;
        self.validate()?;
        let bytes = serde_json::to_vec_pretty(self)?;
        let temp = path.with_extension(format!("{}.tmp", std::process::id()));
        // create_new avoids clobbering a stale recovery file or following a link.
        let mut file = fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&temp)?;
        let result = (|| {
            file.write_all(&bytes)?;
            file.sync_all()?;
            drop(file);
            fs::rename(&temp, path)
        })();
        if result.is_err() {
            let _ = fs::remove_file(&temp);
        }
        result?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn appearance_and_image_import_limits_are_validated() {
        let mut project = Project::default();
        project.appearance.brightness = f32::INFINITY;
        assert!(project.validate().is_err());
        project.appearance = crate::Appearance::default();
        project.image = Some(crate::ImageLayer {
            path: "\\\\server\\wallpaper.png".into(),
            depth_path: None,
        });
        assert!(project.validate().is_err());
        project.image = Some(crate::ImageLayer {
            path: "C:/Pictures/local.png".into(),
            depth_path: None,
        });
        assert!(project.validate().is_ok());
    }
    #[test]
    fn shipped_project_round_trips_and_unknown_fields_are_rejected() {
        let source = include_str!("../../../examples/aurora/project.serein.json");
        let project: Project = serde_json::from_str(source).expect("shipped project");
        project.validate().expect("valid project");
        let mut value = serde_json::to_value(&project).expect("serialize");
        value["unknown_runtime"] = true.into();
        assert!(serde_json::from_value::<Project>(value).is_err());
        let restored: Project =
            serde_json::from_str(&serde_json::to_string(&project).expect("serialize"))
                .expect("round trip");
        assert_eq!(restored, project);
    }
    #[test]
    fn rejects_future_versions_and_non_finite_filter_settings() {
        let mut project = Project {
            schema_version: 99,
            ..Project::default()
        };
        assert!(project.validate().is_err());
        project.schema_version = SCHEMA_VERSION;
        project.audio.bands[2].q = f32::NAN;
        assert!(project.validate().is_err());
    }
    #[test]
    fn rejects_reversed_bus_ranges() {
        let mut project = Project::default();
        project.audio.bus.low_hz = 1000.;
        project.audio.bus.high_hz = 100.;
        assert!(project.validate().is_err());
    }
    #[test]
    fn failed_validation_preserves_existing_file() {
        let path = std::env::temp_dir().join(format!("serein-project-{}.json", std::process::id()));
        let mut project = Project::default();
        project.save(&path).expect("initial save");
        project.name.clear();
        assert!(project.save(&path).is_err());
        assert_eq!(
            Project::load(&path).expect("previous file remains"),
            Project::default()
        );
        std::fs::remove_file(path).expect("cleanup");
    }
}
