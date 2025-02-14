use strum::VariantArray;

#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct Score {
    pub bpm: Option<u32>,
    pub parts: Vec<Vec<Event>>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Event {
    Rest,
    Note(Note),
    Chord(Vec<Note>),
}

#[derive(Debug, Default, Copy, Clone, PartialEq, Eq)]
pub struct Note {
    pub base: BaseNote,
    pub accidental: Option<Accidental>,
    pub octave: Option<i8>,
    pub duration: usize,
}

#[derive(Debug, Default, Copy, Clone, PartialEq, Eq, VariantArray)]
pub enum Accidental {
    Flat,
    #[default]
    Natural,
    Sharp,
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
