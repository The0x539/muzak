use rodio::{Source, source::SineWave};
use std::time::Duration;

// (cons 'sine (lambda (f) (format "0.7*sin(t*%.2f)" (* 2 float-pi f))))
pub fn sine(freq: f32) -> SineWave {
    SineWave::new(freq)
}

pub fn square(freq: f32) -> SquareWave {
    SquareWave {
        sine: SineWave::new(freq),
    }
}

pub struct SquareWave {
    sine: SineWave,
}

impl Iterator for SquareWave {
    type Item = f32;

    fn next(&mut self) -> Option<Self::Item> {
        self.sine.next().map(|n| n.ceil())
    }
}

impl Source for SquareWave {
    fn current_frame_len(&self) -> Option<usize> {
        self.sine.current_frame_len()
    }

    fn channels(&self) -> u16 {
        self.sine.channels()
    }

    fn sample_rate(&self) -> u32 {
        self.sine.sample_rate()
    }

    fn total_duration(&self) -> Option<std::time::Duration> {
        self.sine.total_duration()
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

        let (mut sum, mut count) = (0.0, 0);
        for note in &mut self.notes {
            if let Some(sample) = note.next() {
                sum += sample;
                count += 1;
            }
        }
        (count > 0).then_some(sum / count as f32)
    }
}

impl<I: Source<Item = f32>> Source for Chord<I> {
    fn current_frame_len(&self) -> Option<usize> {
        self.notes.get(0).and_then(|n| n.current_frame_len())
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
