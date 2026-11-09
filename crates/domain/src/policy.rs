#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PlaybackDecision {
    Run,
    Pause,
    Unload,
}

impl PlaybackDecision {
    pub fn explanation(self) -> &'static str {
        match self {
            Self::Run => "Run · frame ceiling applies",
            Self::Pause => "Pause · retain scene, submit no frames",
            Self::Unload => "Unload · release scene and audio subscriptions",
        }
    }
}

#[derive(Debug, Default, Clone, Copy)]
pub struct PlaybackContext {
    pub locked: bool,
    pub display_off: bool,
    pub fullscreen: bool,
    pub on_battery: bool,
    pub user_paused: bool,
    pub release_for_games: bool,
}

pub fn decide_playback(context: PlaybackContext) -> PlaybackDecision {
    if context.locked || context.display_off || (context.fullscreen && context.release_for_games) {
        PlaybackDecision::Unload
    } else if context.user_paused || context.fullscreen || context.on_battery {
        PlaybackDecision::Pause
    } else {
        PlaybackDecision::Run
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DisplayInfo {
    pub name: String,
    pub x: i32,
    pub y: i32,
    pub width: u32,
    pub height: u32,
    pub primary: bool,
}

pub fn virtual_bounds(displays: &[DisplayInfo]) -> Option<(i64, i64, u64, u64)> {
    let left = displays.iter().map(|d| i64::from(d.x)).min()?;
    let top = displays.iter().map(|d| i64::from(d.y)).min()?;
    let right = displays
        .iter()
        .map(|d| i64::from(d.x) + i64::from(d.width))
        .max()?;
    let bottom = displays
        .iter()
        .map(|d| i64::from(d.y) + i64::from(d.height))
        .max()?;
    Some((left, top, (right - left) as u64, (bottom - top) as u64))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn lock_overrides_other_policies() {
        assert_eq!(
            decide_playback(PlaybackContext {
                locked: true,
                user_paused: true,
                ..Default::default()
            }),
            PlaybackDecision::Unload
        );
    }
    #[test]
    fn game_can_pause_or_release_memory() {
        let mut context = PlaybackContext {
            fullscreen: true,
            ..Default::default()
        };
        assert_eq!(decide_playback(context), PlaybackDecision::Pause);
        context.release_for_games = true;
        assert_eq!(decide_playback(context), PlaybackDecision::Unload);
    }
    #[test]
    fn span_includes_negative_origin_and_portrait_monitor() {
        let displays = [
            DisplayInfo {
                name: "Portrait".into(),
                x: -1080,
                y: -400,
                width: 1080,
                height: 1920,
                primary: false,
            },
            DisplayInfo {
                name: "Main".into(),
                x: 0,
                y: 0,
                width: 2560,
                height: 1440,
                primary: true,
            },
        ];
        assert_eq!(virtual_bounds(&displays), Some((-1080, -400, 3640, 1920)));
        assert_eq!(virtual_bounds(&[]), None);
    }
}
