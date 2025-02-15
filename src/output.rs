use rodio::{Source, source};
use std::time::Duration;

// return-position `impl Trait` presents some lifetime issues that I can't seem to solve yet, so whatever.
// Proper use of impl-Trait here seems to be waiting on #![feature(precise_capturing_of_types)]
pub type PartSource<S> = source::Amplify<source::FromIter<std::vec::IntoIter<S>>>;
pub type EventSource<S> = source::TakeDuration<Chord<S>>;

impl crate::types::Part {
    pub fn to_source<S>(
        &self,
        instrument: fn(f32, Duration) -> S,
        beat_duration: Duration,
    ) -> PartSource<EventSource<S>>
    where
        S: Source<Item = f32>,
    {
        let mut event_sources = vec![];

        for event in &self.events {
            event_sources.push(event.to_source(instrument, beat_duration));
        }

        rodio::source::from_iter(event_sources).amplify(0.2)
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
    pub fn sine(freq: f32, _duration: Duration) -> impl Source<Item = f32> {
        source::SignalGenerator::new(48000, freq, source::Function::Sine)
    }

    // (cons 'square (lambda (f) (format "ceil(sin(t*%.2f))" (* 2 float-pi f))))
    pub fn square(freq: f32, _duration: Duration) -> impl Source<Item = f32> {
        source::SignalGenerator::new(48000, freq, source::Function::Square)
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
