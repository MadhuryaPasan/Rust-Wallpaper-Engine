use crate::filter::{Biquad, Coefficients};
use rustfft::{Fft, FftPlanner, num_complex::Complex};
use serein_domain::AudioSettings;
use std::sync::Arc;

pub const FFT_SIZE: usize = 2048;
pub const SPECTRUM_BINS: usize = 128;
pub const SAMPLE_RATE: f32 = 48000.;

#[derive(Debug, Clone)]
pub struct AudioFrame {
    pub pre_db: [f32; SPECTRUM_BINS],
    pub post_db: [f32; SPECTRUM_BINS],
    pub waveform: [f32; 192],
    pub rms: f32,
    pub peak: f32,
    pub bus: f32,
    pub sequence: u64,
}
impl Default for AudioFrame {
    fn default() -> Self {
        Self {
            pre_db: [-90.; SPECTRUM_BINS],
            post_db: [-90.; SPECTRUM_BINS],
            waveform: [0.; 192],
            rms: 0.,
            peak: 0.,
            bus: 0.,
            sequence: 0,
        }
    }
}

pub struct Analyzer {
    fft: Arc<dyn Fft<f32>>,
    scratch: Vec<Complex<f32>>,
    pre: Vec<Complex<f32>>,
    post: Vec<Complex<f32>>,
    filters: [Biquad; 7],
    coefficients: [Coefficients; 7],
    settings: AudioSettings,
    frame: AudioFrame,
}

