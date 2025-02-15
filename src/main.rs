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

    let mut part_sources = vec![];

    for part in &score.parts {
        let mut part_duration = Duration::from_secs(0);
        let mut event_sources = vec![];

        for event in part {
            let mut chord = output::Chord { notes: vec![] };
            let mut num_beats = 1;

            for note in event.notes() {
                num_beats = num_beats.max(note.duration as u32);
                let freq = note.to_frequency();
                chord.notes.push(rodio::source::SineWave::new(freq));
            }

            let event_duration = num_beats * beat_duration;
            part_duration += event_duration;
            event_sources.push(chord.take_duration(event_duration));
        }

        part_sources.push(rodio::source::from_iter(event_sources).amplify(0.2));
    }

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
        sink.sleep_until_end();
    }
}
