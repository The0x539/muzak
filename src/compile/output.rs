use std::fmt::{Display, Formatter, Write};

use musicxml::datatypes::Step;
use strum::{EnumCount, IntoStaticStr, VariantArray};

use crate::types::Instrument;

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

                for event in measure.events_mut() {
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

                if cur.carryover == 0 {
                    // not a tied note, so we don't care
                    continue;
                }

                let prev_event = prev.last_event();
                if prev_event.notes.len() <= 1 {
                    // not a chord, so we don't care
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
    pub transpose: Option<Transpose>,
    pub instrument: Option<Instrument>,
    pub measures: Vec<Measure>,
}
impl Part {
    pub fn add_measure(&mut self) {
        self.measures.push(Default::default())
    }

    pub fn last_measure(&mut self) -> &mut Measure {
        self.measures.last_mut().unwrap()
    }

    pub fn apply_transpose(&mut self) {
        let Some(transpose) = self.transpose.take() else {
            return;
        };

        for note in self
            .measures
            .iter_mut()
            .flat_map(|m| m.events_mut())
            .flat_map(|e| &mut e.notes)
        {
            note.octave += transpose.octave;
            note.semitone += transpose.chromatic;
            *note = note.normalized();
        }
    }
}

#[derive(Debug, Default, Copy, Clone)]
pub struct Transpose {
    pub chromatic: i16,
    pub octave: i16,
}

#[derive(Debug, Default, Clone)]
pub struct Measure {
    /// The duration of the note at the start of the measure,
    /// if that note is tied to the end of the previous measure.
    pub carryover: u32,
    pub items: Vec<MeasureItem>,
    pub divisions: Option<u32>,
}

impl Measure {
    pub fn events(&self) -> impl DoubleEndedIterator<Item = &Event> {
        self.items.iter().filter_map(|item| match item {
            MeasureItem::Event(event) => Some(event),
            _ => None,
        })
    }

    pub fn events_mut(&mut self) -> impl DoubleEndedIterator<Item = &mut Event> {
        self.items.iter_mut().filter_map(|item| match item {
            MeasureItem::Event(event) => Some(event),
            _ => None,
        })
    }

    pub fn last_event(&mut self) -> &mut Event {
        self.try_last_event().unwrap()
    }

    pub fn try_last_event(&mut self) -> Option<&mut Event> {
        self.events_mut().next_back()
    }

    pub fn push_event(&mut self, event: Event) {
        self.items.push(MeasureItem::Event(event))
    }
}

#[derive(Debug, Clone)]
pub enum MeasureItem {
    Event(Event),
    Dynamic(Dynamic),
}

#[derive(Debug, Default, Copy, Clone, PartialEq, Eq, IntoStaticStr, VariantArray, EnumCount)]
pub enum Dynamic {
    #[strum(to_string = "𝓹𝓹")]
    Pianissimo,
    #[strum(to_string = "𝓹")]
    Piano,
    #[strum(to_string = "𝓶𝓹")]
    MezzoPiano,
    #[default]
    #[strum(to_string = "𝓶𝓯")]
    MezzoForte,
    #[strum(to_string = "𝓯")]
    Forte,
    #[strum(to_string = "𝓯𝓯")]
    Fortissimo,
}

#[derive(Debug, Clone)]
pub struct Event {
    pub duration: u32,
    pub notes: Vec<Note>,
    pub staccato: bool,
}

impl Default for Event {
    fn default() -> Self {
        Self {
            duration: 1,
            notes: vec![],
            staccato: false,
        }
    }
}

#[derive(Debug, PartialEq, Eq, Copy, Clone)]
pub struct Note {
    pub step: Step,
    pub semitone: i16,
    pub octave: i16,
}

impl Note {
    pub fn normalized(mut self) -> Self {
        const STEPS: [Step; 7] = [
            Step::A,
            Step::B,
            Step::C,
            Step::D,
            Step::E,
            Step::F,
            Step::G,
        ];

        let mut step_up = STEPS;
        step_up.rotate_left(1);

        let mut step_down = STEPS;
        step_down.rotate_right(1);

        loop {
            let max_sharps = match self.step {
                Step::B | Step::E => 0,
                _ => 1,
            };
            let max_flats = match self.step {
                Step::C | Step::F => 0,
                _ => -1,
            };

            if self.semitone > max_sharps {
                self.semitone -= 1;
                self.step = step_up[self.step as usize];
                if self.step == Step::C {
                    self.octave += 1;
                }
            } else if self.semitone < max_flats {
                self.semitone += 1;
                self.step = step_down[self.step as usize];
                if self.step == Step::B {
                    self.octave -= 1;
                }
            } else {
                break self;
            }
        }
    }
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

impl Display for MeasureItem {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            MeasureItem::Event(x) => x.fmt(f),
            MeasureItem::Dynamic(x) => f.write_str(x.into()),
        }
    }
}

impl Display for Measure {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        for _ in 0..self.carryover {
            f.write_char('~')?;
        }

        if self.events().all(|e| e.notes.is_empty()) {
            let dur: u32 = self.events().map(|e| e.duration).sum();
            write!(f, "🛏{dur}")?;
            return Ok(());
        }

        for (i, item) in self.items.iter().enumerate() {
            if i > 0 || self.carryover > 0 {
                f.write_char(' ')?;
            }
            write!(f, "{item}")?;
        }
        Ok(())
    }
}

impl Display for Part {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        if let Some(instrument) = self.instrument {
            writeln!(f, "{}", instrument)?;
        }
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
