use std::fmt::{Display, Formatter, Write};

use musicxml::datatypes::Step;

#[derive(Debug, Default, Clone)]
pub struct Score {
    pub parts: Vec<Part>,
    pub bpm: u32,
}

impl Score {
    pub fn unify_divisions(&mut self) -> u32 {
        let max = self
            .parts
            .iter()
            .flat_map(|p| &p.measures)
            .filter_map(|m| m.divisions)
            .max()
            .unwrap();

        for part in &mut self.parts {
            let mut ratio = max; // assume the default divisions count is 1
            for measure in &mut part.measures {
                if let Some(n) = &mut measure.divisions {
                    assert_eq!(max % *n, 0);
                    ratio = max / *n;
                    *n = max;
                };

                for event in &mut measure.events {
                    event.duration *= ratio;
                }
            }
        }

        max
    }

    pub fn fix_carryover_chords(&mut self) {
        for part in &mut self.parts {
            for i in 1..part.measures.len() {
                let [prev, cur] = &mut part.measures[i - 1..=i] else {
                    unreachable!()
                };

                let prev_event = prev.events.last_mut().unwrap();

                if prev_event.notes.len() <= 1 {
                    // not a chord, so we don't care
                    continue;
                }

                if cur.carryover == 0 {
                    // not a tied note, so we don't care
                    continue;
                }

                prev_event.duration += cur.carryover;
                cur.carryover = 0;
            }
        }
    }

    pub fn add_part(&mut self) {
        self.parts.push(Default::default())
    }

    pub fn last_part(&mut self) -> &mut Part {
        self.parts.last_mut().unwrap()
    }

    pub fn last_measure(&mut self) -> &mut Measure {
        self.last_part().last_measure()
    }

    pub fn add_measure(&mut self) {
        self.last_part().add_measure()
    }
}

#[derive(Debug, Default, Clone)]
pub struct Part {
    pub measures: Vec<Measure>,
}

impl Part {
    pub fn add_measure(&mut self) {
        self.measures.push(Default::default())
    }

    pub fn last_measure(&mut self) -> &mut Measure {
        self.measures.last_mut().unwrap()
    }
}

#[derive(Debug, Default, Clone)]
pub struct Measure {
    /// The duration of the note at the start of the measure,
    /// if that note is tied to the end of the previous measure.
    pub carryover: u32,
    pub events: Vec<Event>,
    pub divisions: Option<u32>,
}

impl Measure {
    pub fn last_event(&mut self) -> &mut Event {
        self.events.last_mut().unwrap()
    }

    pub fn push_event(&mut self, event: Event) {
        self.events.push(event)
    }
}

#[derive(Debug, Clone)]
pub struct Event {
    pub duration: u32,
    pub notes: Vec<Note>,
    pub staccato: bool,
}

#[derive(Debug, PartialEq, Eq, Copy, Clone)]
pub struct Note {
    pub step: Step,
    pub semitone: i16,
    pub octave: u8,
}

impl Display for Note {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        const STEPS: [char; 7] = ['A', 'B', 'C', 'D', 'E', 'F', 'G'];

        let mut step_index = self.step as usize;
        let octave = self.octave;
        let mut sharp = false;

        match self.semitone {
            0 => {}
            1 => sharp = true,
            -1 => {
                sharp = true;
                // The octave changes when going from B to C, and there is no C♭
                step_index = step_index.checked_sub(1).unwrap_or(6);
            }
            _ => todo!(),
        }

        let mut step = STEPS[step_index];
        if octave >= 5 {
            step = step.to_ascii_lowercase();
        }

        f.write_char(step)?;
        if sharp {
            f.write_char('#')?;
        }

        if !matches!(octave, 4 | 5) {
            f.write_char(char::from_digit(octave as u32, 10).unwrap())?;
        }

        Ok(())
    }
}

impl Display for Event {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        let mut duration = self.duration;
        if self.staccato {
            assert_eq!(
                duration % 2,
                0,
                "TODO: get staccato working in cases like this"
            );
            duration /= 2;
        }
        match &self.notes[..] {
            [/* rest */] => {
                for _ in 0..duration {
                    f.write_char('/')?;
                }
            }
            [note] => {
                write!(f, "{note}")?;
                for _ in 1..duration {
                    f.write_char('~')?;
                }
            }
            chord => {
                f.write_char('[')?;
                for note in chord {
                    write!(f, "{note}")?;
                }
                for _ in 1..duration {
                    f.write_char('~')?;
                }
                f.write_char(']')?;
            }
        }
        if self.staccato {
            for _ in 0..duration {
                f.write_char('/')?;
            }
        }
        Ok(())
    }
}

impl Display for Measure {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        for _ in 0..self.carryover {
            f.write_char('~')?;
        }
        for (i, event) in self.events.iter().enumerate() {
            if i > 0 || self.carryover > 0 {
                f.write_char(' ')?;
            }
            write!(f, "{event}")?;
        }
        Ok(())
    }
}

impl Display for Part {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        for (i, measure) in self.measures.iter().enumerate() {
            if i > 0 {
                f.write_char('\n')?;
            }
            write!(f, "{measure}")?;
        }
        Ok(())
    }
}

impl Display for Score {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "{}", self.bpm)?;
        for (i, part) in self.parts.iter().enumerate() {
            if i > 0 {
                f.write_str("\n|\n")?;
            }
            write!(f, "{part}")?;
        }
        Ok(())
    }
}
