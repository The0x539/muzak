// This module is taken from a preceding "bells" project.
// Its craftsmanship is inferior to that of the rest of muzak-rs.
// Perhaps one day its data structures shall be unified with those of the parsing/playback phase.

use std::collections::BTreeSet;
use std::collections::HashMap;
use std::sync::atomic::AtomicBool;
use std::sync::atomic::Ordering;

use musicxml::datatypes::*;
use musicxml::elements::*;

static LEGACY_SEMITONES: AtomicBool = AtomicBool::new(false);

fn legacy_semitones() -> bool {
    LEGACY_SEMITONES.load(Ordering::Relaxed)
}

pub fn compile(xml: &str, padding: u8, rotation: u8, legacy_semitones: bool) -> String {
    LEGACY_SEMITONES.store(legacy_semitones, Ordering::Relaxed);

    let data = xml.as_bytes().to_vec(); // ¯\_(ツ)_/¯
    let mxml = musicxml::read_score_data_partwise(data).unwrap();

    let mut state = State::default();
    let mut score = state.score(&mxml);

    score.cleanup();

    for _ in 0..padding {
        score.parts.insert(0, empty_part());
    }
    for _ in 0..rotation {
        score.parts.rotate_right(1);
    }

    post_process(&score.to_string())
}

fn post_process(score: &str) -> String {
    let mut lines = score.lines().map(String::from).collect::<Vec<_>>();

    // Merge consecutive long-rest lines
    for i in (1..lines.len()).rev() {
        fn get_long_rest(line: &str) -> Option<u32> {
            line.strip_prefix("🛏")?.parse().ok()
        }

        let Some((a, b)) = get_long_rest(&lines[i - 1]).zip(get_long_rest(&lines[i])) else {
            continue;
        };

        lines.remove(i);
        lines[i - 1] = format!("🛏{}", a + b);
    }

    lines.join("\n")
}

fn empty_part() -> output::Part {
    let mut part = output::Part::default();
    part.measures.push(Default::default());
    part.measures[0].push_event(1);
    part
}

mod extensions;
use extensions::*;

pub(crate) mod output;

#[derive(Default)]
struct State {
    score: output::Score,
    repeat_start: usize,
    first_ending_length: usize,
    // Associates measure numbers with ending numbers.
    // The purpose of this is to persist across parts,
    // as only the first part seems to actually get the volta,
    // even though it affects all the parts.
    volta_memory: HashMap<String, String>,

    // (voice, staff) tuple, so dynamics don't get applied to the wrong staff
    current_voice: (u32, u32),
    // These reset for each part.
    voices_seen: BTreeSet<(u32, u32)>,
    voices_processed: BTreeSet<(u32, u32)>,

    // Used to keep track of a measure's longest duration across voices,
    // in order to pad a measure with a rest if the current voice doesn't appear in that measure
    measure_pos: u32,
}

impl State {
    fn score(&mut self, score: &ScorePartwise) -> output::Score {
        let part_list = score
            .content
            .part_list
            .content
            .content
            .iter()
            .filter_map(|x| match x {
                PartListElement::ScorePart(score_part) => Some(score_part),
                _ => None,
            });

        for (part, metadata) in std::iter::zip(&score.content.part, part_list) {
            let part_name = metadata.content.part_name.content.to_lowercase();
            let instrument = [
                ("beep", crate::types::Instrument::Beep),
                ("sine", crate::types::Instrument::Beep),
                ("key", crate::types::Instrument::Keyboard),
                ("bell", crate::types::Instrument::Bell),
                ("waterphone", crate::types::Instrument::Waterphone),
                ("snare", crate::types::Instrument::Drum),
                ("drum", crate::types::Instrument::Drum),
            ]
            .into_iter()
            .find(|(keyword, _)| part_name.contains(keyword))
            .map(|(_, v)| v);

            self.voices_seen.clear();
            self.voices_seen.insert((1, 1));
            self.voices_processed.clear();

            while let Some(voice) = self
                .voices_seen
                .difference(&self.voices_processed)
                .copied()
                .next()
            {
                self.current_voice = voice;
                self.part(part);
                self.score.last_part().instrument = instrument;
                self.voices_processed.insert(voice);
            }
        }
        // let (beat_unit, bpm) = self.bpm.unwrap_or((0.25, 120));
        // self.score.bpm = (bpm as f64 * 4.0 * beat_unit) as u32;

        std::mem::take(&mut self.score)
    }

