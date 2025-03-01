use crate::output::Instrument;
use rodio::source::*;
use std::time::Duration;

const SAMPLE_RATE: rodio::SampleRate = 48000;

// (cons 'sine (lambda (f) (format "0.7*sin(t*%.2f)" (* 2 float-pi f))))
fn sine(frequency: f32) -> SignalGenerator {
    SignalGenerator::new(SAMPLE_RATE, frequency, Function::Sine)
}

// (cons 'square (lambda (f) (format "ceil(sin(t*%.2f))" (* 2 float-pi f))))
fn square(frequency: f32) -> SignalGenerator {
    SignalGenerator::new(SAMPLE_RATE, frequency, Function::Square)
}

// (cons 'beep (muzak/make-instrument :waveform 'sine :effects nil))
pub struct Beep;
impl Instrument for Beep {
    type Note = TakeDuration<Amplify<SignalGenerator>>;

    fn play_note(frequency: f32, duration: Duration) -> Self::Note {
        sine(frequency).amplify(0.7).take_duration(duration)
    }
}

// (cons 'keyboard (muzak/make-instrument :waveform 'square :effects '(linear) :sustain muzak//default-duration))
pub struct Keyboard;
impl Instrument for Keyboard {
    type Note = TakeDuration<Amplify<LinearGainRamp<SignalGenerator>>>;

    fn play_note(frequency: f32, duration: Duration) -> Self::Note {
        square(frequency)
            .linear_gain_ramp(duration, 1.0, 0.5, true)
            .amplify(0.5)
            .take_duration(duration)
    }
}
