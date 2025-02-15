use rodio::{Source, source::SineWave};

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
