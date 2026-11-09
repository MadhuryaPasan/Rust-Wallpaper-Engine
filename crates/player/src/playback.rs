use raw_window_handle::{HasWindowHandle, RawWindowHandle};
use serein_audio::{AudioService, AudioSource};
use serein_domain::{DisplayInfo, DisplayMode, PlayerSession, PlayerStatus, virtual_bounds};
use serein_render::{Material, Uniforms, compile};
use std::{
    path::{Path, PathBuf},
    sync::Arc,
    time::{Duration, Instant},
};
use winit::{
    application::ApplicationHandler,
    dpi::{PhysicalPosition, PhysicalSize},
    event::WindowEvent,
    event_loop::{ActiveEventLoop, ControlFlow, EventLoop},
    window::{Window, WindowId},
};

struct SurfaceWindow {
    ready: bool,
    smoothed_pointer: [f32; 2],
    window: Arc<Window>,
    surface: wgpu::Surface<'static>,
    config: wgpu::SurfaceConfiguration,
    material: Material,
    display: DisplayInfo,
    view: [f32; 4],
}
struct Gpu {
    instance: wgpu::Instance,
    adapter: wgpu::Adapter,
    device: wgpu::Device,
    queue: wgpu::Queue,
}
struct Player {
    path: PathBuf,
    session: PlayerSession,
    gpu: Option<Gpu>,
    windows: Vec<SurfaceWindow>,
    audio: AudioService,
    audio_active: bool,
    session_modified: Option<std::time::SystemTime>,
    last_pointer: Option<(i32, i32)>,
    last_bus: f32,
    last_input: Instant,
    displays: Vec<DisplayInfo>,
    clock: f32,
    last_frame: Instant,
    last_poll: Instant,
    last_topology: Instant,
    last_status: Instant,
    frames: u64,
    paused: bool,
    error: Option<String>,
}

pub fn run(path: &Path) -> Result<(), String> {
    let session = PlayerSession::load(path)?;
    compile(&session.project.shader)?;
    let mut audio = AudioService::new();
    audio.configure(session.project.audio.clone());
    audio.set_device(session.device_id.clone());
    let audio_active = session.project.appearance.audio_amount > 0.;
    if audio_active {
        audio.start(AudioSource::SystemOutput)?;
    }
    let mut player = Player {
        path: path.into(),
        session,
        gpu: None,
        windows: Vec::new(),
        audio,
        audio_active,
        session_modified: std::fs::metadata(path)
            .and_then(|metadata| metadata.modified())
            .ok(),
        last_pointer: None,
        last_bus: 0.,
        last_input: Instant::now(),
        displays: serein_platform::displays(),
        clock: 0.,
        last_frame: Instant::now(),
        last_poll: Instant::now(),
        last_topology: Instant::now(),
        last_status: Instant::now() - Duration::from_secs(2),
        frames: 0,
        paused: false,
        error: None,
    };
    EventLoop::new()
        .map_err(|e| e.to_string())?
        .run_app(&mut player)
        .map_err(|e| e.to_string())?;
    if let Some(error) = player.error.take() {
        return Err(error);
    }
    Ok(())
}

