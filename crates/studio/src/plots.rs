use crate::{app::Message, theme};
use iced::{
    Color, Font, Point, Rectangle, Renderer, Theme, mouse,
    widget::canvas::{self, Action, Frame, Geometry, Path, Stroke, Text},
};
use serein_audio::{AudioFrame, SAMPLE_RATE, SPECTRUM_BINS, response_db};
use serein_domain::AudioSettings;

pub struct Equalizer<'a> {
    pub settings: &'a AudioSettings,
    pub audio: &'a AudioFrame,
    pub selected: usize,
    pub post: bool,
}
#[derive(Default)]
pub struct Drag {
    band: Option<usize>,
}

fn plot_area(bounds: Rectangle) -> Rectangle {
    Rectangle {
        x: 42.,
        y: 18.,
        width: (bounds.width - 88.).max(1.),
        height: (bounds.height - 50.).max(1.),
    }
}
fn x_for(hz: f32, area: Rectangle) -> f32 {
    area.x + (hz / 20.).log10() / 3. * area.width
}
fn y_for(db: f32, area: Rectangle) -> f32 {
    area.y + (24. - db) / 48. * area.height
}
fn label(frame: &mut Frame, content: impl Into<String>, x: f32, y: f32, color: Color, size: f32) {
    frame.fill_text(Text {
        content: content.into(),
        position: Point::new(x, y),
        color,
        size: size.into(),
        font: Font::MONOSPACE,
        ..Default::default()
    });
}
fn stroke(color: Color, width: f32) -> Stroke<'static> {
    Stroke::default().with_color(color).with_width(width)
}

impl canvas::Program<Message> for Equalizer<'_> {
    type State = Drag;
    fn update(
        &self,
        state: &mut Drag,
        event: &canvas::Event,
        bounds: Rectangle,
        cursor: mouse::Cursor,
    ) -> Option<Action<Message>> {
        let area = plot_area(bounds);
        match event {
            canvas::Event::Mouse(mouse::Event::ButtonReleased(mouse::Button::Left)) => {
                state.band = None;
                None
            }
            canvas::Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Left)) => {
                let position = cursor.position_in(bounds)?;
                let nearest = self
                    .settings
                    .bands
                    .iter()
                    .enumerate()
                    .find(|(_, band)| {
                        position.distance(Point::new(
                            x_for(band.frequency_hz, area),
                            y_for(band.gain_db, area),
                        )) < 18.
                    })
                    .or_else(|| {
                        // Clicking the plot selects the closest band horizontally, too.
                        area.contains(position)
                            .then(|| {
                                self.settings
                                    .bands
                                    .iter()
                                    .enumerate()
                                    .min_by(|(_, a), (_, b)| {
                                        (x_for(a.frequency_hz, area) - position.x).abs().total_cmp(
                                            &(x_for(b.frequency_hz, area) - position.x).abs(),
                                        )
                                    })
                            })
                            .flatten()
                    });
                if let Some((index, _)) = nearest {
                    state.band = Some(index);
                    return Some(Action::publish(Message::SelectBand(index)).and_capture());
                }
                None
            }
            canvas::Event::Mouse(mouse::Event::CursorMoved { position })
                if state.band.is_some() =>
            {
                // Use this event's position; a batched cursor snapshot can lag a drag.
                let position = Point::new(position.x - bounds.x, position.y - bounds.y);
                let hz = 20. * 1000_f32.powf(((position.x - area.x) / area.width).clamp(0., 1.));
                let gain = (24. - (position.y - area.y) / area.height * 48.).clamp(-24., 24.);
                Some(Action::publish(Message::DragBand(state.band?, hz, gain)).and_capture())
            }
            _ => None,
        }
    }
    fn draw(
        &self,
        _: &Drag,
        renderer: &Renderer,
        _: &Theme,
        bounds: Rectangle,
        _: mouse::Cursor,
    ) -> Vec<Geometry> {
        let mut frame = Frame::new(renderer, bounds.size());
        let area = plot_area(bounds);
        for hz in [
            20., 50., 100., 200., 500., 1000., 2000., 5000., 10000., 20000.,
        ] {
            let x = x_for(hz, area);
            frame.stroke(
                &Path::line(Point::new(x, area.y), Point::new(x, area.y + area.height)),
                stroke(theme::BORDER.scale_alpha(0.45), 1.),
            );
            let caption = if hz >= 1000. {
                format!("{}k", hz / 1000.)
            } else {
                format!("{hz:.0}")
            };
            label(
                &mut frame,
                caption,
                x - 9.,
                area.y + area.height + 12.,
                theme::MUTED,
                11.,
            );
        }
        for db in [-24., -12., 0., 12., 24.] {
            let y = y_for(db, area);
            frame.stroke(
                &Path::line(Point::new(area.x, y), Point::new(area.x + area.width, y)),
                stroke(
                    theme::BORDER.scale_alpha(if db == 0. { 1. } else { 0.45 }),
                    1.,
                ),
            );
            label(
                &mut frame,
                format!("{db:+.0}"),
                4.,
                y - 6.,
                theme::MUTED,
                11.,
            );
        }
        for db in [0., -30., -60., -90.] {
            let y = area.y + area.height * (-db / 90.);
            label(
                &mut frame,
                format!("{db:.0}"),
                area.x + area.width + 7.,
                y - 6.,
                theme::SIGNAL,
                11.,
            );
        }
        let bins = if self.post {
            &self.audio.post_db
        } else {
            &self.audio.pre_db
        };
        let points: Vec<Point> = bins
            .iter()
            .enumerate()
            .map(|(i, db)| {
                Point::new(
                    area.x + (i as f32 + 0.5) / SPECTRUM_BINS as f32 * area.width,
                    area.y + area.height * (1. - ((db + 90.) / 90.).clamp(0., 1.)),
                )
            })
            .collect();
        let fill = Path::new(|p| {
            p.move_to(Point::new(area.x, area.y + area.height));
            for point in &points {
                p.line_to(*point);
            }
            p.line_to(Point::new(area.x + area.width, area.y + area.height));
            p.close();
        });
        frame.fill(&fill, theme::SIGNAL.scale_alpha(0.12));
        let spectrum = Path::new(|p| {
            p.move_to(points[0]);
            for point in &points[1..] {
                p.line_to(*point);
            }
        });
        frame.stroke(&spectrum, stroke(theme::SIGNAL.scale_alpha(0.65), 1.3));
        let response = Path::new(|p| {
            for i in 0..=400 {
                let hz = 20. * 1000_f32.powf(i as f32 / 400.);
                let db = response_db(&self.settings.bands, self.settings.bypass, hz, SAMPLE_RATE)
                    .clamp(-24., 24.);
                let point = Point::new(x_for(hz, area), y_for(db, area));
                if i == 0 {
                    p.move_to(point);
                } else {
                    p.line_to(point);
                }
            }
        });
        frame.stroke(&response, stroke(theme::TEXT, 2.));
        for (i, band) in self.settings.bands.iter().enumerate() {
            let position = Point::new(x_for(band.frequency_hz, area), y_for(band.gain_db, area));
            let color = if band.enabled {
                theme::BANDS[i]
            } else {
                theme::MUTED
            };
            if i == self.selected {
                frame.fill(&Path::circle(position, 16.), color.scale_alpha(0.14));
            }
            frame.fill(&Path::circle(position, 10.), theme::INSET);
            frame.stroke(
                &Path::circle(position, 10.),
                stroke(color, if i == self.selected { 2. } else { 1. }),
            );
            label(
                &mut frame,
                (i + 1).to_string(),
                position.x - 3.5,
                position.y - 7.,
                color,
                12.,
            );
        }
        vec![frame.into_geometry()]
    }
    fn mouse_interaction(
        &self,
        state: &Drag,
        bounds: Rectangle,
        cursor: mouse::Cursor,
    ) -> mouse::Interaction {
        if state.band.is_some() {
            mouse::Interaction::Grabbing
        } else if cursor.is_over(bounds) {
            mouse::Interaction::Crosshair
        } else {
            mouse::Interaction::default()
        }
    }
}

