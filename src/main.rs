use std::time::Duration;

use rodio::Source;
use winnow::Parser;

pub mod fmt;
pub mod output;
pub mod parse;
pub mod types;

fn main() {
    let path = dirs::audio_dir().unwrap().join("obra dinn.txt");
    let buffer = std::fs::read_to_string(path).unwrap();
    let mut input = &*buffer;

    let score = parse::score.parse(&mut input).unwrap();

    let beat_duration = Duration::from_secs(15) / score.bpm.unwrap_or(75);

    let mut part_sources = vec![];
    for (i, part) in score.parts.iter().enumerate() {
        let part: Box<dyn Source<Item = f32> + Send + Sync> = match i {
            0 => Box::new(part.to_source(output::instruments::beep, beat_duration)),
            _ => Box::new(part.to_source(output::instruments::keyboard, beat_duration)),
        };
        part_sources.push(part);
    }

    let song_duration = beat_duration * score.duration();

    let n = part_sources.len();
    let song_source = output::Chord {
        notes: part_sources,
    }
    .take_duration(song_duration)
    .amplify(n as f32);

    if true {
        let samples = song_source.collect::<Vec<_>>();
        wavers::write("./obra dinn.wav", &samples, 48000, 1).unwrap();
    } else {
        let stream_handle = rodio::OutputStreamBuilder::open_default_stream().unwrap();
        let sink = rodio::Sink::connect_new(&stream_handle.mixer());
        sink.append(song_source);
        std::thread::sleep(song_duration);
    }
}
