use crate::{
    platform::{self, Tray, TrayAction},
    preview,
};
use crate::{preferences::Preferences, runtime::Runtime};
use iced::{
    Element, Subscription, Task, keyboard,
    time::{self, Duration, Instant},
    widget::text_editor,
    window,
};
use serein_audio::{AudioFrame, AudioService, AudioSource, OutputDevice, output_devices};
use serein_domain::{AudioSettings, DisplayInfo, FilterKind, Project};
use serein_domain::{DisplayMode, ImageLayer, PlayerSession, data_directory};
use std::{path::PathBuf, sync::Arc};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Page {
    Library,
    Customize,
    Studio,
    Audio,
    Displays,
    Performance,
    Roadmap,
}
impl Page {
    pub const ALL: [Self; 7] = [
        Self::Library,
        Self::Customize,
        Self::Audio,
        Self::Displays,
        Self::Performance,
        Self::Studio,
        Self::Roadmap,
    ];
    pub fn label(self) -> &'static str {
        match self {
            Self::Library => "Library",
            Self::Customize => "Customize",
            Self::Studio => "Advanced",
            Self::Audio => "Audio lab",
            Self::Displays => "Displays",
            Self::Performance => "Performance",
            Self::Roadmap => "About",
        }
    }
}

#[derive(Debug, Clone)]
pub enum Pending {
    Open,
    Quit,
}

#[derive(Debug, Clone)]
pub enum Message {
    Navigate(Page),
    Tick(Instant),
    Resize(iced::Size),
    Close,
    Hide,
    Quit,
    TogglePreview,
    SetSource(AudioSource),
    Pointer(bool),
    Post(bool),
    SelectBand(usize),
    DragBand(usize, f32, f32),
    Frequency(f32),
    Gain(f32),
    Q(f32),
    Filter(FilterKind),
    Enabled(bool),
    Bypass(bool),
    ResetEq,
    UndoEq,
    EqPreset(usize),
    BusName(String),
    BusLow(f32),
    BusHigh(f32),
    BusGain(f32),
    Attack(f32),
    Release(f32),
    BusPreset(usize),
    ShaderEdit(text_editor::Action),
    Compile,
    Name(String),
    Fps(u32),
    Save,
    Saved(u64, Result<Option<PathBuf>, String>),
    Open,
    Opened(u64, Result<Option<(PathBuf, Box<Project>)>, String>),
    Starter(usize),
    LocalWallpaper(usize),
    Discard,
    Cancel,
    RefreshDisplays,
    CommandPalette,
    CommandText(String),
    ExecuteCommand,
    UndoCode,
    RedoCode,
    OutputSelected(OutputDevice),
    RefreshOutputs,
    ApplyWallpaper,
    StopWallpaper,
    PauseWallpaper,
    Appearance(usize, f32),
    Palette(u32),
    DisplayMode(DisplayMode),
    Monitor(String),
    ImportImage(bool),
    ImageImported(u64, bool, Result<Option<PathBuf>, String>),
    NativeWindow(u64),
    OpenGuide,
    PlaybackRule(usize, bool),
}

pub struct Studio {
    pub page: Page,
    pub project: Project,
    pub editor: text_editor::Content,
    pub compiled: Arc<str>,
    pub diagnostics: String,
    pub compile_ok: bool,
    pub path: Option<PathBuf>,
    pub revision: u64,
    pub dirty: bool,
    pub file_busy: bool,
    pub status: String,
    pub selected_band: usize,
    pub post: bool,
    pub history: Vec<AudioSettings>,
    pub audio: AudioService,
    pub audio_frame: AudioFrame,
    pub audio_status: String,
    pub analyzer_running: bool,
    pub source: AudioSource,
    pub preview_running: bool,
    pub preview_time: f32,
    pub pointer: bool,
    pub window_size: iced::Size,
    pub displays: Vec<DisplayInfo>,
    pub pending: Option<Pending>,
    pub tray: Option<Tray>,
    pub hidden: bool,
    pub command_open: bool,
    pub command: String,
    pub code_undo: Vec<String>,
    pub code_redo: Vec<String>,
    pub thumbnails: Vec<iced::widget::image::Handle>,
    pub local_wallpapers: Vec<crate::local_library::LocalWallpaper>,
    pub selected_local: Option<String>,
    pub devices: Vec<OutputDevice>,
    pub selected_output: OutputDevice,
    pub preferences: Preferences,
    pub runtime: Runtime,
    pub selected_preset: Option<usize>,
    pub document_id: u64,
    native_window: u64,
    last_devices: Instant,
    last_autosave: Instant,
    autosaved_revision: u64,
    last_tick: Instant,
}

