// This module is taken from a preceding "bells" project.
// Its craftsmanship is inferior to that of the rest of muzak-rs.
// Perhaps one day its data structures shall be unified with those of the parsing/playback phase.

use musicxml::datatypes::*;
use musicxml::elements::*;

pub fn compile(xml: &str, padding: u8, rotation: u8) -> String {
    let data = xml.as_bytes().to_vec(); // ¯\_(ツ)_/¯
    let mxml = musicxml::read_score_data_partwise(data).unwrap();

    let mut state = State::default();
    let mut score = state.score(&mxml);

    score.unify_divisions();
    score.fix_carryover_chords();

    for _ in 0..padding {
        score.parts.insert(0, empty_part());
    }
    for _ in 0..rotation {
        score.parts.rotate_right(1);
    }

    score.to_string()
}

fn empty_part() -> output::Part {
    output::Part {
        measures: vec![output::Measure {
            events: vec![output::Event {
                duration: 1,
                notes: vec![],
            }],
            ..Default::default()
        }],
    }
}

mod extensions;
use extensions::*;

mod output;

#[derive(Default)]
struct State {
    bpm: Option<(f64, u32)>,
    tie_already_used_this_note: bool,
}

impl State {
    fn score(&mut self, score: &ScorePartwise) -> output::Score {
        let mut output = output::Score::default();
        for part in &score.content.part {
            output.parts.push(self.part(part));
        }
        // TODO: get rid of this "state" altogether.
        // problem is that bpm is specified with the measure, not the score,
        // but we care about it score-wide
        output.bpm = self.bpm.unwrap_or((0.25, 120)).1;
        output
    }

    fn part(&mut self, part: &Part) -> output::Part {
        let mut output = output::Part::default();
        for element in &part.content {
            match &element {
                PartElement::Measure(m) => output.measures.push(self.measure(m)),
                _ => {}
            };
        }
        output
    }

    fn measure(&mut self, measure: &Measure) -> output::Measure {
        let mut output = output::Measure::default();
        for element in &measure.content {
            match element {
                MeasureElement::Direction(d) => self.direction(d),
                MeasureElement::Note(n) => self.note(n, &mut output),
                MeasureElement::Attributes(a) => {
                    if let Some(divisions) = &a.content.divisions {
                        output.divisions = Some(divisions.content.0);
                    }
                }
                MeasureElement::Backup(..) => break,
                _ => {}
            }
        }
        output
    }

    fn direction(&mut self, direction: &Direction) {
        for ty in &direction.content.direction_type {
            match &ty.content {
                DirectionTypeContents::Metronome(m) => self.metronome(m),
                _ => {}
            }
        }
    }

    fn metronome(&mut self, metronome: &Metronome) {
        match &metronome.content {
            MetronomeContents::BeatBased(beat) => {
                let unit = beat.beat_unit.content;
                let count = match &beat.equals {
                    BeatEquation::BPM(bpm) => bpm.content.parse::<u32>().unwrap(),
                    e => panic!("unhandled beat equation: {e:?}"),
                };
                assert!(self.bpm.is_none());
                self.bpm = Some((unit.as_float(), count));
            }
            m => panic!("unhandled metronome type {m:?}"),
        }
    }

    fn note(&mut self, note: &Note, output: &mut output::Measure) {
        let NoteType::Normal(info) = &note.content.info else {
            println!("eep, non-normal note");
            return;
        };

        let duration = info.duration.content.0;

        let AudibleType::Pitch(pitch) = info.audible else {
            assert!(matches!(info.audible, AudibleType::Rest(..)));
            output.events.push(output::Event {
                duration,
                notes: vec![],
            });
            return;
        };

        let output_note = output::Note {
            step: pitch.content.step.content,
            semitone: pitch.content.alter.map_or(0, |a| a.content.0),
            octave: pitch.content.octave.content.0,
        };

        if info.tie.len() > 0 && info.tie[0].attributes.r#type == StartStop::Stop {
            assert_eq!(info.tie.len(), 1);
            let Some(prev) = output.events.last_mut() else {
                // This is the first note in the measure,
                // and it's tied to the last note of the previous measure.
                if !self.tie_already_used_this_note {
                    output.carryover += duration;
                    self.tie_already_used_this_note = true;
                }
                return;
            };
            assert!(
                prev.notes.contains(&output_note),
                "tie between different notes",
            );
            if !self.tie_already_used_this_note {
                prev.duration += duration;
                self.tie_already_used_this_note = true;
            }
            return;
        } else {
            self.tie_already_used_this_note = false;
        }

        if info.chord.is_some() {
            let prev = output.events.last_mut().unwrap();
            assert_eq!(prev.duration, duration);
            prev.notes.push(output_note);
            return;
        }

        output.events.push(output::Event {
            duration,
            notes: vec![output_note],
        });
    }
}
