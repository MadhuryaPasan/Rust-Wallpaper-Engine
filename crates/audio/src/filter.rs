use serein_domain::{EqBand, FilterKind};
use std::f64::consts::PI;

#[derive(Debug, Clone, Copy)]
pub struct Coefficients {
    b: [f64; 3],
    a: [f64; 3],
}

impl Coefficients {
    pub const IDENTITY: Self = Self {
        b: [1., 0., 0.],
        a: [1., 0., 0.],
    };
    pub fn for_band(band: EqBand, sample_rate: f32) -> Self {
        if !band.enabled {
            return Self::IDENTITY;
        }
        let omega = 2. * PI * f64::from(band.frequency_hz.clamp(1., sample_rate * 0.49))
            / f64::from(sample_rate.max(1.));
        let c = omega.cos();
        let s = omega.sin();
        let alpha = s / (2. * f64::from(band.q.clamp(0.1, 18.)));
        let a = 10_f64.powf(f64::from(band.gain_db.clamp(-24., 24.)) / 40.);
        // RBJ cookbook coefficients; normalizing once keeps the sample loop small.
        let (b, d) = match band.kind {
            FilterKind::Peak => (
                [1. + alpha * a, -2. * c, 1. - alpha * a],
                [1. + alpha / a, -2. * c, 1. - alpha / a],
            ),
            FilterKind::LowPass => (
                [(1. - c) / 2., 1. - c, (1. - c) / 2.],
                [1. + alpha, -2. * c, 1. - alpha],
            ),
            FilterKind::HighPass => (
                [(1. + c) / 2., -(1. + c), (1. + c) / 2.],
                [1. + alpha, -2. * c, 1. - alpha],
            ),
            FilterKind::Notch => ([1., -2. * c, 1.], [1. + alpha, -2. * c, 1. - alpha]),
            FilterKind::LowShelf => {
                let k = 2. * a.sqrt() * alpha;
                (
                    [
                        a * ((a + 1.) - (a - 1.) * c + k),
                        2. * a * ((a - 1.) - (a + 1.) * c),
                        a * ((a + 1.) - (a - 1.) * c - k),
                    ],
                    [
                        (a + 1.) + (a - 1.) * c + k,
                        -2. * ((a - 1.) + (a + 1.) * c),
                        (a + 1.) + (a - 1.) * c - k,
                    ],
                )
            }
            FilterKind::HighShelf => {
                let k = 2. * a.sqrt() * alpha;
                (
                    [
                        a * ((a + 1.) + (a - 1.) * c + k),
                        -2. * a * ((a - 1.) + (a + 1.) * c),
                        a * ((a + 1.) + (a - 1.) * c - k),
                    ],
                    [
                        (a + 1.) - (a - 1.) * c + k,
                        2. * ((a - 1.) - (a + 1.) * c),
                        (a + 1.) - (a - 1.) * c - k,
                    ],
                )
            }
        };
        Self {
            b: b.map(|v| v / d[0]),
            a: d.map(|v| v / d[0]),
        }
    }

    pub fn magnitude(self, frequency_hz: f32, sample_rate: f32) -> f32 {
        let omega = 2. * PI * f64::from(frequency_hz) / f64::from(sample_rate);
        let power = |v: [f64; 3]| {
            let real = v[0] + v[1] * omega.cos() + v[2] * (2. * omega).cos();
            let imag = -v[1] * omega.sin() - v[2] * (2. * omega).sin();
            real * real + imag * imag
        };
        (power(self.b) / power(self.a).max(1e-24)).sqrt() as f32
    }
}

#[derive(Default)]
pub struct Biquad {
    z1: f64,
    z2: f64,
}

impl Biquad {
    pub fn process(&mut self, input: f32, c: Coefficients) -> f32 {
        let x = f64::from(input);
        let y = c.b[0] * x + self.z1;
        self.z1 = c.b[1] * x - c.a[1] * y + self.z2;
        self.z2 = c.b[2] * x - c.a[2] * y;
        if !y.is_finite() {
            self.z1 = 0.;
            self.z2 = 0.;
            return 0.;
        }
        y as f32
    }
}

pub fn response_db(bands: &[EqBand; 7], bypass: bool, hz: f32, sample_rate: f32) -> f32 {
    if bypass {
        return 0.;
    }
    let gain: f32 = bands
        .iter()
        .map(|band| Coefficients::for_band(*band, sample_rate).magnitude(hz, sample_rate))
        .product();
    20. * gain.max(1e-6).log10()
}

#[cfg(test)]
mod tests {
    use super::*;
    fn band(kind: FilterKind) -> EqBand {
        EqBand {
            kind,
            frequency_hz: 1000.,
            gain_db: 12.,
            q: 0.707,
            enabled: true,
        }
    }
    #[test]
    fn bell_reaches_requested_gain_at_center() {
        let c = Coefficients::for_band(band(FilterKind::Peak), 48000.);
        assert!((20. * c.magnitude(1000., 48000.).log10() - 12.).abs() < 0.01);
    }
    #[test]
    fn low_pass_rejects_high_frequencies() {
        let c = Coefficients::for_band(band(FilterKind::LowPass), 48000.);
        assert!(c.magnitude(100., 48000.) > 0.99);
        assert!(c.magnitude(10000., 48000.) < 0.02);
    }
    #[test]
    fn disabled_filter_is_identity() {
        let mut settings = band(FilterKind::Notch);
        settings.enabled = false;
        let c = Coefficients::for_band(settings, 48000.);
        let mut filter = Biquad::default();
        for sample in [0.5, -0.7, 0., 1.] {
            assert_eq!(filter.process(sample, c), sample);
        }
    }
}