    fn part(&mut self, part: &Part) {
        self.score.add_part();
        self.repeat_start = 0;
        self.first_ending_length = 0;

        for element in &part.content {
            // This matches a pattern established by other similar functions,
            // and acknowledges the potential to add more arms in the future
            #[allow(clippy::single_match)]
            match &element {
                PartElement::Measure(m) => self.measure(m),
                _ => {}
            };
        }
    }

    fn measure(&mut self, measure: &Measure) {
        self.score.add_measure();
        self.measure_pos = 0;
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
                MeasureElement::Barline(b) => self.barline(b, &measure.attributes.number.0),
                MeasureElement::Backup(b) => {
                    let duration = b.content.duration.content.0;
                    self.measure_pos -= duration;
                }
                MeasureElement::Forward(f) => {
                    let duration = f.content.duration.content.0;
                    self.measure_pos += duration;
                    self.score.last_measure().push_event(duration);
                }
                _ => {}
            }
        }

        self.postprocess_measure();
    }

    // Call just before the current final measure is about to be covered by another measure.
    // This happens when adding a new measure or when rendering a repeat.
    fn postprocess_measure(&mut self) {
        let measure = self.score.last_measure();

        if let Some(remainder @ 1..) = self.measure_pos.checked_sub(measure.duration()) {
            measure.push_event(remainder);
        }
    }

    fn direction(&mut self, direction: &Direction) {
        let staff = direction.content.staff.value();
        if staff != self.current_voice.1 {
            return;
        }

        for ty in &direction.content.direction_type {
            match &ty.content {
                DirectionTypeContents::Metronome(m) => self.metronome(m),
                DirectionTypeContents::Dynamics(d) => {
                    for dynamic in d.iter().flat_map(|d| &d.content) {
                        let output_dynamic = match dynamic {
                            DynamicsType::Pp(_) => output::Dynamic::Pianissimo,
                            DynamicsType::P(_) => output::Dynamic::Piano,
                            DynamicsType::Mp(_) => output::Dynamic::MezzoPiano,
                            DynamicsType::Mf(_) => output::Dynamic::MezzoForte,
                            DynamicsType::F(_) => output::Dynamic::Forte,
                            DynamicsType::Ff(_) => output::Dynamic::Fortissimo,
                            _ => panic!("Unsupported dynamic: {dynamic:?}"),
                        };
                        self.score
                            .last_measure()
                            .items
                            .push(output::MeasureItem::Dynamic(output_dynamic));
                    }
                }
                _ => {}
            }
        }
    }

    fn metronome(&mut self, metronome: &Metronome) {
        assert_eq!(
            self.score.last_measure().events().next(),
            None,
            "unsupported: metronome marking in the middle of a measure"
        );
        assert_eq!(
            self.score.last_measure().metronome,
            None,
            "unsupported: multiple metronome markings in one measure"
        );

        let metronome = metronome.value();
        self.score.last_measure().metronome = Some(metronome);
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
                    self.postprocess_measure();

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
            // eprintln!("eep, non-normal note: {:?}", note.content.info);
            return;
        };

        let duration = info.duration.content.0;

        if info.chord.is_none() {
            self.measure_pos += duration;
        }

        let output_note = match info.audible {
            AudibleType::Pitch(pitch) => Some(output::Note {
                step: pitch.content.step.content,
                semitone: pitch.content.alter.map_or(0, |a| a.content.0),
                octave: pitch.content.octave.content.0.into(),
            }),
            AudibleType::Unpitched(u) => Some(output::Note {
                step: u.content.display_step.content,
                semitone: 0,
                octave: u.content.display_octave.content.0.into(),
            }),
            AudibleType::Rest(..) => None,
        };

        let staff = note.content.staff.value();

        if let Some(voice_elem) = &note.content.voice {
            let voice_number: u32 = voice_elem.content.parse().unwrap();
            let voice = (voice_number, staff);

            if voice_number == 2 {
                assert_eq!(staff, 1);
            }

            self.voices_seen.insert(voice);
            if voice != self.current_voice {
                return;
            }
        }

        if info.tie.first().map(|t| t.attributes.r#type) == Some(StartStop::Stop) {
            // just gonna assume no tied staccato notes for now

            if let Some(prev) = self.score.last_measure().try_last_event() {
                assert!(
                    prev.notes
                        .contains(output_note.as_ref().expect("what is a tied rest")),
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

        let event = if info.chord.is_some() {
            let prev = self.score.last_measure().last_event();
            assert_eq!(
                prev.duration, duration,
                "chord with notes of different duration"
            );
            prev
        } else {
            self.score.last_measure().push_event(duration)
        };

        event.notes.extend(output_note);

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
            event.staccato = true;
        }
    }
}
