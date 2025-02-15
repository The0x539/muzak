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

    // assert_eq!(score.parts.len(), 1);
    let part = score.parts.into_iter().nth(1).unwrap();

    let mut part_duration = Duration::from_secs(0);

    let mut sources = vec![];

    for event in &part {
        let mut chord = output::mixers::Chord { notes: vec![] };
        let mut num_beats = 1;

        for note in event.notes() {
            num_beats = num_beats.max(note.duration as u32);
            let freq = note.to_frequency();
            chord.notes.push(rodio::source::SineWave::new(freq));
        }

        let event_duration = num_beats * beat_duration;
        part_duration += event_duration;
        sources.push(chord.take_duration(event_duration));
    }

    let part_source = rodio::source::from_iter(sources).amplify(0.25);
    let (_stream, handle) = rodio::OutputStream::try_default().unwrap();
    handle.play_raw(part_source.convert_samples()).unwrap();
    std::thread::sleep(part_duration);
}
