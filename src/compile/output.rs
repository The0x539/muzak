use std::fmt::{Display, Formatter, Write};

use musicxml::datatypes::Step;
use strum::{Display, EnumCount, IntoStaticStr, VariantArray};

use crate::types::Instrument;

#[derive(Debug, Default, Clone)]
pub struct Score {
    pub parts: Vec<Part>,
    pub bpm: Option<u32>,
}

impl Score {
    pub fn cleanup(&mut self) {
        self.parts.retain(|p| !p.is_empty());

        for part in &mut self.parts {
            part.apply_transpose();
        }

        self.adjust_metronomes();
        self.fix_carryover_chords();
        self.fix_hyper_staccato();
    }

    fn adjust_metronomes(&mut self) {
        if self.parts.is_empty() {
            return;
        }

        // Confirm a rather convenient assumption about how measures are divided.
        // If someone has a score that can contradict this assumption, I'll fix it,
        // but this will rather annoying.
        let divisions = self.parts[0].measures[0].divisions.unwrap();
        for part in &self.parts {
            // Make It Through fails this for some reason
            //assert_eq!(part.measures.len(), self.parts[0].measures.len());
            assert_eq!(part.measures[0].divisions, Some(divisions));
            for measure in &part.measures[1..] {
                assert!(matches!(measure.divisions, None | Some(4)));
            }
        }

        if let Some(m) = self.parts[0].measures[0].metronome.take() {
            self.bpm = Some((divisions as f32 * m.beat as f32 * m.note_value.to_f32()) as u32);
        }

        for i in 1..self.parts.len() {
            let [src_part, dst_part] = &mut self.parts[i - 1..=i] else {
                unreachable!()
            };
            for (m1, m2) in std::iter::zip(&mut src_part.measures, &mut dst_part.measures) {
                m2.metronome = m2.metronome.or(m1.metronome);
            }
        }

        for part in &mut self.parts {
            for measure in &mut part.measures {
                if let Some(metronome) = &mut measure.metronome {
                    metronome.beat *= divisions;
                }
            }
        }
    }

    // yeah just double everything, maybe later we can double the specific bits that need it
    fn fix_hyper_staccato(&mut self) {
        let events = self
            .parts
            .iter_mut()
            .flat_map(|p| &mut p.measures)
            .flat_map(|m| &mut m.items)
            .filter_map(|i| i.as_event_mut())
            .collect::<Vec<_>>();

        if !events.iter().any(|e| e.staccato && e.duration % 2 != 0) {
            return;
        }

        events.into_iter().for_each(|e| e.duration *= 2);

        if let Some(n) = &mut self.bpm {
            *n *= 2;
        }

        for part in &mut self.parts {
            for measure in &mut part.measures {
                measure.carryover *= 2;
                if let Some(metronome) = &mut measure.metronome {
                    metronome.beat *= 2;
                }
            }
        }
    }

    fn fix_carryover_chords(&mut self) {
        for part in &mut self.parts {
            for i in 1..part.measures.len() {
                let [prev, cur] = &mut part.measures[i - 1..=i] else {
                    unreachable!()
                };

                if cur.carryover == 0 {
                    // not a tied note, so we don't care
                    continue;
                }

                let Some(prev_event) = prev.try_last_event() else {
                    // assume it's a three-measure tie.
                    // more than that? idk, stop doing that
                    if i > 2 {
                        let carryover = std::mem::take(&mut cur.carryover);
                        let prev_prev = &mut part.measures[i - 2];
                        let prev_event = prev_prev.last_event();
                        assert!(
                            prev_event.notes.len() >= 1,
                            "unsupported: three-measure non-chord?"
                        );
                        prev_event.duration += carryover;
                    }

                    continue;
                };

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
    pub fn is_empty(&self) -> bool {
        self.measures.iter().all(|m| m.is_empty())
    }

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
    pub metronome: Option<Metronome>,
}

impl Measure {
    pub fn is_empty(&self) -> bool {
        self.carryover == 0 && self.events().all(|i| i.is_rest())
    }

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

    pub fn duration(&self) -> u32 {
        self.carryover + self.events().map(|e| e.duration).sum::<u32>()
    }

    pub fn push_event(&mut self, duration: u32) -> &mut Event {
        self.items.push(MeasureItem::Event(Event {
            duration,
            ..Default::default()
        }));
        match self.items.last_mut() {
            Some(MeasureItem::Event(event)) => event,
            _ => unreachable!(),
        }
    }
}

#[derive(Debug, Clone)]
pub enum MeasureItem {
    Event(Event),
    Dynamic(Dynamic),
}

impl MeasureItem {
    fn as_event_mut(&mut self) -> Option<&mut Event> {
        match self {
            Self::Event(e) => Some(e),
            _ => None,
        }
    }
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

#[derive(Debug, Display, Copy, Clone, PartialEq, Eq, IntoStaticStr, VariantArray, EnumCount)]
pub enum Quaver {
    #[strum(to_string = "𝅜")]
    Double,
    #[strum(to_string = "𝅝")]
    Whole,
    #[strum(to_string = "𝅗𝅥")]
    Half,
    #[strum(to_string = "𝅘𝅥")]
    Quarter,
    #[strum(to_string = "𝅘𝅥𝅮")]
    Eighth,
    #[strum(to_string = "𝅘𝅥𝅯")]
    Sixteenth, // MuseScore Studio 4.6 already stops at eighths for metronome marks
}

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub struct Metronome {
    pub note_value: Quaver,
    pub beat: u32,
}

impl Display for Metronome {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}={}", self.note_value, self.beat)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Event {
    pub duration: u32,
    pub notes: Vec<Note>,
    pub staccato: bool,
}

impl Event {
    pub fn is_rest(&self) -> bool {
        self.notes.is_empty()
    }
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
        let mut octave = self.octave;
        let mut accidental = None;

        if super::legacy_semitones() {
            match self.semitone {
                0 => {}
                1 => accidental = Some('#'),
                -1 => {
                    // go down to the next letter no matter what
                    step_index = step_index.checked_sub(1).unwrap_or(6);

                    // C♭ = B♮; F♭ = E♮; otherwise a flat is equivalent to the previous sharp
                    if !matches!(self.step, Step::C | Step::F) {
                        accidental = Some('#');
                    }
                    // B-C is where the octave transition happens, so C♭5 = B♮4
                    if self.step == Step::C {
                        octave -= 1;
                    }
                }
                _ => todo!(),
            }
        } else {
            accidental = match self.semitone {
                0 => None,
                -1 => Some('♭'),
                1 => Some('♯'),
                _ => todo!(),
            }
        }

        let mut step = STEPS[step_index];
        if octave >= 5 {
            step = step.to_ascii_lowercase();
        }

        f.write_char(step)?;
        if let Some(ch) = accidental {
            f.write_char(ch)?;
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
        if self.metronome.is_some() && self.carryover > 0 {
            eprintln!(
                "warning: measure has both carryover and a metronome marking. this may end poorly."
            )
        }

        for _ in 0..self.carryover {
            f.write_char('~')?;
        }

        if let Some(m) = &self.metronome {
            writeln!(f, "{m}")?;
        }

        if self.events().all(|e| e.notes.is_empty()) && self.carryover == 0 {
            write!(f, "🛏{}", self.duration())?;
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
        if let Some(bpm) = self.bpm {
            writeln!(f, "{bpm}")?;
        }
        for (i, part) in self.parts.iter().enumerate() {
            if i > 0 {
                f.write_str("\n|\n")?;
            }
            write!(f, "{part}")?;
        }
        Ok(())
    }
}
