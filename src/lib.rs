use std::{num::NonZeroUsize, time::Duration};

use rodio::Source;
use types::Score;

pub use rodio;

mod compile;
mod fmt;
pub mod parse;

pub mod instruments;
pub mod output;
#[macro_use]
pub mod types;

pub use compile::compile;

use crate::types::Quaver;

type Result<T, E = Box<dyn std::error::Error + Send + Sync>> = std::result::Result<T, E>;

type ParseError<'a> = winnow::error::ParseError<&'a str, winnow::error::ContextError>;

pub fn parse(song_text: &str) -> Result<types::Score, ParseError<'_>> {
    winnow::Parser::parse(&mut parse::score, song_text)
}

#[derive(Debug, Copy, Clone)]
pub struct MixOptions {
    pub max_duration: Option<Duration>,
    pub max_tracks: Option<NonZeroUsize>,
    pub volume: f32,
}

pub fn mix(score: &Score, options: MixOptions) -> (impl Source<Item = f32> + 'static, Duration) {
    let beat = score.initial_tempo().duration_of(Quaver::Quarter);

    let mut mixer = output::Chord::new();

    let track_limit = options.max_tracks.map_or(usize::MAX, |n| n.get());

    for (i, part) in score.parts.iter().enumerate().take(track_limit) {
        let instrument = match part.instrument {
            Some(x) => x,
            None if i == 0 => types::Instrument::Beep,
            None => types::Instrument::Keyboard,
        };
        let play_fn = match_instrument!(instrument, |T| T::play_part);
        mixer.add(play_fn(part, beat));
    }

    let true_duration = score.duration();
    let mixed = mixer.amplify(options.volume).take_duration(true_duration);
    (mixed, true_duration)
}

pub fn play(source: impl Source<Item = f32> + Send + 'static) {
    let mut sink_handle = rodio::DeviceSinkBuilder::open_default_sink().unwrap();
    let player = rodio::Player::connect_new(&sink_handle.mixer());
    player.append(source);
    player.sleep_until_end();
    sink_handle.log_on_drop(false);
    std::thread::sleep(std::time::Duration::from_millis(500));
}
