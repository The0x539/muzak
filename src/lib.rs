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

pub fn mix(score: &Score) -> impl Source<Item = f32> + 'static {
    let beat = score.beat_duration();

    let (mixer_sink, mixer_source) = rodio::mixer::mixer(1, SAMPLE_RATE);

    for (i, part) in score.parts.iter().enumerate() {
        match i {
            0 => mixer_sink.add(part.to_source(instruments::beep, beat)),
            _ => mixer_sink.add(part.to_source(instruments::keyboard, beat)),
        }
    }

    mixer_source.take_duration(beat * score.beat_count())
}

pub fn play(score: &Score) {
    let mixer_source = mix(score);

    let stream_handle = rodio::OutputStreamBuilder::open_default_stream().unwrap();
    let output_sink = stream_handle.mixer();
    output_sink.add(mixer_source);

    std::thread::sleep(score.beat_duration() * score.beat_count());
}