impl Studio {
    pub fn new() -> Self {
        let project =
            Project::load(&data_directory().join("draft.serein.json")).unwrap_or_default();
        let preferences = Preferences::load();
        let devices = output_devices().unwrap_or_else(|_| vec![OutputDevice::default()]);
        let selected_output = devices
            .iter()
            .find(|device| device.id == preferences.output_id)
            .cloned()
            .unwrap_or_else(|| OutputDevice {
                id: preferences.output_id.clone(),
                name: "Selected output unavailable (reconnecting)".into(),
            });
        let local_wallpapers = crate::local_library::load();
        let selected_local = local_wallpapers
            .iter()
            .find(|wallpaper| wallpaper.project.image == project.image && project.image.is_some())
            .map(|wallpaper| wallpaper.id.clone());
        let selected_preset = preferences
            .selected_preset
            .filter(|index| {
                *index < serein_render::catalog::PRESETS.len() && project.image.is_none()
            })
            .or_else(|| {
                serein_render::catalog::PRESETS
                    .iter()
                    .position(|preset| preset.name == project.name && project.image.is_none())
            });
        let compiled = preview::compile(&project.shader).unwrap_or_else(|_| {
            preview::compile(serein_domain::DEFAULT_SHADER).expect("bundled shader is tested")
        });
        let tray_result = Tray::new();
        let status = tray_result
            .as_ref()
            .err()
            .map(|e| format!("Tray unavailable: {e}. Closing the window will quit the studio."))
            .unwrap_or_else(|| {
                "Choose a wallpaper, customize it, and apply it to your desktop".into()
            });
        let mut audio = AudioService::new();
        audio.configure(project.audio.clone());
        audio.set_device(preferences.output_id.clone());
        let _ = audio.start(AudioSource::SystemOutput);
        Self {
            page: Page::Library,
            editor: text_editor::Content::with_text(&project.shader),
            project,
            compiled,
            diagnostics: "Shader ready. Edit in Studio, then compile to update the viewport."
                .into(),
            compile_ok: true,
            path: None,
            revision: 0,
            dirty: false,
            file_busy: false,
            status,
            selected_band: 0,
            post: true,
            history: Vec::new(),
            audio,
            audio_frame: AudioFrame::default(),
            audio_status: "Generated test signal selected · analyzer stopped".into(),
            analyzer_running: true,
            source: AudioSource::SystemOutput,
            preview_running: true,
            preview_time: 0.,
            pointer: true,
            window_size: iced::Size::new(1280., 780.),
            displays: platform::displays(),
            pending: None,
            tray: tray_result.ok(),
            hidden: false,
            command_open: false,
            command: String::new(),
            code_undo: Vec::new(),
            code_redo: Vec::new(),
            thumbnails: crate::library::thumbnails(),
            local_wallpapers,
            selected_local,
            devices,
            selected_output,
            preferences,
            runtime: Runtime::default(),
            selected_preset,
            document_id: 1,
            native_window: 0,
            last_devices: Instant::now(),
            last_autosave: Instant::now(),
            autosaved_revision: 0,
            last_tick: Instant::now(),
        }
    }
    pub fn boot() -> (Self, Task<Message>) {
        (
            Self::new(),
            window::oldest()
                .and_then(window::raw_id::<Message>)
                .map(Message::NativeWindow),
        )
    }
    pub fn player_session(&self) -> PlayerSession {
        PlayerSession {
            token: self.document_id,
            project: self.project.clone(),
            device_id: self.preferences.output_id.clone(),
            display_mode: self.preferences.display_mode,
            monitor_name: self.preferences.monitor_name.clone(),
            paused: self
                .runtime
                .session
                .as_ref()
                .is_some_and(|session| session.paused),
            pause_fullscreen: self.preferences.pause_fullscreen,
            pause_battery: self.preferences.pause_battery,
        }
    }
    fn synchronize_wallpaper(&mut self) -> Result<(), String> {
        if self.runtime.document_id != self.document_id {
            return Ok(());
        }
        if let Some(mut session) = self.runtime.session.clone() {
            // Display choices are a pending Apply action; only content controls
            // update the currently applied wallpaper as they are adjusted.
            session.project = self.project.clone();
            self.runtime.update(session)?;
        }
        Ok(())
    }
    fn save_locally(&mut self) {
        let mut project = self.project.clone();
        project.shader = self.editor.text();
        let directory = data_directory();
        if let Err(error) = std::fs::create_dir_all(&directory)
            .map_err(|e| e.to_string())
            .and_then(|_| {
                project
                    .save(&directory.join("draft.serein.json"))
                    .map_err(|e| e.to_string())
            })
        {
            self.status = error;
            return;
        }
        if let Some(index) = self.selected_preset {
            let profiles = directory.join("profiles");
            if std::fs::create_dir_all(&profiles).is_ok() {
                let path = profiles.join(format!(
                    "{}.json",
                    serein_render::catalog::PRESETS[index].id
                ));
                if let Err(error) = project.save(&path) {
                    self.status = error.to_string();
                    return;
                }
            }
        }
        if let Some(id) = &self.selected_local {
            let directory = data_directory().join("library");
            if std::fs::create_dir_all(&directory).is_ok() {
                if let Err(error) = project.save(&directory.join(format!("{id}.json"))) {
                    self.status = error.to_string();
                    return;
                }
                if let Some(local) = self
                    .local_wallpapers
                    .iter_mut()
                    .find(|local| &local.id == id)
                {
                    local.project = project.clone();
                }
            }
        }
        self.preferences.selected_preset = self.selected_preset;
        self.preferences.selected_local = self.selected_local.clone();
        if let Err(error) = self.preferences.save() {
            self.status = error;
        }
        self.autosaved_revision = self.revision;
        if self.path.is_none() {
            self.dirty = false;
        }
        self.status = "Settings saved locally".into();
    }
    fn start_listening(&mut self) {
        self.audio.set_device(self.preferences.output_id.clone());
        match self.audio.start(self.source) {
            Ok(()) => self.analyzer_running = true,
            Err(error) => self.status = error,
        }
    }
    pub fn title(&self) -> String {
        format!(
            "{}{} — Serein Studio",
            self.project.name,
            if self.dirty { " *" } else { "" }
        )
    }
    pub fn view(&self) -> Element<'_, Message> {
        crate::views::shell(self)
    }
    pub fn subscription(&self) -> Subscription<Message> {
        let active = !self.hidden && (self.preview_running || self.analyzer_running);
        let rate = if active {
            1000 / self.project.preview_fps.max(1) as u64
        } else {
            250
        };
        Subscription::batch([
            time::every(Duration::from_millis(rate.max(8))).map(Message::Tick),
            window::resize_events().map(|(_, size)| Message::Resize(size)),
            window::close_requests().map(|_| Message::Close),
            keyboard::listen().filter_map(|event| match event {
                keyboard::Event::KeyPressed {
                    key: keyboard::Key::Character(c),
                    modifiers,
                    ..
                } if modifiers.control() && c.eq_ignore_ascii_case("s") => Some(Message::Save),
                keyboard::Event::KeyPressed {
                    key: keyboard::Key::Character(c),
                    modifiers,
                    ..
                } if modifiers.control() && c.eq_ignore_ascii_case("k") => {
                    Some(Message::CommandPalette)
                }
                keyboard::Event::KeyPressed {
                    key: keyboard::Key::Named(keyboard::key::Named::Enter),
                    modifiers,
                    ..
                } if modifiers.control() => Some(Message::Compile),
                keyboard::Event::KeyPressed {
                    key: keyboard::Key::Named(keyboard::key::Named::Escape),
                    ..
                } => Some(Message::Cancel),
                _ => None,
            }),
        ])
    }
    fn changed(&mut self) {
        self.revision += 1;
        self.dirty = true;
    }
    fn remember_audio(&mut self) {
        if self.history.last() != Some(&self.project.audio) {
            if self.history.len() == 80 {
                self.history.remove(0);
            }
            self.history.push(self.project.audio.clone());
        }
    }
    fn configure_audio(&mut self) {
        self.audio.configure(self.project.audio.clone());
        self.changed();
    }
    fn request(&mut self, pending: Pending) -> Task<Message> {
        if self.file_busy {
            self.status = "Wait for the current file operation to finish.".into();
            return Task::none();
        }
        if self.dirty {
            self.pending = Some(pending);
            Task::none()
        } else {
            self.execute(pending)
        }
    }
    fn execute(&mut self, pending: Pending) -> Task<Message> {
        match pending {
            Pending::Quit => {
                self.save_locally();
                self.runtime.stop();
                self.audio.stop();
                iced::exit()
            }
            Pending::Open => {
                self.file_busy = true;
                let revision = self.revision;
                Task::perform(
                    async {
                        let Some(file) = rfd::AsyncFileDialog::new()
                            .add_filter("Serein project", &["json"])
                            .pick_file()
                            .await
                        else {
                            return Ok(None);
                        };
                        let path = file.path().to_path_buf();
                        let project = Project::load(&path).map_err(|e| e.to_string())?;
                        preview::compile(&project.shader).map_err(|e| {
                            format!(
                                "Project shader could not compile. Fix it before importing:\n{e}"
                            )
                        })?;
                        Ok(Some((path, Box::new(project))))
                    },
                    move |result| Message::Opened(revision, result),
                )
            }
        }
    }
    fn replace(&mut self, project: Project, path: Option<PathBuf>) {
        if let Ok(compiled) = preview::compile(&project.shader) {
            self.compiled = compiled;
        }
        self.editor = text_editor::Content::with_text(&project.shader);
        self.project = project;
        self.path = path;
        self.document_id += 1;
        self.history.clear();
        self.code_undo.clear();
        self.code_redo.clear();
        self.audio.configure(self.project.audio.clone());
        self.dirty = false;
        self.revision += 1;
        self.preview_time = 0.;
        self.preview_running = true;
        self.compile_ok = true;
        self.diagnostics = "Project shader compiled. Preview updated.".into();
        self.status = "Wallpaper selected".into();
    }
    pub fn update(&mut self, message: Message) -> Task<Message> {
        match message {
            Message::Navigate(page) => {
                self.page = page;
            }

            Message::Resize(size) => self.window_size = size,
            Message::Tick(now) => {
                if !self.hidden
                    && self.native_window != 0
                    && serein_platform::is_minimized(self.native_window as isize)
                {
                    return self.update(Message::Hide);
                }
                self.runtime.poll();
                if self.last_autosave.elapsed() > Duration::from_millis(600) {
                    self.last_autosave = now;
                    if self.revision != self.autosaved_revision {
                        self.save_locally();
                    }
                    if self.runtime.active()
                        && self.runtime.document_id == self.document_id
                        && let Err(error) = self.synchronize_wallpaper()
                    {
                        self.status = error;
                    }
                }
                if !self.hidden && self.last_devices.elapsed() > Duration::from_secs(5) {
                    self.last_devices = now;
                    if let Ok(devices) = output_devices() {
                        self.devices = devices;
                    }
                }

                let delta = (now - self.last_tick).as_secs_f32().min(1.0);
                self.last_tick = now;
                if !self.hidden && self.preview_running {
                    self.preview_time += delta;
                }
                if self.analyzer_running {
                    let snapshot = self.audio.snapshot();
                    self.audio_frame = snapshot.frame;
                    self.audio_status = snapshot.status;
                    self.analyzer_running = snapshot.running;
                    if snapshot.updated.elapsed() > Duration::from_millis(500) {
                        self.audio_frame = AudioFrame::default();
                    }
                }
                if let Some(action) = self.tray.as_ref().and_then(Tray::poll) {
                    match action {
                        TrayAction::Pause => {
                            self.runtime.toggle_pause();
                        }
                        TrayAction::Stop => {
                            self.runtime.stop();
                        }
                        TrayAction::Quit => {
                            self.hidden = false;
                            let show = window::oldest()
                                .and_then(|id| window::set_mode(id, window::Mode::Windowed));
                            return Task::batch([show, self.request(Pending::Quit)]);
                        }
                        TrayAction::Show => {
                            self.hidden = false;
                            self.preview_running = true;
                            self.start_listening();
                            return window::oldest().and_then(|id| {
                                Task::batch([
                                    window::set_mode(id, window::Mode::Windowed),
                                    window::gain_focus(id),
                                ])
                            });
                        }
                    }
                }
            }
            Message::Close | Message::Hide if self.tray.is_some() => {
                self.save_locally();
                self.hidden = true;
                self.preview_running = false;
                self.audio.stop();
                self.analyzer_running = false;
                self.audio_frame = AudioFrame::default();
                self.audio_status = "Desktop wallpaper continues in the background".into();
                return window::oldest().and_then(|id| window::set_mode(id, window::Mode::Hidden));
            }
            Message::Close | Message::Quit | Message::Hide => return self.request(Pending::Quit),
            Message::TogglePreview => self.preview_running = !self.preview_running,
            Message::SetSource(source) => {
                self.source = source;
                self.start_listening();
            }
            Message::Pointer(enabled) => self.pointer = enabled,
            Message::Post(post) => self.post = post,
            Message::SelectBand(index) => self.selected_band = index.min(6),
            Message::DragBand(index, hz, db) => {
                self.remember_audio();
                self.selected_band = index.min(6);
                self.project.audio.bands[self.selected_band].frequency_hz = hz;
                if self.project.audio.bands[self.selected_band]
                    .kind
                    .uses_gain()
                {
                    self.project.audio.bands[self.selected_band].gain_db = db;
                }
                self.configure_audio();
            }
            Message::Frequency(log_hz) => {
                self.remember_audio();
                self.project.audio.bands[self.selected_band].frequency_hz =
                    10_f32.powf(log_hz).clamp(20., 20000.);
                self.configure_audio();
            }
            Message::Gain(db) => {
                self.remember_audio();
                self.project.audio.bands[self.selected_band].gain_db = db;
                self.configure_audio();
            }
            Message::Q(q) => {
                self.remember_audio();
                self.project.audio.bands[self.selected_band].q = q;
                self.configure_audio();
            }
            Message::Filter(kind) => {
                self.remember_audio();
                self.project.audio.bands[self.selected_band].kind = kind;
                if !kind.uses_gain() {
                    self.project.audio.bands[self.selected_band].gain_db = 0.;
                }
                self.configure_audio();
            }
            Message::Enabled(enabled) => {
                self.remember_audio();
                self.project.audio.bands[self.selected_band].enabled = enabled;
                self.configure_audio();
            }
            Message::Bypass(bypass) => {
                self.remember_audio();
                self.project.audio.bypass = bypass;
                self.configure_audio();
            }
            Message::ResetEq => {
                self.remember_audio();
                self.project.audio.bands = AudioSettings::default().bands;
                self.project.audio.bypass = false;
                self.configure_audio();
            }
            Message::UndoEq => {
                if let Some(settings) = self.history.pop() {
                    self.project.audio = settings;
                    self.configure_audio();
                }
            }
            Message::EqPreset(index) => {
                self.remember_audio();
                self.project.audio.bands = AudioSettings::default().bands;
                self.project.audio.bypass = false;
                match index {
                    1 => {
                        self.project.audio.bands[0].gain_db = 8.;
                        self.project.audio.bands[1].gain_db = 4.;
                        self.project.audio.bands[4].gain_db = -5.;
                    }
                    2 => {
                        self.project.audio.bands[0].kind = FilterKind::HighPass;
                        self.project.audio.bands[0].frequency_hz = 300.;
                        self.project.audio.bands[6].kind = FilterKind::LowPass;
                        self.project.audio.bands[6].frequency_hz = 3400.;
                    }
                    _ => {}
                }
                self.configure_audio();
            }
            Message::BusName(name) => {
                self.project.audio.bus.name = name.chars().take(64).collect();
                self.configure_audio();
            }
            Message::BusLow(log_hz) => {
                self.project.audio.bus.low_hz = 10_f32
                    .powf(log_hz)
                    .clamp(20., 19999.)
                    .min(self.project.audio.bus.high_hz - 1.);
                self.configure_audio();
            }
            Message::BusHigh(log_hz) => {
                self.project.audio.bus.high_hz = 10_f32
                    .powf(log_hz)
                    .clamp(21., 20000.)
                    .max(self.project.audio.bus.low_hz + 1.);
                self.configure_audio();
            }
            Message::BusGain(gain) => {
                self.project.audio.bus.gain = gain;
                self.configure_audio();
            }
            Message::Attack(ms) => {
                self.project.audio.bus.attack_ms = ms;
                self.configure_audio();
            }
            Message::Release(ms) => {
                self.project.audio.bus.release_ms = ms;
                self.configure_audio();
            }
            Message::BusPreset(index) => {
                let (name, low, high) = match index {
                    1 => ("Mid motion", 250., 2500.),
                    2 => ("Air shimmer", 4000., 16000.),
                    _ => ("Bass pulse", 30., 220.),
                };
                self.project.audio.bus.name = name.into();
                self.project.audio.bus.low_hz = low;
                self.project.audio.bus.high_hz = high;
                self.configure_audio();
            }
            Message::ShaderEdit(action) => {
                if action.is_edit() {
                    push_code_history(&mut self.code_undo, self.editor.text());
                    self.code_redo.clear();
                    self.changed();
                    self.compile_ok = false;
                }
                self.editor.perform(action);
            }
            Message::UndoCode => {
                if let Some(source) = self.code_undo.pop() {
                    push_code_history(&mut self.code_redo, self.editor.text());
                    self.editor = text_editor::Content::with_text(&source);
                    self.changed();
                    self.compile_ok = false;
                }
            }
            Message::RedoCode => {
                if let Some(source) = self.code_redo.pop() {
                    push_code_history(&mut self.code_undo, self.editor.text());
                    self.editor = text_editor::Content::with_text(&source);
                    self.changed();
                    self.compile_ok = false;
                }
            }
            Message::Compile => match preview::compile(&self.editor.text()) {
                Ok(source) => {
                    self.compiled = source;
                    self.project.shader = self.editor.text();
                    self.compile_ok = true;
                    self.diagnostics = "Compiled · viewport updated with the new shader".into();
                }
                Err(error) => {
                    self.compile_ok = false;
                    self.diagnostics = format!(
                        "Compile failed · preview retains the last good shader\n{error}\nFix the highlighted WGSL error and compile again."
                    );
                }
            },
            Message::Name(name) => {
                self.project.name = name.chars().take(120).collect();
                self.changed();
            }
            Message::Fps(fps) => {
                self.project.preview_fps = fps;
                self.changed();
            }
            Message::Save => {
                if self.file_busy {
                    return Task::none();
                }
                let mut project = self.project.clone();
                project.shader = self.editor.text();
                if let Err(error) = project.validate() {
                    self.status = error.to_string();
                    return Task::none();
                }
                let existing = self.path.clone();
                let revision = self.revision;
                self.file_busy = true;
                return Task::perform(
                    async move {
                        let path = if let Some(path) = existing {
                            path
                        } else {
                            let Some(file) = rfd::AsyncFileDialog::new()
                                .set_file_name("wallpaper.serein.json")
                                .add_filter("Serein project", &["json"])
                                .save_file()
                                .await
                            else {
                                return Ok(None);
                            };
                            file.path().to_path_buf()
                        };
                        project.save(&path).map_err(|e| e.to_string())?;
                        Ok(Some(path))
                    },
                    move |result| Message::Saved(revision, result),
                );
            }
            Message::Saved(revision, result) => {
                self.file_busy = false;
                match result {
                    Ok(Some(path)) => {
                        self.status = format!("Saved {}", path.display());
                        self.path = Some(path);
                        if self.revision == revision {
                            self.dirty = false;
                        }
                    }
                    Ok(None) => self.status = "Save canceled · changes remain in the studio".into(),
                    Err(error) => self.status = error,
                }
            }
            Message::Open => return self.request(Pending::Open),
            Message::Opened(revision, result) => {
                self.file_busy = false;
                match result { Ok(Some((path, project))) if revision == self.revision => { self.replace(*project, Some(path)); self.selected_preset = None; self.selected_local = None; }, Ok(Some(_)) => self.status = "Project was edited during loading. Save your work, then open the file again.".into(), Ok(None) => self.status = "Open canceled".into(), Err(error) => self.status = error }
            }
            Message::Starter(index) => {
                self.save_locally();
                let index = index.min(serein_render::catalog::PRESETS.len() - 1);
                let (name, shader) = starter(index);
                let profile = data_directory().join("profiles").join(format!(
                    "{}.json",
                    serein_render::catalog::PRESETS[index].id
                ));
                let project = Project::load(&profile).unwrap_or_else(|_| Project {
                    name: name.into(),
                    shader: shader.into(),
                    ..Project::default()
                });
                self.replace(project, None);
                self.selected_preset = Some(index);
                self.selected_local = None;
                self.page = Page::Library;
                self.save_locally();
            }
            Message::LocalWallpaper(index) => {
                self.save_locally();
                if let Some(wallpaper) = self.local_wallpapers.get(index) {
                    let project = wallpaper.project.clone();
                    let id = wallpaper.id.clone();
                    self.replace(project, None);
                    self.selected_preset = None;
                    self.selected_local = Some(id);
                    self.page = Page::Library;
                    self.save_locally();
                }
            }
            Message::Discard => {
                if let Some(pending) = self.pending.take() {
                    return self.execute(pending);
                }
            }
            Message::Cancel => {
                self.pending = None;
                self.command_open = false;
            }
            Message::RefreshDisplays => {
                self.displays = platform::displays();
                self.status = format!("Detected {} physical display(s)", self.displays.len());
            }
            Message::PlaybackRule(index, enabled) => {
                if index == 0 {
                    self.preferences.pause_fullscreen = enabled;
                } else {
                    self.preferences.pause_battery = enabled;
                }
                if let Err(error) = self.preferences.save() {
                    self.status = error;
                }
                if let Some(mut session) = self.runtime.session.clone() {
                    session.pause_fullscreen = self.preferences.pause_fullscreen;
                    session.pause_battery = self.preferences.pause_battery;
                    if let Err(error) = self.runtime.update(session) {
                        self.status = error;
                    }
                }
            }
            Message::NativeWindow(id) => self.native_window = id,
            Message::OpenGuide => {
                let directory = data_directory().join("help");
                let path = directory.join("Creator-guide.md");
                let result = std::fs::create_dir_all(&directory)
                    .and_then(|_| {
                        std::fs::write(&path, include_str!("../../../docs/07-creator-guide.md"))
                    })
                    .and_then(|_| {
                        let mut command = std::process::Command::new("explorer.exe");
                        command.arg(&path);
                        #[cfg(windows)]
                        {
                            use std::os::windows::process::CommandExt;
                            command.creation_flags(0x0800_0000);
                        }
                        command.spawn().map(|_| ())
                    });
                if let Err(error) = result {
                    self.status = format!(
                        "Unable to open the guide: {error}. Read docs/07-creator-guide.md in the project folder."
                    );
                }
            }
            Message::OutputSelected(device) => {
                self.preferences.output_id = device.id.clone();
                self.selected_output = device;
                self.start_listening();
                if let Some(mut session) = self.runtime.session.clone() {
                    session.device_id = self.preferences.output_id.clone();
                    if let Err(error) = self.runtime.update(session) {
                        self.status = error;
                    }
                }
                if let Err(error) = self.preferences.save() {
                    self.status = error;
                }
            }
            Message::RefreshOutputs => match output_devices() {
                Ok(devices) => self.devices = devices,
                Err(error) => self.status = error,
            },
            Message::ApplyWallpaper => {
                self.save_locally();
                let session = self.player_session();
                if let Err(error) = self.runtime.apply(session, self.document_id) {
                    self.status = error;
                }
            }
            Message::StopWallpaper => self.runtime.stop(),
            Message::PauseWallpaper => self.runtime.toggle_pause(),
            Message::Appearance(index, value) => {
                match index {
                    0 => self.project.appearance.speed = value,
                    1 => self.project.appearance.brightness = value,
                    2 => self.project.appearance.detail = value,
                    3 => self.project.appearance.audio_amount = value,
                    _ => self.project.appearance.parallax = value,
                }
                self.changed();
            }
            Message::Palette(palette) => {
                self.project.appearance.palette = palette;
                self.changed();
            }
            Message::DisplayMode(mode) => {
                self.preferences.display_mode = mode;
                if self.preferences.monitor_name.is_none() {
                    self.preferences.monitor_name =
                        self.displays.first().map(|display| display.name.clone());
                }
                if let Err(error) = self.preferences.save() {
                    self.status = error;
                }
            }
            Message::Monitor(name) => {
                self.preferences.monitor_name = Some(name);
                if let Err(error) = self.preferences.save() {
                    self.status = error;
                }
            }
            Message::ImportImage(depth) => {
                if self.file_busy {
                    return Task::none();
                }
                self.file_busy = true;
                let revision = self.revision;
                return Task::perform(
                    async move {
                        let Some(file) = rfd::AsyncFileDialog::new()
                            .add_filter("Images", &["png", "jpg", "jpeg", "webp"])
                            .pick_file()
                            .await
                        else {
                            return Ok(None);
                        };
                        let decoded = serein_render::load_image(file.path())?;
                        let directory = data_directory().join("images");
                        std::fs::create_dir_all(&directory).map_err(|e| e.to_string())?;
                        let stamp = std::time::SystemTime::now()
                            .duration_since(std::time::UNIX_EPOCH)
                            .map_err(|e| e.to_string())?
                            .as_nanos();
                        let extension = file
                            .path()
                            .extension()
                            .and_then(|s| s.to_str())
                            .unwrap_or("png");
                        let label: String = file
                            .path()
                            .file_stem()
                            .map(|stem| {
                                stem.to_string_lossy()
                                    .chars()
                                    .filter(|c| c.is_alphanumeric() || *c == '-' || *c == '_')
                                    .take(64)
                                    .collect()
                            })
                            .unwrap_or_else(|| "Image".into());
                        let path = directory.join(format!("{stamp}-{label}.{extension}"));
                        std::fs::copy(file.path(), &path).map_err(|e| e.to_string())?;
                        if !depth {
                            serein_render::save_image_thumbnail(
                                &decoded,
                                &path.with_extension("thumb.png"),
                            )?;
                        }
                        Ok(Some(path))
                    },
                    move |result| Message::ImageImported(revision, depth, result),
                );
            }
            Message::ImageImported(revision, depth, result) => {
                self.file_busy = false;
                match result {
                    Ok(Some(path)) if revision == self.revision => {
                        if depth {
                            if let Some(image) = &mut self.project.image {
                                image.depth_path = Some(path);
                                self.changed();
                            }
                        } else {
                            self.save_locally();
                            let project = Project {
                                name: path
                                    .file_stem()
                                    .and_then(|stem| stem.to_str())
                                    .and_then(|stem| stem.split_once('-'))
                                    .map(|(_, name)| name.replace('_', " "))
                                    .filter(|name| !name.is_empty())
                                    .unwrap_or_else(|| "My image parallax".into()),
                                shader: serein_render::catalog::IMAGE_SHADER.into(),
                                image: Some(ImageLayer {
                                    path,
                                    depth_path: None,
                                }),
                                ..Project::default()
                            };
                            let id = project
                                .image
                                .as_ref()
                                .and_then(|image| image.path.file_stem())
                                .map(|stem| stem.to_string_lossy().into_owned())
                                .unwrap_or_else(|| format!("photo-{}", self.document_id));
                            let thumbnail = iced::widget::image::Handle::from_path(
                                project
                                    .image
                                    .as_ref()
                                    .expect("new image project")
                                    .path
                                    .with_extension("thumb.png"),
                            );
                            self.replace(project, None);
                            self.selected_preset = None;
                            self.selected_local = Some(id.clone());
                            self.local_wallpapers.insert(
                                0,
                                crate::local_library::LocalWallpaper {
                                    id,
                                    project: self.project.clone(),
                                    thumbnail,
                                },
                            );
                            self.page = Page::Library;
                            self.changed();
                        }
                    }
                    Ok(Some(_)) => {
                        self.status =
                            "Project changed during import. Import the image again when ready."
                                .into()
                    }
                    Ok(None) => self.status = "Image selection canceled".into(),
                    Err(error) => self.status = error,
                }
            }
            Message::CommandPalette => {
                self.command_open = true;
                return iced::widget::operation::focus("command-input");
            }
            Message::CommandText(command) => self.command = command,
            Message::ExecuteCommand => match command_message(&self.command, self) {
                Ok(message) => {
                    self.command_open = false;
                    self.command.clear();
                    return self.update(message);
                }
                Err(error) => self.status = error,
            },
        }
        Task::none()
    }
}

