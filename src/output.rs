use rodio::source::*;
use std::time::Duration;

pub trait Instrument {
    type Note: Source<Item = f32> + Send + 'static;

    fn play_note(frequency: f32, duration: Duration) -> Self::Note;

    fn play_chord(
        event: &crate::types::Event,
        beat_duration: Duration,
    ) -> (Chord<Self::Note>, Duration) {
        let mut chord = crate::output::Chord { notes: vec![] };

        let event_duration = event.beat_count() * beat_duration;

        for note in event.notes() {
            let freq = note.to_frequency();
            let note = Self::play_note(freq, event_duration);
            chord.add(note);
        }

        (chord, event_duration)
    }

    // TODO: only use delay+chord style sequencing for instruments that actually need it,
    // i.e., ones that apply sustain to notes. for instruments that don't, the more
    // lightweight and straightforward from_iter approach works just fine
    fn play_part(part: &crate::types::Part, beat_duration: Duration) -> BoxSource<f32> {
        let mut track = Chord::new();

        let mut offset = Duration::ZERO;

        for event in &part.events {
            let (chord, event_duration) = Self::play_chord(event, beat_duration);
            if !event.notes().is_empty() {
                track.add(chord.delay(offset));
            }

            offset += event_duration;
        }

        Box::new(track.low_pass(1000).take_duration(offset))
    }
}

pub type BoxSource<T> = Box<dyn Source<Item = T> + Send + 'static>;

/// A more barebones implementation of something like rodio::mixer,
/// with no fancy boxed sources or shared references.
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