impl Default for Analyzer {
    fn default() -> Self {
        Self::new(AudioSettings::default())
    }
}
impl Analyzer {
    pub fn new(settings: AudioSettings) -> Self {
        let fft = FftPlanner::<f32>::new().plan_fft_forward(FFT_SIZE);
        let scratch = vec![Complex::default(); fft.get_inplace_scratch_len()];
        let mut analyzer = Self {
            fft,
            scratch,
            pre: vec![Complex::default(); FFT_SIZE],
            post: vec![Complex::default(); FFT_SIZE],
            filters: std::array::from_fn(|_| Biquad::default()),
            coefficients: [Coefficients::IDENTITY; 7],
            settings: settings.clone(),
            frame: AudioFrame::default(),
        };
        analyzer.configure(settings);
        analyzer
    }
    pub fn configure(&mut self, settings: AudioSettings) {
        let filters_changed =
            settings.bands != self.settings.bands || settings.bypass != self.settings.bypass;
        self.coefficients = settings.bands.map(|band| {
            if settings.bypass {
                Coefficients::IDENTITY
            } else {
                Coefficients::for_band(band, SAMPLE_RATE)
            }
        });
        // Parameter changes reset state to prevent old resonances surviving a bypass.
        // Production automation will interpolate coefficients across a short ramp.
        if filters_changed {
            self.filters = std::array::from_fn(|_| Biquad::default());
        }
        self.settings = settings;
    }
    pub fn process(&mut self, samples: &[f32; FFT_SIZE]) -> &AudioFrame {
        let mut power = 0.;
        let mut peak: f32 = 0.;
        for (i, &raw) in samples.iter().enumerate() {
            let x = if raw.is_finite() { raw } else { 0. };
            let mut filtered = x;
            for (filter, c) in self.filters.iter_mut().zip(self.coefficients) {
                filtered = filter.process(filtered, c);
            }
            let window =
                0.5 - 0.5 * (std::f32::consts::TAU * i as f32 / (FFT_SIZE - 1) as f32).cos();
            self.pre[i] = Complex::new(x * window, 0.);
            self.post[i] = Complex::new(filtered * window, 0.);
            power += filtered * filtered;
            peak = peak.max(filtered.abs());
        }
        for i in 0..192 {
            self.frame.waveform[i] = samples[i * FFT_SIZE / 192];
        }
        self.fft
            .process_with_scratch(&mut self.pre, &mut self.scratch);
        self.fft
            .process_with_scratch(&mut self.post, &mut self.scratch);
        for i in 0..SPECTRUM_BINS {
            let low = 20. * 1000_f32.powf(i as f32 / SPECTRUM_BINS as f32);
            let high = 20. * 1000_f32.powf((i + 1) as f32 / SPECTRUM_BINS as f32);
            let first = (low * FFT_SIZE as f32 / SAMPLE_RATE).floor().max(1.) as usize;
            let last = ((high * FFT_SIZE as f32 / SAMPLE_RATE).ceil() as usize)
                .clamp(first + 1, FFT_SIZE / 2);
            let magnitude = |values: &[Complex<f32>]| {
                values[first..last]
                    .iter()
                    .map(|v| v.norm() * 4. / FFT_SIZE as f32)
                    .fold(0_f32, f32::max)
            };
            self.frame.pre_db[i] = (20. * magnitude(&self.pre).max(1e-5).log10()).max(-90.);
            self.frame.post_db[i] = (20. * magnitude(&self.post).max(1e-5).log10()).max(-90.);
        }
        // A bus integrates FFT power directly, avoiding bias from log-bin overlap.
        let bus = &self.settings.bus;
        let first = (bus.low_hz * FFT_SIZE as f32 / SAMPLE_RATE).ceil().max(1.) as usize;
        let last = (bus.high_hz * FFT_SIZE as f32 / SAMPLE_RATE).floor() as usize;
        let energy: f32 = if first <= last {
            self.post[first..=last.min(FFT_SIZE / 2)]
                .iter()
                .map(|v| v.norm_sqr())
                .sum()
        } else {
            0.
        };
        // Hann mean-square = 3/8. This estimates RMS within the selected range.
        let range_rms = (energy * 2. / (FFT_SIZE as f32).powi(2) / 0.375).sqrt();
        let target = (range_rms * bus.gain).clamp(0., 1.);
        let tau_ms = if target > self.frame.bus {
            bus.attack_ms
        } else {
            bus.release_ms
        };
        let retention = (-(FFT_SIZE as f32 / SAMPLE_RATE) / (tau_ms / 1000.).max(0.001)).exp();
        self.frame.bus = target + retention * (self.frame.bus - target);
        self.frame.rms = (power / FFT_SIZE as f32).sqrt();
        self.frame.peak = peak;
        self.frame.sequence += 1;
        &self.frame
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn tone(hz: f32) -> [f32; FFT_SIZE] {
        std::array::from_fn(|i| 0.2 * (std::f32::consts::TAU * hz * i as f32 / SAMPLE_RATE).sin())
    }
    #[test]
    fn bass_bus_rejects_out_of_range_tone() {
        let mut low = Analyzer::default();
        let mut high = Analyzer::default();
        for _ in 0..10 {
            low.process(&tone(93.75));
            high.process(&tone(6000.));
        }
        assert!(low.frame.bus > 0.35);
        assert!(high.frame.bus < 0.001);
    }
    #[test]
    fn filtering_changes_pcm_and_post_spectrum() {
        let mut settings = AudioSettings::default();
        settings.bands[0].kind = serein_domain::FilterKind::LowPass;
        settings.bands[0].frequency_hz = 300.;
        let mut analyzer = Analyzer::new(settings);
        let frame = analyzer.process(&tone(6000.));
        let pre = frame.pre_db.iter().copied().fold(-90_f32, f32::max);
        let post = frame.post_db.iter().copied().fold(-90_f32, f32::max);
        assert!(pre - post > 30.);
    }
    #[test]
    fn silence_remains_finite_and_envelope_releases() {
        let mut analyzer = Analyzer::default();
        analyzer.process(&tone(93.75));
        let initial = analyzer.frame.bus;
        for _ in 0..30 {
            analyzer.process(&[0.; FFT_SIZE]);
        }
        assert!(analyzer.frame.bus < initial * 0.02);
        assert!(analyzer.frame.post_db.iter().all(|x| x.is_finite()));
    }
}
