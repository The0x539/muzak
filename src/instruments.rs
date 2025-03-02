use crate::output::{ApplyEffect, Effect, Instrument, SourceExt};
use rodio::source::*;
use std::time::Duration;

const SAMPLE_RATE: rodio::SampleRate = 48000;

// (cons 'sine (lambda (f) (format "0.7*sin(t*%.2f)" (* 2 float-pi f))))
// (cons 'square (lambda (f) (format "ceil(sin(t*%.2f))" (* 2 float-pi f))))
// (cons 'triangle (lambda (f) (format "asin(sin(t*%.2f))" (* 2 float-pi f))))

fn wave(function: Function, frequency: f32) -> SignalGenerator {
    SignalGenerator::new(SAMPLE_RATE, frequency, function)
}

// (cons 'dampen (lambda (s _) (format "pow(2.72,-10*%.1f*(t-%.1f))" (or muzak//note-duration muzak//default-duration) s)))
pub struct Dampen;
impl Effect for Dampen {
    fn calculate(&self, elapsed: f32) -> f32 {
        std::f32::consts::E.powf(-2.0 * elapsed)
    }
}

// (cons 'swell (lambda (s d) (format "((t-%.1f)/%.1f)" s (+ d))))
pub struct Swell(pub f32);
impl Effect for Swell {
    fn calculate(&self, elapsed: f32) -> f32 {
        (elapsed / self.0).clamp(0.0, 1.0)
    }
}

// (cons 'beep (muzak/make-instrument :waveform 'sine :effects nil))
pub struct Beep;
impl Instrument for Beep {
    type Note = TakeDuration<Amplify<SignalGenerator>>;

    fn play_note(frequency: f32, duration: Duration) -> Self::Note {
        wave(Function::Sine, frequency)
            .amplify(0.8)
            .take_duration(duration)
    }
}

// (cons 'bells (muzak/make-instrument :waveform 'square :effects '(dampen) :sustain 4))
pub struct Bells;
impl Instrument for Bells {
    type Note = TakeDuration<ApplyEffect<SignalGenerator, Dampen>>;
    const HAS_SUSTAIN: bool = true;

    fn play_note(frequency: f32, duration: Duration) -> Self::Note {
        wave(Function::Square, frequency)
            .with_effect(Dampen)
            .take_duration(duration + Duration::from_secs(4))
    }
}

// (cons 'keyboard (muzak/make-instrument :waveform 'square :effects '(linear)))
pub struct Keyboard;
impl Instrument for Keyboard {
    type Note = TakeDuration<LinearGainRamp<Amplify<SignalGenerator>>>;

    fn play_note(frequency: f32, duration: Duration) -> Self::Note {
        wave(Function::Square, frequency)
            .amplify(0.5)
            .linear_gain_ramp(duration, 1.0, 0.5, true)
            .take_duration(duration /* + Duration::from_millis(200) */)
    }
}

// (cons 'waterphone (muzak/make-instrument :waveform 'triangle :effects '(swell)))
pub struct Waterphone;
impl Instrument for Waterphone {
    type Note = TakeDuration<ApplyEffect<SignalGenerator, Swell>>;

    fn play_note(frequency: f32, duration: Duration) -> Self::Note {
        wave(Function::Triangle, frequency)
            .with_effect(Swell(duration.as_secs_f32()))
            .take_duration(duration)
    }
}
