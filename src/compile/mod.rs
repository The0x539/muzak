// This module is taken from a preceding "bells" project.
// Its craftsmanship is inferior to that of the rest of muzak-rs.
// Perhaps one day its data structures shall be unified with those of the parsing/playback phase.

use std::collections::HashMap;

use musicxml::datatypes::*;
use musicxml::elements::*;

pub fn compile(xml: &str, padding: u8, rotation: u8) -> String {
    let data = xml.as_bytes().to_vec(); // ¯\_(ツ)_/¯
    let mxml = musicxml::read_score_data_partwise(data).unwrap();

    let mut state = State::default();
    let mut score = state.score(&mxml);

    for part in &mut score.parts {
        part.apply_transpose();
    }

    let divisions = score.unify_divisions();
    score.fix_carryover_chords();

    // I have no idea how right or wrong this is.
    score.bpm = (score.bpm * divisions) / 4;

    for _ in 0..padding {
        score.parts.insert(0, empty_part());
    }
    for _ in 0..rotation {
        score.parts.rotate_right(1);
    }

    score.to_string()
}

fn empty_part() -> output::Part {
    let mut part = output::Part::default();
    part.measures.push(Default::default());
    part.measures[0].events.push(Default::default());
    part
}

mod extensions;
use extensions::*;

mod output;

#[derive(Default)]
struct State {
    bpm: Option<(f64, u32)>,
    score: output::Score,
    repeat_start: usize,
    first_ending_length: usize,
    // Associates measure numbers with ending numbers.
    // The purpose of this is to persist across parts,
    // as only the first part seems to actually get the volta,
    // even though it affects all the parts.
    volta_memory: HashMap<String, String>,
}

impl State {
    fn score(&mut self, score: &ScorePartwise) -> output::Score {
        for part in &score.content.part {
            self.part(part);
        }
        self.score.bpm = self.bpm.unwrap_or((0.25, 120)).1; // TODO: clean this up
        std::mem::take(&mut self.score)
    }

    fn part(&mut self, part: &Part) {
        self.score.add_part();
        self.repeat_start = 0;
        self.first_ending_length = 0;

        for element in &part.content {
            match &element {
                PartElement::Measure(m) => self.measure(m),
                _ => {}
            };
        }
    }

    fn measure(&mut self, measure: &Measure) {
        self.score.add_measure();
        for element in &measure.content {
            match element {
                MeasureElement::Direction(d) => self.direction(d),
                MeasureElement::Note(n) => self.note(n),
                MeasureElement::Attributes(a) => {
                    if let Some(divisions) = &a.content.divisions {
                        self.score.last_measure().divisions = Some(divisions.content.0);
                    }

                    for transpose in &a.content.transpose {
                        assert!(
                            self.score.last_part().transpose.is_none(),
                            "multiple transposes on one part"
                        );

                        self.score.last_part().transpose = Some(output::Transpose {
                            octave: transpose
                                .content
                                .octave_change
                                .map_or(0, |x| x.content.into()),
                            chromatic: transpose.content.chromatic.content.0,
                        })
                    }
                }
                MeasureElement::Backup(..) => break,
                MeasureElement::Barline(b) => self.barline(b, &measure.attributes.number.0),
                _ => {}
            }
        }
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

    fn barline(&mut self, barline: &Barline, measure_number: &str) {
        if let Some(ending) = &barline.content.ending {
            self.volta_memory.insert(
                measure_number.to_owned(),
                ending.attributes.number.0.clone(),
            );
        }

        if barline.attributes.location == Some(RightLeftMiddle::Left) {
            if let Some(repeat) = &barline.content.repeat {
                if repeat.attributes.direction == BackwardForward::Forward {
                    self.repeat_start = self.score.last_part().measures.len() - 1;
                }
            }
        } else if barline.attributes.location == Some(RightLeftMiddle::Right) {
            if self
                .volta_memory
                .get(measure_number)
                .is_some_and(|n| n == "1")
            {
                self.first_ending_length += 1;
            }

            if let Some(repeat) = &barline.content.repeat {
                if repeat.attributes.direction == BackwardForward::Backward {
                    self.score
                        .last_part()
                        .measures
                        .extend_from_within(self.repeat_start..);

                    // limited support for volta brackets
                    // assumes only one repetition and nothing funky
                    for _ in 0..self.first_ending_length {
                        self.score.last_part().measures.pop();
                    }
                    self.first_ending_length = 0;
                }
            }
        }
    }

    fn note(&mut self, note: &Note) {
        let NoteType::Normal(info) = &note.content.info else {
            println!("eep, non-normal note");
            return;
        };

        let duration = info.duration.content.0;

        let AudibleType::Pitch(pitch) = info.audible else {
            assert!(matches!(info.audible, AudibleType::Rest(..)));
            self.score.last_measure().push_event(output::Event {
                duration,
                notes: vec![],
                staccato: false,
            });
            return;
        };

        let output_note = output::Note {
            step: pitch.content.step.content,
            semitone: pitch.content.alter.map_or(0, |a| a.content.0),
            octave: pitch.content.octave.content.0.into(),
        };

        if info.tie.get(0).map(|t| t.attributes.r#type) == Some(StartStop::Stop) {
            // just gonna assume no tied staccato notes for now

            if let Some(prev) = self.score.last_measure().events.last_mut() {
                assert!(
                    prev.notes.contains(&output_note),
                    "new note introduced at end of tie",
                );
                if info.chord.is_none() {
                    prev.duration += duration;
                }
            } else {
                // This is the first note in the measure,
                // and it's tied to the last note of the previous measure.
                if info.chord.is_none() {
                    // "Carryover" is mostly an artifact of an earlier design,
                    // but it does still allow us to put the carryover tildes on the new measure's line.
                    self.score.last_measure().carryover += duration;
                }
            }
            return;
        }

        if info.chord.is_some() {
            let prev = self.score.last_measure().last_event();
            assert_eq!(
                prev.duration, duration,
                "chord with notes of different duration"
            );
            prev.notes.push(output_note);
        } else {
            self.score.last_measure().push_event(output::Event {
                duration,
                notes: vec![output_note],
                staccato: false,
            });
        }

        let mut articulations = note
            .content
            .notations
            .iter()
            .flat_map(|n| &n.content.notations)
            .filter_map(|t| match t {
                NotationContentTypes::Articulations(a) => Some(a),
                _ => None,
            })
            .flat_map(|a| &a.content);

        if articulations.any(|a| matches!(a, ArticulationsType::Staccato(_))) {
            self.score.last_measure().last_event().staccato = true;
        }
    }
}
