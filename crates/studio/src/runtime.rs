use serein_domain::{PlayerSession, PlayerStatus, data_directory};
use std::{
    path::PathBuf,
    process::{Child, Command},
};

pub struct Runtime {
    child: Option<Child>,
    pub session: Option<PlayerSession>,
    pub status: String,
    pub document_id: u64,
    pub running: bool,
    path: PathBuf,
    last_poll: std::time::Instant,
}
impl Default for Runtime {
    fn default() -> Self {
        Self {
            child: None,
            session: None,
            status: "Choose a wallpaper and select Apply wallpaper.".into(),
            document_id: 0,
            running: false,
            path: data_directory().join("runtime/session.json"),
            last_poll: std::time::Instant::now(),
        }
    }
}
impl Runtime {
    pub fn apply(&mut self, session: PlayerSession, document_id: u64) -> Result<(), String> {
        serein_render::compile(&session.project.shader)?;
        session.save(&self.path)?;
        let alive = self
            .child
            .as_mut()
            .is_some_and(|child| child.try_wait().ok().flatten().is_none());
        if !alive {
            let executable = std::env::current_exe()
                .map_err(|e| e.to_string())?
                .with_file_name("serein-player.exe");
            if !executable.is_file() {
                return Err("Desktop player is missing. Build the workspace with cargo build --workspace, then launch Serein again.".into());
            }
            let mut command = Command::new(executable);
            command.arg(&self.path);
            #[cfg(windows)]
            {
                use std::os::windows::process::CommandExt;
                command.creation_flags(0x0800_0000);
            }
            self.child = Some(
                command
                    .spawn()
                    .map_err(|e| format!("Unable to start the desktop player: {e}"))?,
            );
        }
        self.session = Some(session);
        self.document_id = document_id;
        self.status = "Applying wallpaper beneath desktop icons…".into();
        self.running = false;
        Ok(())
    }
    pub fn update(&mut self, session: PlayerSession) -> Result<(), String> {
        if self.session.as_ref() != Some(&session) {
            session.save(&self.path)?;
            self.session = Some(session);
        }
        Ok(())
    }
    pub fn poll(&mut self) {
        if self.last_poll.elapsed() < std::time::Duration::from_millis(250) {
            return;
        }
        self.last_poll = std::time::Instant::now();
        let Some(child) = &mut self.child else {
            return;
        };
        if let Ok(bytes) = std::fs::read(self.path.with_extension("status.json"))
            && let Ok(status) = serde_json::from_slice::<PlayerStatus>(&bytes)
            && status.process_id == child.id()
            && self
                .session
                .as_ref()
                .is_some_and(|session| session.token == status.token)
        {
            self.running = status.running;
            self.status = status.message;
        }
        if let Ok(Some(exit)) = child.try_wait() {
            if self.running || self.status.starts_with("Applying") {
                self.status = format!(
                    "Desktop player exited ({exit}). Try applying again; check the selected image and display."
                );
            }
            self.running = false;
            self.child = None;
        }
    }
    pub fn active(&self) -> bool {
        self.child.is_some()
    }
    pub fn stop(&mut self) {
        if let Some(mut child) = self.child.take() {
            let _ = child.kill();
            let _ = child.wait();
        }
        self.running = false;
        self.session = None;
        self.status = "Wallpaper stopped · your Windows background is visible again".into();
    }
    pub fn toggle_pause(&mut self) {
        if let Some(mut session) = self.session.clone() {
            session.paused = !session.paused;
            if let Err(error) = self.update(session) {
                self.status = error;
            }
        }
    }
}
impl Drop for Runtime {
    fn drop(&mut self) {
        self.stop();
    }
}