impl Player {
    fn status(&self, message: String) {
        let status = PlayerStatus {
            token: self.session.token,
            process_id: std::process::id(),
            running: self.error.is_none() && self.frames > 0,
            paused: self.paused,
            frames: self.frames,
            message,
        };
        if let Ok(bytes) = serde_json::to_vec(&status) {
            let path = self.path.with_extension("status.json");
            let temp = path.with_extension("pending");
            if std::fs::write(&temp, bytes).is_ok() {
                let _ = std::fs::rename(temp, path);
            }
        }
    }
    fn targets(&self) -> Vec<DisplayInfo> {
        match self.session.display_mode {
            DisplayMode::EveryDisplay | DisplayMode::Span => self.displays.clone(),
            DisplayMode::Primary => self
                .displays
                .iter()
                .find(|d| d.primary)
                .cloned()
                .into_iter()
                .collect(),
            DisplayMode::Selected => self
                .displays
                .iter()
                .find(|d| Some(&d.name) == self.session.monitor_name.as_ref())
                .cloned()
                .into_iter()
                .collect(),
        }
    }
    fn rebuild(&mut self, event_loop: &ActiveEventLoop) -> Result<(), String> {
        self.windows.clear();
        self.frames = 0;
        let targets = self.targets();
        if targets.is_empty() {
            return Err(
                "The selected monitor is unavailable. Choose a connected display and apply again."
                    .into(),
            );
        }
        let source = compile(&self.session.project.shader)?;
        let bounds = virtual_bounds(&targets).ok_or("No display bounds")?;
        for display in targets {
            let mut attributes = Window::default_attributes()
                .with_title("Serein desktop wallpaper")
                .with_decorations(false)
                .with_resizable(false)
                .with_active(false)
                .with_visible(false)
                .with_inner_size(PhysicalSize::new(display.width, display.height))
                .with_position(PhysicalPosition::new(display.x, display.y));
            #[cfg(windows)]
            {
                use winit::platform::windows::WindowAttributesExtWindows;
                attributes = attributes.with_skip_taskbar(true);
            }
            let window = Arc::new(
                event_loop
                    .create_window(attributes)
                    .map_err(|e| e.to_string())?,
            );
            let raw = match window.window_handle().map_err(|e| e.to_string())?.as_raw() {
                RawWindowHandle::Win32(handle) => handle.hwnd.get(),
                _ => return Err("Desktop playback currently requires Windows".into()),
            };
            if self.gpu.is_none() {
                // HWND flip swapchains can submit frames that never appear in a
                // layered Explorer child. DirectComposition keeps presentation
                // on the GPU and connects that surface to Windows' visual tree.
                let mut descriptor = wgpu::InstanceDescriptor {
                    backends: wgpu::Backends::DX12,
                    ..Default::default()
                };
                descriptor.backend_options.dx12.presentation_system =
                    wgpu::wgt::Dx12SwapchainKind::DxgiFromVisual;
                let instance = wgpu::Instance::new(&descriptor);
                let surface = instance
                    .create_surface(window.clone())
                    .map_err(|e| e.to_string())?;
                let adapter =
                    pollster::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions {
                        power_preference: wgpu::PowerPreference::LowPower,
                        compatible_surface: Some(&surface),
                        force_fallback_adapter: false,
                    }))
                    .map_err(|e| e.to_string())?;
                let (device, queue) =
                    pollster::block_on(adapter.request_device(&wgpu::DeviceDescriptor::default()))
                        .map_err(|e| e.to_string())?;
                drop(surface);
                self.gpu = Some(Gpu {
                    instance,
                    adapter,
                    device,
                    queue,
                });
            }
            let gpu = self.gpu.as_ref().ok_or("GPU unavailable")?;
            let surface = gpu
                .instance
                .create_surface(window.clone())
                .map_err(|e| e.to_string())?;
            let config = surface
                .get_default_config(&gpu.adapter, display.width, display.height)
                .ok_or("This display cannot present on the selected GPU")?;
            surface.configure(&gpu.device, &config);
            let material = Material::new(
                &gpu.device,
                &gpu.queue,
                config.format,
                &source,
                self.session.project.image.as_ref(),
            )?;
            let view = if self.session.display_mode == DisplayMode::Span {
                [
                    (display.x as i64 - bounds.0) as f32 / bounds.2 as f32,
                    (display.y as i64 - bounds.1) as f32 / bounds.3 as f32,
                    display.width as f32 / bounds.2 as f32,
                    display.height as f32 / bounds.3 as f32,
                ]
            } else {
                [0., 0., 1., 1.]
            };
            window.set_visible(true);
            // winit reapplies its cached native styles when showing the window.
            // Attach last, otherwise that operation removes WS_CHILD/WS_EX_LAYERED.
            serein_platform::attach_wallpaper(raw, &display)?;
            self.windows.push(SurfaceWindow {
                ready: false,
                smoothed_pointer: [0.5, 0.5],
                window,
                surface,
                config,
                material,
                display,
                view,
            });
        }
        Ok(())
    }
    fn refresh(&mut self, event_loop: &ActiveEventLoop) -> Result<(), String> {
        if self.last_poll.elapsed() >= Duration::from_millis(250) {
            self.last_poll = Instant::now();
            let modified = std::fs::metadata(&self.path)
                .and_then(|metadata| metadata.modified())
                .ok();
            if modified != self.session_modified
                && let Ok(session) = PlayerSession::load(&self.path)
                && session != self.session
            {
                if session.project.shader != self.session.project.shader {
                    compile(&session.project.shader)?;
                }
                let rebuild = session.project.shader != self.session.project.shader
                    || session.project.image != self.session.project.image
                    || session.display_mode != self.session.display_mode
                    || session.monitor_name != self.session.monitor_name;
                if session.device_id != self.session.device_id {
                    self.audio.set_device(session.device_id.clone());
                    if self.audio_active {
                        self.audio.start(AudioSource::SystemOutput)?;
                    }
                }
                for surface in &mut self.windows {
                    surface.ready = false;
                }
                self.audio.configure(session.project.audio.clone());
                self.session = session;
                if rebuild {
                    self.rebuild(event_loop)?;
                }
            }
            let paused = self.session.paused
                || serein_platform::should_pause(
                    &self.displays,
                    self.session.pause_fullscreen,
                    self.session.pause_battery,
                );
            if self.paused && !paused {
                self.last_frame = Instant::now();
            }
            self.paused = paused;
            let needs_audio = !paused && self.session.project.appearance.audio_amount > 0.;
            if needs_audio != self.audio_active {
                if needs_audio {
                    self.audio.start(AudioSource::SystemOutput)?;
                } else {
                    self.audio.stop();
                }
                self.audio_active = needs_audio;
            }
            self.session_modified = modified;
        }

        if self.last_topology.elapsed() > Duration::from_secs(3) {
            let displays = serein_platform::displays();
            self.last_topology = Instant::now();
            if displays != self.displays && !displays.is_empty() {
                self.displays = displays;
                self.rebuild(event_loop)?;
            }
        }
        if self.last_status.elapsed() >= Duration::from_secs(1) {
            self.last_status = Instant::now();
            for surface in &self.windows {
                let RawWindowHandle::Win32(handle) = surface
                    .window
                    .window_handle()
                    .map_err(|e| e.to_string())?
                    .as_raw()
                else {
                    return Err("Desktop window handle is unavailable".into());
                };
                if !serein_platform::wallpaper_attached(handle.hwnd.get(), &surface.display) {
                    self.status("Desktop attachment lost · reconnecting…".into());
                    self.rebuild(event_loop)?;
                    return Ok(());
                }
            }
            self.status(if self.frames == 0 {
                "Preparing the first desktop frame…".into()
            } else if self.paused {
                if self.session.paused {
                    "Paused · use Resume wallpaper to continue".into()
                } else {
                    "Paused by automatic playback rules".into()
                }
            } else {
                format!(
                    "Desktop attached · {} display(s) · {} FPS ceiling",
                    self.windows.len(),
                    self.session.project.preview_fps
                )
            });
        }
        Ok(())
    }
    fn render(&mut self, id: WindowId) -> Result<(), String> {
        let Some(gpu) = &self.gpu else {
            return Ok(());
        };
        let Some(surface) = self.windows.iter_mut().find(|s| s.window.id() == id) else {
            return Ok(());
        };
        let frame = match surface.surface.get_current_texture() {
            Ok(frame) => frame,
            Err(wgpu::SurfaceError::Lost | wgpu::SurfaceError::Outdated) => {
                surface.surface.configure(&gpu.device, &surface.config);
                return Ok(());
            }
            Err(wgpu::SurfaceError::Timeout | wgpu::SurfaceError::Other) => return Ok(()),
            Err(error) => return Err(format!("Wallpaper presentation failed: {error}")),
        };
        let (origin_x, origin_y, width, height) = if self.session.display_mode == DisplayMode::Span
        {
            virtual_bounds(&self.displays).ok_or("Display bounds unavailable")?
        } else {
            (
                surface.display.x as i64,
                surface.display.y as i64,
                surface.display.width as u64,
                surface.display.height as u64,
            )
        };
        let pointer = serein_platform::pointer_position()
            .map(|(x, y)| {
                [
                    ((x as i64 - origin_x) as f32 / width as f32).clamp(0., 1.),
                    ((y as i64 - origin_y) as f32 / height as f32).clamp(0., 1.),
                ]
            })
            .unwrap_or([0.5, 0.5]);
        for (value, target) in surface.smoothed_pointer.iter_mut().zip(pointer) {
            *value += (target - *value) * 0.12;
        }
        let snapshot = self.audio.snapshot();
        let audio = if snapshot.updated.elapsed() < Duration::from_millis(500) {
            snapshot.frame.bus
        } else {
            0.
        };
        let mut uniforms = Uniforms::new(
            [width as f32, height as f32],
            self.clock,
            audio,
            surface.smoothed_pointer,
            &self.session.project.appearance,
        );
        uniforms.view = surface.view;
        surface.material.update(&gpu.queue, uniforms);
        let view = frame.texture.create_view(&Default::default());
        let mut encoder = gpu.device.create_command_encoder(&Default::default());
        {
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("Desktop wallpaper"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &view,
                    depth_slice: None,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color::BLACK),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                ..Default::default()
            });
            surface.material.draw(&mut pass);
        }
        gpu.queue.submit([encoder.finish()]);
        frame.present();
        surface.ready = true;
        self.frames += 1;
        Ok(())
    }
    fn fail(&mut self, event_loop: &ActiveEventLoop, error: String) {
        self.error = Some(error.clone());
        self.status(error);
        event_loop.exit();
    }
}
impl ApplicationHandler for Player {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if let Err(error) = self.rebuild(event_loop) {
            self.fail(event_loop, error);
        }
    }
    fn window_event(&mut self, event_loop: &ActiveEventLoop, id: WindowId, event: WindowEvent) {
        match event {
            WindowEvent::RedrawRequested => {}
            WindowEvent::Resized(size) if size.width > 0 && size.height > 0 => {
                if let (Some(gpu), Some(window)) = (
                    &self.gpu,
                    self.windows.iter_mut().find(|s| s.window.id() == id),
                ) {
                    window.config.width = size.width;
                    window.config.height = size.height;
                    window.surface.configure(&gpu.device, &window.config);
                }
            }
            WindowEvent::CloseRequested | WindowEvent::Destroyed
                if self.windows.iter().any(|surface| surface.window.id() == id) =>
            {
                event_loop.exit();
            }
            // Rebuilding drops the previous surfaces. Their queued Destroyed
            // events must not terminate the replacement desktop windows.
            _ => {}
        }
    }
    fn about_to_wait(&mut self, event_loop: &ActiveEventLoop) {
        if let Err(error) = self.refresh(event_loop) {
            self.fail(event_loop, error);
            return;
        }
        let photo = self.session.project.image.is_some()
            && self.session.project.shader.trim() == serein_render::catalog::IMAGE_SHADER.trim();
        let mut active_image = true;
        if photo {
            let pointer = serein_platform::pointer_position();
            let bus = self.audio.snapshot().frame.bus;
            if pointer != self.last_pointer || (bus - self.last_bus).abs() > 0.002 {
                self.last_input = Instant::now();
                self.last_pointer = pointer;
                self.last_bus = bus;
            }
            // A settled photo does not submit identical GPU frames indefinitely.
            active_image = self.last_input.elapsed() < Duration::from_millis(1500);
        }
        let interval = Duration::from_secs_f64(1. / self.session.project.preview_fps.max(1) as f64);
        if (!self.paused && active_image && self.last_frame.elapsed() >= interval)
            || self.windows.iter().any(|s| !s.ready)
        {
            self.clock += self.last_frame.elapsed().as_secs_f32().min(1.0);
            self.last_frame = Instant::now();
            // Desktop child windows can stay occluded under Explorer's transparent
            // icon layer. WM_PAINT delivery is therefore not a playback clock.
            let windows: Vec<_> = self
                .windows
                .iter()
                .map(|surface| surface.window.id())
                .collect();
            for id in windows {
                if let Err(error) = self.render(id) {
                    self.fail(event_loop, error);
                    return;
                }
            }
        }
        event_loop.set_control_flow(ControlFlow::WaitUntil(
            Instant::now()
                + if self.paused {
                    Duration::from_millis(250)
                } else {
                    if photo && !active_image {
                        interval
                    } else {
                        interval.saturating_sub(self.last_frame.elapsed())
                    }
                },
        ));
    }
}
