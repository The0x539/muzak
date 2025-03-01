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

// This is only distinct from the Fn trait to make nameable types for static dispatch.
pub trait Effect {
    fn calculate(&self, elapsed: f32) -> f32;
}

/// A generic wrapper to apply any "time -> multiplier" modifier to a source.
#[derive(Clone, Debug)]
pub struct ApplyEffect<I, E> {
    input: I,
    effect: E,
    elapsed: f32,
    sample_idx: u64,
}

impl<I: Source, E: Effect> ApplyEffect<I, E> {
    pub fn new(input: I, effect: E) -> Self {
        Self {
            input,
            effect,
            elapsed: 0.0,
            sample_idx: 0,
        }
    }
}

impl<I: Source, E: Effect> Iterator for ApplyEffect<I, E> {
    type Item = I::Item;

    fn next(&mut self) -> Option<I::Item> {
        let factor = self.effect.calculate(self.elapsed);

        self.sample_idx += 1;
        if self.sample_idx % (self.channels() as u64) == 0 {
            self.elapsed += 1.0 / (self.input.sample_rate() as f32);
        }

        self.input.next().map(|value| value * factor)
    }

    #[inline]
    fn size_hint(&self) -> (usize, Option<usize>) {
        self.input.size_hint()
    }
}

impl<I: Source, E: Effect> Source for ApplyEffect<I, E> {
    fn current_span_len(&self) -> Option<usize> {
        self.input.current_span_len()
    }

    fn channels(&self) -> rodio::ChannelCount {
        self.input.channels()
    }

    fn sample_rate(&self) -> rodio::SampleRate {
        self.input.sample_rate()
    }

    fn total_duration(&self) -> Option<Duration> {
        self.input.total_duration()
    }

    fn try_seek(&mut self, pos: Duration) -> Result<(), SeekError> {
        self.elapsed = pos.as_secs_f32();
        self.input.try_seek(pos)
    }
}

pub trait SourceExt: Source<Item = f32> + Sized {
    fn with_effect<E: Effect>(self, effect: E) -> ApplyEffect<Self, E> {
        ApplyEffect::new(self, effect)
    }
}

impl<S: Source<Item = f32>> SourceExt for S {}
