use rodio::source::*;
use std::time::Duration;

use crate::types::PartItem;

pub trait Instrument {
    type Note: Source<Item = f32> + Send + 'static;

    // Instruments without sustain can be mixed by just concatenating audio sources,
    // which is significantly faster than the delay&sum approach.
    const HAS_SUSTAIN: bool = false;

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

    fn play_part(part: &crate::types::Part, beat_duration: Duration) -> BoxSource<f32> {
        if Self::HAS_SUSTAIN {
            Box::new(play_part_with_sustain::<Self>(part, beat_duration))
        } else {
            Box::new(play_part_without_sustain::<Self>(part, beat_duration))
        }
    }
}

fn play_part_with_sustain<T: Instrument + ?Sized>(
    part: &crate::types::Part,
    beat_duration: Duration,
) -> impl Source<Item = f32> + 'static {
    let mut track = Chord::new();
    let mut offset = Duration::ZERO;
    let mut volume = 1.0;

    for item in &part.items {
        match item {
            PartItem::Event(event) => {
                let (chord, event_duration) = T::play_chord(event, beat_duration);
                if !event.notes().is_empty() {
                    track.add(chord.amplify(volume).delay(offset));
                }
                offset += event_duration;
            }
            PartItem::Dynamic(dynamic) => volume = dynamic.to_multiplier(),
        }
    }

    track.low_pass(1000)
}

fn play_part_without_sustain<T: Instrument + ?Sized>(
    part: &crate::types::Part,
    beat_duration: Duration,
) -> impl Source<Item = f32> + 'static {
    let mut events = vec![];

    let mut volume = 1.0;

    for item in &part.items {
        match item {
            PartItem::Event(event) => {
                let (chord, duration) = T::play_chord(event, beat_duration);
                // Notes already adjust themselves to the necessary duration,
                // but rests do not and would otherwise be infinite.
                events.push(chord.amplify(volume).take_duration(duration));
            }
            PartItem::Dynamic(dynamic) => volume = dynamic.to_multiplier(),
        }
    }

    from_iter(events).low_pass(1000)
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
        if self.notes.is_empty() {
            // No notes means a rest, which is silence, not no duration.
            return Some(0.0);
        }

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
