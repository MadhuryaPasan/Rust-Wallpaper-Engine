use crate::{Analyzer, AudioFrame, FFT_SIZE, SAMPLE_RATE};
use serein_domain::AudioSettings;
use std::sync::{
    Arc, Mutex,
    atomic::{AtomicBool, Ordering},
};
use std::{
    thread,
    time::{Duration, Instant},
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AudioSource {
    TestSignal,
    SystemOutput,
}

#[derive(Clone)]
pub struct AudioSnapshot {
    pub frame: AudioFrame,
    pub status: String,
    pub running: bool,
    pub updated: Instant,
}

pub struct AudioService {
    shared: Arc<Mutex<AudioSnapshot>>,
    settings: Arc<Mutex<AudioSettings>>,
    stop: Arc<AtomicBool>,
    worker: Option<thread::JoinHandle<()>>,
    device_id: Option<String>,
}

impl Default for AudioService {
    fn default() -> Self {
        Self::new()
    }
}
impl AudioService {
    pub fn new() -> Self {
        Self {
            shared: Arc::new(Mutex::new(AudioSnapshot {
                frame: AudioFrame::default(),
                status: "Analyzer stopped".into(),
                running: false,
                updated: Instant::now(),
            })),
            settings: Arc::new(Mutex::new(AudioSettings::default())),
            stop: Arc::new(AtomicBool::new(false)),
            worker: None,
            device_id: None,
        }
    }
    pub fn set_device(&mut self, device_id: Option<String>) {
        self.device_id = device_id;
    }
    pub fn configure(&self, settings: AudioSettings) {
        if let Ok(mut current) = self.settings.lock() {
            *current = settings;
        }
    }
    pub fn snapshot(&self) -> AudioSnapshot {
        self.shared
            .lock()
            .map(|s| s.clone())
            .unwrap_or_else(|_| AudioSnapshot {
                frame: AudioFrame::default(),
                status: "Analyzer unavailable. Stop and restart the source.".into(),
                running: false,
                updated: Instant::now(),
            })
    }
    pub fn start(&mut self, source: AudioSource) -> Result<(), String> {
        self.stop();
        self.stop.store(false, Ordering::Relaxed);
        let shared = Arc::clone(&self.shared);
        let settings = Arc::clone(&self.settings);
        let stop = Arc::clone(&self.stop);
        let device_id = self.device_id.clone();
        if let Ok(mut snapshot) = shared.lock() {
            snapshot.running = true;
            snapshot.status = "Starting audio source…".into();
        }
        self.worker = Some(thread::Builder::new().name("serein-audio-analysis".into()).spawn(move || {
            let mut pipeline = AnalysisWorker::new(shared.clone(), settings, stop);
            match source {
                AudioSource::TestSignal => pipeline.test_signal(),
                AudioSource::SystemOutput => {
                    while !pipeline.stop.load(Ordering::Relaxed) {
                        if let Err(error) = capture_system(&mut pipeline, device_id.as_deref()) {
                            if let Ok(mut snapshot) = shared.lock() {
                                snapshot.frame = AudioFrame::default();
                                snapshot.status = format!("Output unavailable · reconnecting automatically: {error}");
                            }
                            for _ in 0..10 { if pipeline.stop.load(Ordering::Relaxed) { break; } thread::sleep(Duration::from_millis(50)); }
                        }
                    }
                }
            }
        }).map_err(|e| format!("Unable to start audio analysis: {e}. Stop other previews and try again."))?);
        Ok(())
    }
    pub fn stop(&mut self) {
        self.stop.store(true, Ordering::Relaxed);
        if let Some(worker) = self.worker.take() {
            let _ = worker.join();
        }
        if let Ok(mut snapshot) = self.shared.lock() {
            snapshot.running = false;
            snapshot.frame = AudioFrame::default();
            snapshot.status = "Analyzer stopped".into();
        }
    }
}
impl Drop for AudioService {
    fn drop(&mut self) {
        self.stop();
    }
}

struct AnalysisWorker {
    shared: Arc<Mutex<AudioSnapshot>>,
    settings: Arc<Mutex<AudioSettings>>,
    stop: Arc<AtomicBool>,
    analyzer: Analyzer,
    applied: AudioSettings,
}
impl AnalysisWorker {
    fn new(
        shared: Arc<Mutex<AudioSnapshot>>,
        settings: Arc<Mutex<AudioSettings>>,
        stop: Arc<AtomicBool>,
    ) -> Self {
        let applied = settings.lock().map(|s| s.clone()).unwrap_or_default();
        Self {
            shared,
            settings,
            stop,
            analyzer: Analyzer::new(applied.clone()),
            applied,
        }
    }
    fn publish(&mut self, samples: &[f32; FFT_SIZE], status: &str) {
        if let Ok(settings) = self.settings.lock()
            && *settings != self.applied
        {
            self.applied = settings.clone();
            self.analyzer.configure(self.applied.clone());
        }
        let frame = self.analyzer.process(samples);
        // Only the newest frame is retained. A slow UI cannot accumulate latency.
        if let Ok(mut snapshot) = self.shared.lock() {
            snapshot.frame = frame.clone();
            snapshot.status = status.into();
            snapshot.updated = Instant::now();
        }
    }
    fn test_signal(&mut self) {
        let mut sample_index = 0_u64;
        let step = Duration::from_secs_f32(FFT_SIZE as f32 / SAMPLE_RATE);
        while !self.stop.load(Ordering::Relaxed) {
            let start = Instant::now();
            let samples = std::array::from_fn(|i| {
                let t = (sample_index + i as u64) as f64 / f64::from(SAMPLE_RATE);
                let beat = ((t * 1.7).fract() * -5.).exp();
                let bass = (std::f64::consts::TAU * 82. * t).sin() * (0.03 + beat * 0.24);
                let mid = (std::f64::consts::TAU * 660. * t).sin() * 0.04;
                let high =
                    (std::f64::consts::TAU * 3600. * t).sin() * 0.015 * (1. + (t * 3.).sin());
                (bass + mid + high) as f32
            });
            sample_index += FFT_SIZE as u64;
            self.publish(
                &samples,
                "Generated test signal · not played through speakers",
            );
            thread::sleep(step.saturating_sub(start.elapsed()));
        }
    }
}

#[cfg(not(windows))]
fn capture_system(_: &mut AnalysisWorker, _: Option<&str>) -> Result<(), String> {
    Err("System loopback is available on Windows".into())
}

#[cfg(windows)]
fn capture_system(worker: &mut AnalysisWorker, device_id: Option<&str>) -> Result<(), String> {
    use wasapi::{DeviceEnumerator, Direction, SampleType, StreamMode, WaveFormat};
    let error = |e: wasapi::WasapiError| e.to_string();
    wasapi::initialize_mta().ok().map_err(|e| e.to_string())?;
    struct ComGuard;
    impl Drop for ComGuard {
        fn drop(&mut self) {
            wasapi::deinitialize();
        }
    }
    let _com = ComGuard;
    let enumerator = DeviceEnumerator::new().map_err(error)?;
    let device = match device_id {
        Some(id) => enumerator.get_device(id),
        None => enumerator.get_default_device(Direction::Render),
    }
    .map_err(error)?;
    let active_id = device.get_id().map_err(error)?;
    let device_name = device
        .get_friendlyname()
        .unwrap_or_else(|_| "Selected output".into());
    let mut last_device_check = Instant::now();
    let mut client = device.get_iaudioclient().map_err(error)?;
    let format = WaveFormat::new(32, 32, SampleType::Float, SAMPLE_RATE as usize, 2, None);
    client
        .initialize_client(
            &format,
            Direction::Capture,
            StreamMode::EventsShared {
                autoconvert: true,
                buffer_duration_hns: 1_000_000,
            },
            None,
        )
        .map_err(error)?;
    let event = client.set_get_eventhandle().map_err(error)?;
    let capture = client.get_audiocaptureclient().map_err(error)?;
    let mut bytes = vec![0_u8; client.get_buffer_size().map_err(error)? as usize * 8];
    let mut block = [0_f32; FFT_SIZE];
    let mut filled = 0;
    let mut last_packet = Instant::now();
    client.start_stream().map_err(error)?;
    while !worker.stop.load(Ordering::Relaxed) {
        if last_device_check.elapsed() > Duration::from_secs(1) {
            if device_id.is_none()
                && enumerator
                    .get_default_device(Direction::Render)
                    .and_then(|d| d.get_id())
                    .map_err(error)?
                    != active_id
            {
                let _ = client.stop_stream();
                return Err("Windows default output changed".into());
            }
            last_device_check = Instant::now();
        }
        // Bounded wait also lets device changes release the endpoint promptly.
        let _ = event.wait_for_event(50);
        while capture.get_next_packet_size().map_err(error)?.unwrap_or(0) > 0 {
            let (frames, _) = capture.read_from_device(&mut bytes).map_err(error)?;
            last_packet = Instant::now();
            for stereo in bytes[..frames as usize * 8].as_chunks::<8>().0 {
                let left = f32::from_le_bytes([stereo[0], stereo[1], stereo[2], stereo[3]]);
                let right = f32::from_le_bytes([stereo[4], stereo[5], stereo[6], stereo[7]]);
                block[filled] = (left + right) * 0.5;
                filled += 1;
                if filled == FFT_SIZE {
                    worker.publish(
                        &block,
                        &format!("{device_name} \u{00b7} 48 kHz \u{00b7} live output analysis"),
                    );
                    filled = 0;
                }
            }
        }
        if last_packet.elapsed() > Duration::from_millis(100) {
            block.fill(0.);
            filled = 0;
            worker.publish(&block, &format!("{device_name} \u{00b7} waiting for audio"));
            last_packet = Instant::now();
        }
    }
    client.stop_stream().map_err(error)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[cfg(windows)]
    #[test]
    fn disconnected_selection_retries_without_a_user_start_action() {
        let mut service = AudioService::new();
        service.set_device(Some("serein-test-unavailable-endpoint".into()));
        service
            .start(AudioSource::SystemOutput)
            .expect("automatic capture worker");
        let deadline = Instant::now() + Duration::from_secs(2);
        while !service.snapshot().status.contains("reconnecting") && Instant::now() < deadline {
            thread::sleep(Duration::from_millis(20));
        }
        assert!(service.snapshot().running);
        assert!(service.snapshot().status.contains("reconnecting"));
        let start = Instant::now();
        service.stop();
        assert!(start.elapsed() < Duration::from_secs(1));
    }
    #[test]
    fn test_source_publishes_and_stop_clears_state() {
        let mut service = AudioService::new();
        service
            .start(AudioSource::TestSignal)
            .expect("start test source");
        let deadline = Instant::now() + Duration::from_secs(2);
        while service.snapshot().frame.sequence < 2 && Instant::now() < deadline {
            thread::sleep(Duration::from_millis(10));
        }
        let active = service.snapshot();
        assert!(active.running && active.frame.sequence >= 2);
        assert!(active.frame.peak > 0.01);
        service.stop();
        assert!(!service.snapshot().running);
        assert_eq!(service.snapshot().frame.bus, 0.);
    }

    #[cfg(windows)]
    #[test]
    #[ignore = "requires a Windows desktop with an available output endpoint"]
    fn system_loopback_initializes_and_releases_endpoint() {
        let mut service = AudioService::new();
        service
            .start(AudioSource::SystemOutput)
            .expect("start loopback worker");
        let deadline = Instant::now() + Duration::from_secs(3);
        while service.snapshot().frame.sequence == 0
            && service.snapshot().running
            && Instant::now() < deadline
        {
            thread::sleep(Duration::from_millis(20));
        }
        let active = service.snapshot();
        assert!(
            active.running && active.frame.sequence > 0,
            "{}",
            active.status
        );
        service.stop();
        assert!(!service.snapshot().running);
    }
}
