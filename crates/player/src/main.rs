#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]
mod playback;
fn main() {
    let Some(path) = std::env::args().nth(1).map(std::path::PathBuf::from) else {
        eprintln!("Serein player is started by the wallpaper library.");
        return;
    };
    if let Err(error) = playback::run(&path) {
        let status = serein_domain::PlayerStatus {
            token: serein_domain::PlayerSession::load(&path)
                .map(|s| s.token)
                .unwrap_or(0),
            process_id: std::process::id(),
            running: false,
            paused: false,
            frames: 0,
            message: error,
        };
        if let Ok(bytes) = serde_json::to_vec(&status) {
            let _ = std::fs::write(path.with_extension("status.json"), bytes);
        }
    }
}
