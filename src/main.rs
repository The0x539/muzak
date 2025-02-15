use std::time::Duration;

use rodio::Source;
use winnow::Parser;

pub mod fmt;
pub mod output;
pub mod parse;
pub mod types;

fn main() {
    let path = dirs::audio_dir().unwrap().join("balatro.txt");
    let buffer = std::fs::read_to_string(path)
        .unwrap()
        .split_whitespace()
        .collect::<String>();

    let mut input = &*buffer;

    let score = parse::score.parse(&mut input).unwrap();
    let beat_duration = Duration::from_secs(15) / score.bpm.unwrap_or(75);

    let part_sources = score
        .parts
        .iter()
        .map(|part| part.to_source(output::instruments::sine, beat_duration))
        .collect::<Vec<_>>();

    let n = part_sources.len();
    let song_source = output::Chord {
        notes: part_sources,
    }
    .amplify(n as f32);

    if false {
        let samples = song_source.collect::<Vec<_>>();
        wavers::write("./balatro.wav", &samples, 48000, 1).unwrap();
    } else {
        let stream_handle = rodio::OutputStreamBuilder::open_default_stream().unwrap();
        let sink = rodio::Sink::connect_new(&stream_handle.mixer());
        sink.append(song_source);
        std::thread::sleep(beat_duration * score.duration());
    }
}
