use std::time::Duration;

use strum::VariantArray;

#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct Score {
    pub bpm: Option<u32>,
    pub parts: Vec<Part>,
}

impl Score {
    pub fn beat_count(&self) -> u32 {
        self.parts.iter().map(|p| p.beat_count()).max().unwrap_or(1)
    }

    pub fn beat_duration(&self) -> Duration {
        Duration::from_secs(15) / self.bpm.unwrap_or(75)
    }
}

#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct Part {
    pub instrument: Option<Instrument>,
    pub events: Vec<Event>,
}

impl Part {
    pub fn beat_count(&self) -> u32 {
        self.events.iter().map(|e| e.beat_count()).sum()
    }
}

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum Instrument {
    Beep,
    Keyboard,
    Bell,
    Waterphone,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Event {
    Rest,
    Note(Note),
    Chord(Vec<Note>),
}

impl Event {
    pub fn notes(&self) -> &[Note] {
        match self {
            Self::Rest => &[],
            Self::Note(note) => std::slice::from_ref(note),
            Self::Chord(notes) => notes,
        }
    }

    pub fn beat_count(&self) -> u32 {
        self.notes().iter().map(|n| n.duration).max().unwrap_or(1)
    }
}

#[derive(Debug, Default, Copy, Clone, PartialEq, Eq)]
pub struct Note {
    pub base: BaseNote,
    pub accidental: Option<Accidental>,
    pub octave: Option<i8>,
    pub duration: u32,
}

impl Note {
    pub fn octave(&self) -> i8 {
        if let Some(n) = self.octave {
            n
        } else if self.base.high {
            5
        } else {
            4
        }
    }

    pub fn to_half_step(&self) -> i16 {
        let octave = 12 * i16::from(self.octave() - 4);
        let tone = -9 + i16::from(self.base.note.chromatic_index());
        // yeah sure let's allow B♯ and F♭ and what not, what could go wrong?
        let semitone = self.accidental.unwrap_or_default() as i16;
        octave + tone + semitone
    }

    pub fn to_frequency(&self) -> f32 {
        let step = self.to_half_step() as f32;
        440.0 * f32::powf(2.0, step / 12.0)
    }
}

#[repr(i8)]
#[derive(Debug, Default, Copy, Clone, PartialEq, Eq, VariantArray)]
pub enum Accidental {
    Flat = -1,
    #[default]
    Natural = 0,
    Sharp = 1,
}

#[derive(Debug, Default, Copy, Clone, PartialEq, Eq)]
pub struct BaseNote {
    pub note: Diatonic,
    pub high: bool,
}

#[derive(Debug, Default, Copy, Clone, PartialEq, Eq, PartialOrd, Ord, VariantArray)]
pub enum Diatonic {
    A,
    B,
    #[default]
    C,
    D,
    E,
    F,
    G,
}

impl Diatonic {
    pub fn chromatic_index(&self) -> i8 {
        match self {
            Self::C => 0,
            // C# => 1,
            Self::D => 2,
            // D# => 3,
            Self::E => 4,
            Self::F => 5,
            // F# => 6,
            Self::G => 7,
            // G# => 8,
            Self::A => 9,
            // A# => 10,
            Self::B => 11,
        }
    }
}
