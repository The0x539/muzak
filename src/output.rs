use rodio::Source;
use std::time::Duration;

// return-position `impl Trait` presents some lifetime issues that I can't seem to solve yet, so whatever.
// Proper use of impl-Trait here seems to be waiting on #![feature(precise_capturing_of_types)]

impl crate::types::Part {
    pub fn to_source<S: Source<Item = f32> + 'static>(
        &self,
        instrument: fn(f32, Duration) -> S,
        beat_duration: Duration,
    ) -> impl Source<Item = f32> + 'static {
        let mut track = Chord::new();

        let mut offset = Duration::ZERO;

        for event in &self.events {
            let (event, event_duration) = event.to_source(instrument, beat_duration);
            let event = event.delay(offset);
            offset += event_duration;
            track.add(event);
        }

        track.low_pass(1000).take_duration(offset)
    }
}

impl crate::types::Event {
    pub fn to_source<S: Source<Item = f32>>(
        &self,
        instrument: fn(f32, Duration) -> S,
        beat_duration: Duration,
    ) -> (Chord<S>, Duration) {
        let mut chord = Chord { notes: vec![] };

        let event_duration = self.beat_count() * beat_duration;

        for note in self.notes() {
            let freq = note.to_frequency();
            let note = instrument(freq, event_duration);
            chord.add(note);
        }

        (chord, event_duration)
    }
}

pub mod instruments {
    use rodio::{Source, source};
    use std::time::Duration;

    // (cons 'sine (lambda (f) (format "0.7*sin(t*%.2f)" (* 2 float-pi f))))
    // (cons 'beep (muzak/make-instrument :waveform 'sine :effects nil))
    pub fn beep(freq: f32, duration: Duration) -> impl Source<Item = f32> {
        source::SignalGenerator::new(48000, freq, source::Function::Sine)
            .amplify(0.7)
            .take_duration(duration)
    }

    #[allow(dead_code)]
    fn bezelea_square_signal(phase: f32) -> f32 {
        (std::f32::consts::TAU * phase).sin().round()
    }

    // (cons 'square (lambda (f) (format "ceil(sin(t*%.2f))" (* 2 float-pi f))))
    // (cons 'keyboard (muzak/make-instrument :waveform 'square :effects '(linear) :sustain muzak//default-duration))
    pub fn keyboard(freq: f32, duration: Duration) -> impl Source<Item = f32> {
        // let square = source::SignalGenerator::with_function(48000, freq, bezelea_square_signal);
        let square = source::SignalGenerator::new(48000, freq, source::Function::Sawtooth);

        square
            .linear_gain_ramp(duration, 1.0, 0.5, true)
            .amplify(0.5)
            .take_duration(duration)
    }
}

pub struct Chord<I> {
    notes: Vec<I>,
}

impl<I> Chord<I> {
    pub fn new() -> Self {
        Self { notes: vec![] }
    }

    pub fn add(&mut self, note: I) {
        self.notes.push(note);
    }
}

impl<I: Source<Item = f32>> Iterator for Chord<I> {
    type Item = f32;

    fn next(&mut self) -> Option<Self::Item> {
        let mut any = false; // Is there any source still producing samples?
        let mut sum = 0.0;
        for note in &mut self.notes {
            if let Some(sample) = note.next() {
                sum += sample;
                any = true;
            }
        }
        // sum /= self.notes.len() as f32;
        any.then_some(sum)
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

pub trait SourceExt: Source {
    fn boxed(self) -> Box<dyn Source<Item = Self::Item> + Send + Sync + 'static>
    where
        Self: Send + Sync + 'static;
}

impl<S: Source> SourceExt for S {
    fn boxed(self) -> Box<dyn Source<Item = Self::Item> + Send + Sync + 'static>
    where
        Self: Send + Sync + 'static,
    {
        Box::new(self)
    }
}