pub fn starter(index: usize) -> (&'static str, &'static str) {
    let preset =
        &serein_render::catalog::PRESETS[index.min(serein_render::catalog::PRESETS.len() - 1)];
    (preset.name, preset.shader)
}

fn push_code_history(history: &mut Vec<String>, source: String) {
    // Bound editor-only recovery memory; the player will never load this history.
    history.push(source);
    while history.len() > 80 || history.iter().map(String::len).sum::<usize>() > 4 * 1024 * 1024 {
        history.remove(0);
    }
}

fn command_message(input: &str, app: &Studio) -> Result<Message, String> {
    let (verb, argument) = input.trim().split_once(' ').unwrap_or((input.trim(), ""));
    let number = |low: f32, high: f32| -> Result<f32, String> {
        argument
            .trim()
            .parse::<f32>()
            .ok()
            .filter(|v| v.is_finite() && (low..=high).contains(v))
            .ok_or_else(|| format!("For {verb}, enter a value from {low} to {high}."))
    };
    Ok(match verb.to_ascii_lowercase().as_str() {
        "library" => Message::Navigate(Page::Library),
        "customize" => Message::Navigate(Page::Customize),
        "apply" => Message::ApplyWallpaper,
        "stop-wallpaper" => Message::StopWallpaper,
        "pause-wallpaper" => Message::PauseWallpaper,
        "import-image" => Message::ImportImage(false),
        "depth-map" => Message::ImportImage(true),
        "device" => {
            if argument.trim().eq_ignore_ascii_case("default") {
                Message::OutputSelected(OutputDevice::default())
            } else {
                Message::OutputSelected(
                    app.devices[number(1., app.devices.len() as f32)? as usize - 1].clone(),
                )
            }
        }
        "speed" => Message::Appearance(0, number(0., 4.)?),
        "brightness" => Message::Appearance(1, number(0.1, 2.)?),
        "zoom" => Message::Appearance(2, number(0.5, 3.)?),
        "reaction" => Message::Appearance(3, number(0., 5.)?),
        "depth" => Message::Appearance(4, number(0., 0.12)?),
        "studio" => Message::Navigate(Page::Studio),
        "audio" => Message::Navigate(Page::Audio),
        "displays" => Message::Navigate(Page::Displays),
        "performance" => Message::Navigate(Page::Performance),
        "roadmap" => Message::Navigate(Page::Roadmap),
        "save" => Message::Save,
        "open" => Message::Open,
        "compile" => Message::Compile,
        "help" => Message::OpenGuide,
        "undo-code" => Message::UndoCode,
        "redo-code" => Message::RedoCode,
        "preview" => Message::TogglePreview,
        "analyzer" => Message::Navigate(Page::Audio),
        "tray" => Message::Hide,
        "quit" => Message::Quit,
        "reset" => Message::ResetEq,
        "undo" => Message::UndoEq,
        "refresh" => Message::RefreshDisplays,
        "band" => Message::SelectBand(number(1., 7.)? as usize - 1),
        "frequency" => Message::Frequency(number(20., 20000.)?.log10()),
        "gain" => Message::Gain(number(-24., 24.)?),
        "q" => Message::Q(number(0.1, 18.)?),
        "filter" => Message::Filter(match argument.to_ascii_lowercase().as_str() {
            "bell" => FilterKind::Peak,
            "lowpass" => FilterKind::LowPass,
            "highpass" => FilterKind::HighPass,
            "lowshelf" => FilterKind::LowShelf,
            "highshelf" => FilterKind::HighShelf,
            "notch" => FilterKind::Notch,
            _ => {
                return Err(
                    "Choose filter bell, lowpass, highpass, lowshelf, highshelf or notch.".into(),
                );
            }
        }),
        "enabled" => Message::Enabled(!app.project.audio.bands[app.selected_band].enabled),
        "bypass" => Message::Bypass(!app.project.audio.bypass),
        "low" => Message::BusLow(number(20., app.project.audio.bus.high_hz - 1.)?.log10()),
        "high" => Message::BusHigh(number(app.project.audio.bus.low_hz + 1., 20000.)?.log10()),
        "sensitivity" => Message::BusGain(number(0., 20.)?),
        "attack" => Message::Attack(number(1., 2000.)?),
        "release" => Message::Release(number(1., 5000.)?),
        "name" if !argument.trim().is_empty() => Message::Name(argument.into()),
        "busname" if !argument.trim().is_empty() => Message::BusName(argument.into()),
        "source" => Message::SetSource(match argument.to_ascii_lowercase().as_str() {
            "test" => AudioSource::TestSignal,
            "system" => AudioSource::SystemOutput,
            _ => return Err("Choose source test or source system.".into()),
        }),
        "pointer" => Message::Pointer(!app.pointer),
        "spectrum" => Message::Post(!app.post),
        "fps" => Message::Fps(number(1., 120.)? as u32),
        "template" => {
            Message::Starter(number(1., serein_render::catalog::PRESETS.len() as f32)? as usize - 1)
        }
        "fullscreen" => Message::PlaybackRule(0, !app.preferences.pause_fullscreen),
        "battery" => Message::PlaybackRule(1, !app.preferences.pause_battery),
        "pause" => Message::PauseWallpaper,
        "discard" => Message::Discard,
        "cancel" => Message::Cancel,
        _ => {
            return Err(
                "Command not recognized. Use one of the commands shown above, then press Enter."
                    .into(),
            );
        }
    })
}

#[cfg(test)]
mod tests {
    #[test]
    fn all_shipped_templates_match_preview_abi() {
        for index in 0..3 {
            let (name, shader) = super::starter(index);
            assert!(crate::preview::compile(shader).is_ok(), "{name}");
        }
    }
}
