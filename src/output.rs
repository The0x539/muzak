use rodio::{Source, source};
use std::time::Duration;

// return-position `impl Trait` presents some lifetime issues that I can't seem to solve yet, so whatever.
// Proper use of impl-Trait here seems to be waiting on #![feature(precise_capturing_of_types)]
pub type EventSource<S> = source::TakeDuration<Chord<S>>;

impl crate::types::Part {
    pub fn to_source<S>(
        &self,
        instrument: fn(f32, Duration) -> S,
        beat_duration: Duration,
    ) -> impl Source<Item = f32> + 'static
    where
        S: Source<Item = f32> + 'static,
    {
        let mut event_sources = vec![];

        for event in &self.events {
            event_sources.push(event.to_source(instrument, beat_duration));
        }

        rodio::source::from_iter(event_sources).low_pass(540)
    }
}

impl crate::types::Event {
    pub fn to_source<S>(
        &self,
        instrument: fn(f32, Duration) -> S,
        beat_duration: Duration,
    ) -> EventSource<S>
    where
        S: Source<Item = f32>,
    {
        let mut chord = Chord { notes: vec![] };

        let event_duration = self.duration() * beat_duration;

        for note in self.notes() {
            let freq = note.to_frequency();
            let note = instrument(freq, event_duration);
            chord.notes.push(note);
        }

        chord.take_duration(event_duration)
    }
}

pub mod instruments {
    use rodio::{Source, source};
    use std::time::Duration;

    // (cons 'sine (lambda (f) (format "0.7*sin(t*%.2f)" (* 2 float-pi f))))
    // (cons 'beep (muzak/make-instrument :waveform 'sine :effects nil))
    pub fn beep(freq: f32, _duration: Duration) -> impl Source<Item = f32> {
        source::SignalGenerator::new(48000, freq, source::Function::Sine).amplify(0.7)
    }

    #[allow(dead_code)]
    fn bezelea_square_signal(phase: f32) -> f32 {
        (std::f32::consts::TAU * phase).sin().round()
    }

    // (cons 'square (lambda (f) (format "ceil(sin(t*%.2f))" (* 2 float-pi f))))
    // (cons 'keyboard (muzak/make-instrument :waveform 'square :effects '(linear) :sustain muzak//default-duration))
    pub fn keyboard(freq: f32, duration: Duration) -> impl Source<Item = f32> {
        // let square = source::SignalGenerator::with_function(48000, freq, bezelea_square_signal);
        let square = source::SignalGenerator::new(48000, freq, source::Function::Square);

        square.linear_gain_ramp(duration, 1.0, 0.5, true)
    }
}

pub struct Chord<I: Source<Item = f32>> {
    pub(crate) notes: Vec<I>,
}

impl<I: Source<Item = f32>> Iterator for Chord<I> {
    type Item = f32;

    fn next(&mut self) -> Option<Self::Item> {
        if self.notes.is_empty() {
            return Some(0.0);
        }

        let sum: f32 = self.notes.iter_mut().filter_map(|n| n.next()).sum();
        Some(sum / self.notes.len() as f32)
    }
}

impl<I: Source<Item = f32>> Source for Chord<I> {
    fn current_span_len(&self) -> Option<usize> {
        self.notes.get(0).and_then(|n| n.current_span_len())
    }

    fn channels(&self) -> u16 {
        self.notes.get(0).map_or(1, |n| n.channels())
    }

    fn sample_rate(&self) -> u32 {
        self.notes.get(0).map_or(48000, |n| n.sample_rate())
    }

    fn total_duration(&self) -> Option<Duration> {
        if self.notes.is_empty() {
            return None;
        }

        let mut dur = Duration::from_secs(0);
        for note in &self.notes {
            // if any component has unknown duration, the mixer as a whole does
            dur = dur.max(note.total_duration()?);
        }
        Some(dur)
    }
}
