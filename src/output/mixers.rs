use rodio::Source;

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

    fn total_duration(&self) -> Option<std::time::Duration> {
        self.notes.get(0).and_then(|n| n.total_duration())
    }
}