pub struct Waveform<'a>(pub &'a AudioFrame);
impl canvas::Program<Message> for Waveform<'_> {
    type State = ();
    fn draw(
        &self,
        _: &(),
        renderer: &Renderer,
        _: &Theme,
        bounds: Rectangle,
        _: mouse::Cursor,
    ) -> Vec<Geometry> {
        let mut frame = Frame::new(renderer, bounds.size());
        frame.stroke(
            &Path::line(
                Point::new(0., bounds.height / 2.),
                Point::new(bounds.width, bounds.height / 2.),
            ),
            stroke(theme::BORDER, 1.),
        );
        let path = Path::new(|p| {
            for (i, v) in self.0.waveform.iter().enumerate() {
                let position = Point::new(
                    i as f32 / 191. * bounds.width,
                    bounds.height * (0.5 - v.clamp(-1., 1.) * 0.45),
                );
                if i == 0 {
                    p.move_to(position);
                } else {
                    p.line_to(position);
                }
            }
        });
        frame.stroke(&path, stroke(theme::SIGNAL, 1.3));
        vec![frame.into_geometry()]
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use canvas::Program;

    #[test]
    fn node_drag_maps_window_coordinates_to_frequency_and_gain() {
        let settings = AudioSettings::default();
        let audio = AudioFrame::default();
        let plot = Equalizer {
            settings: &settings,
            audio: &audio,
            selected: 0,
            post: true,
        };
        let bounds = Rectangle {
            x: 120.,
            y: 230.,
            width: 700.,
            height: 225.,
        };
        let area = plot_area(bounds);
        let start = Point::new(bounds.x + x_for(120., area), bounds.y + y_for(0., area));
        let mut state = Drag::default();
        let selection = plot
            .update(
                &mut state,
                &canvas::Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Left)),
                bounds,
                mouse::Cursor::Available(start),
            )
            .expect("node press")
            .into_inner()
            .0;
        assert!(matches!(selection, Some(Message::SelectBand(1))));
        let end = Point::new(bounds.x + x_for(250., area), bounds.y + y_for(6., area));
        // Deliberately stale cursor snapshot: the current event must win.
        let change = plot
            .update(
                &mut state,
                &canvas::Event::Mouse(mouse::Event::CursorMoved { position: end }),
                bounds,
                mouse::Cursor::Available(start),
            )
            .expect("drag message")
            .into_inner()
            .0;
        match change {
            Some(Message::DragBand(1, frequency, gain)) => {
                assert!((frequency - 250.).abs() < 0.1);
                assert!((gain - 6.).abs() < 0.01);
            }
            _ => panic!("unexpected drag message"),
        }
        plot.update(
            &mut state,
            &canvas::Event::Mouse(mouse::Event::ButtonReleased(mouse::Button::Left)),
            bounds,
            mouse::Cursor::Available(end),
        );
        assert!(state.band.is_none());
    }
}
