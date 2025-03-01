use std::time::Duration;

use rodio::Source;
use types::Score;

pub use rodio;

mod compile;
mod fmt;
mod parse;

pub mod output;
pub mod types;

pub use compile::compile;

use output::instruments;

type Result<T, E = Box<dyn std::error::Error + Send + Sync>> = std::result::Result<T, E>;

type ParseError<'a> = winnow::error::ParseError<&'a str, winnow::error::ContextError>;

// TODO: allow specifying this, and pass it down to the sources as needed
const SAMPLE_RATE: rodio::SampleRate = 48000;

pub fn parse(song_text: &str) -> Result<types::Score, ParseError<'_>> {
    winnow::Parser::parse(&mut parse::score, &mut { song_text })
}

pub fn mix(
    score: &Score,
    max_duration: Option<Duration>,
) -> (impl Source<Item = f32> + 'static, Duration) {
    let beat = score.beat_duration();

    let (mixer_sink, mixer_source) = rodio::mixer::mixer(1, SAMPLE_RATE);

    for (i, part) in score.parts.iter().enumerate() {
        match i {
            0 => mixer_sink.add(part.to_source(instruments::beep, beat)),
            _ => mixer_sink.add(part.to_source(instruments::keyboard, beat)),
        }
    }

    let desired_duration = beat * score.beat_count();
    let true_duration = max_duration.unwrap_or(Duration::MAX).min(desired_duration);
    // At the time of writing, rodio is not designed such that take_duration on
    // an infinite source can correctly report its finite and guaranteed duration.
    // This is exactly what we do to construct individual notes/chords,
    // so the full combined source currently cannot report its own duration.
    // This knowledge is necessary for the `sleep` call in `play`, so just return it for now.
    (mixer_source.take_duration(true_duration), true_duration)
}

pub fn play(source: impl Source<Item = f32> + Send + 'static, duration: Duration) {
    let stream_handle = rodio::OutputStreamBuilder::open_default_stream().unwrap();
    let output_sink = stream_handle.mixer();
    output_sink.add(source);
    std::thread::sleep(duration);
}
