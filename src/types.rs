use std::time::Duration;

use strum::{Display, VariantArray};

#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct Score {
    pub bpm: Option<u32>,
    pub parts: Vec<Part>,
}

impl Score {
    pub fn initial_tempo(&self) -> Tempo {
        Tempo {
            note_value: Quaver::Whole,
            beat: self.bpm.unwrap_or(75),
        }
    }

    pub fn duration(&self) -> Duration {
        let tempo = self.initial_tempo();
        let beat = tempo.duration_of(Quaver::Quarter);
        self.parts
            .iter()
            .map(|p| p.duration(beat))
            .max()
            .unwrap_or_default()
    }
}

#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct Part {
    pub instrument: Option<Instrument>,
    pub items: Vec<PartItem>,
}

macro_rules! match_instrument {
    ($enum:expr, |$ty_var:ident| $value:expr) => {{
        use $crate::output::Instrument as _;
        match $enum {
            $crate::types::Instrument::Beep => {
                type $ty_var = $crate::instruments::Beep;
                $value
            }
            $crate::types::Instrument::Keyboard => {
                type $ty_var = $crate::instruments::Keyboard;
                $value
            }
            $crate::types::Instrument::Bell => {
                type $ty_var = $crate::instruments::Bells;
                $value
            }
            $crate::types::Instrument::Waterphone => {
                type $ty_var = $crate::instruments::Waterphone;
                $value
            }
            $crate::types::Instrument::Drum => {
                type $ty_var = $crate::instruments::Drum;
                $value
            }
        }
    }};
}

impl Part {
    pub fn duration(&self, mut beat: Duration) -> Duration {
        let mut dur = Duration::ZERO;

        for item in &self.items {
            let is_last = std::ptr::eq(item, self.items.last().unwrap());

            match item {
                PartItem::Event(event) => {
                    if is_last {
                        if let Some(i) = self.instrument {
                            match_instrument!(i, |T| if T::HAS_SUSTAIN {
                                dur += T::audio_duration(event, beat);
                                continue;
                            });
                        }
                    }

                    dur += beat * event.beat_count()
                }
                PartItem::Tempo(tempo) => beat = tempo.duration_of(Quaver::Quarter),
                PartItem::Dynamic(_) => {}
            }
        }

        dur
    }
}

// TODO: Establish a single source of truth for these characters
#[derive(Debug, Copy, Clone, PartialEq, Eq, Display)]
pub enum Instrument {
    #[strum(to_string = "∿")]
    Beep,
    #[strum(to_string = "🎹")]
    Keyboard,
    #[strum(to_string = "🔔")]
    Bell,
    #[strum(to_string = "🌊")]
    Waterphone,
    #[strum(to_string = "🥁")]
    Drum,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PartItem {
    Event(Event),
    Dynamic(Dynamic),
    Tempo(Tempo),
}

const MINUTE: Duration = Duration::from_secs(60);

pub use crate::compile::output::{Dynamic, Metronome as Tempo, Quaver};

impl Tempo {
    pub const fn new(note_value: Quaver, beat: u32) -> Self {
        Self { note_value, beat }
    }

    pub fn duration_of(&self, note: Quaver) -> Duration {
        let fraction = note.to_f32() / self.note_value.to_f32();
        MINUTE.mul_f32(fraction) / self.beat
    }
}

#[cfg(test)]
#[test]
fn tempo_sanity_check() {
    // default inherited from muzak.el
    let t = Tempo::new(Quaver::Whole, 75);
    assert_eq!(t.duration_of(Quaver::Quarter), MINUTE / 300);

    // A score starting with "60"
    let t = Tempo::new(Quaver::Whole, 60);
    assert_eq!(t.duration_of(Quaver::Whole), MINUTE / 60);
    assert_eq!(t.duration_of(Quaver::Quarter), MINUTE / 240);

    // 𝅘𝅥=90 (anywhere in the score)
    let t = Tempo::new(Quaver::Quarter, 90);
    assert_eq!(t.duration_of(Quaver::Half), MINUTE / 45);
    assert_eq!(t.duration_of(Quaver::Quarter), MINUTE / 90);
}

impl Quaver {
    pub const fn to_f32(&self) -> f32 {
        match self {
            Self::Double => 2.0,
            Self::Whole => 1.0,
            Self::Half => 1.0 / 2.0,
            Self::Quarter => 1.0 / 4.0,
            Self::Eighth => 1.0 / 8.0,
            Self::Sixteenth => 1.0 / 16.0,
        }
    }
}

impl Dynamic {
    pub fn to_multiplier(self) -> f32 {
        let ratio = 2.0_f32.sqrt();
        let pow = match self {
            Self::Pianissimo => -3,
            Self::Piano => -2,
            Self::MezzoPiano => -1,
            Self::MezzoForte => 0, // 1x volume
            Self::Forte => 1,
            Self::Fortissimo => 2,
        };
        ratio.powi(pow)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Event {
    Rest(u32),
    Note(Note),
    Chord(Vec<Note>),
}

impl Event {
    pub fn notes(&self) -> &[Note] {
        match self {
            Self::Rest(..) => &[],
            Self::Note(note) => std::slice::from_ref(note),
            Self::Chord(notes) => notes,
        }
    }

    pub fn beat_count(&self) -> u32 {
        match self {
            Self::Rest(dur) => *dur,
            Self::Note(note) => note.duration,
            Self::Chord(notes) => notes.iter().map(|n| n.duration).max().unwrap_or(1),
        }
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
