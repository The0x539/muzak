use std::fmt::{self, Display, Formatter, Write};

use crate::types::*;

impl Display for Diatonic {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        fmt::Debug::fmt(self, f)
    }
}

impl Display for BaseNote {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        let mut c = ['A', 'B', 'C', 'D', 'E', 'F', 'G'][self.note as usize];
        if self.high {
            c = c.to_ascii_lowercase();
        }
        f.write_char(c)
    }
}

impl Display for Accidental {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        let c = match self {
            Accidental::Flat => '♭',
            Accidental::Natural => '♮',
            Accidental::Sharp => '♯',
        };
        f.write_char(c)
    }
}

impl Display for Note {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        self.base.fmt(f)?;
        if let Some(a) = self.accidental {
            a.fmt(f)?;
        }
        if let Some(o) = self.octave {
            o.fmt(f)?;
        }
        for _ in 1..self.duration {
            f.write_char('~')?;
        }
        Ok(())
    }
}

impl Display for Event {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Event::Rest(dur) => {
                for _ in 0..*dur {
                    f.write_char('/')?;
                }
                Ok(())
            }
            Event::Note(note) => note.fmt(f),
            Event::Chord(notes) => {
                f.write_char('[')?;
                for note in notes {
                    note.fmt(f)?;
                }
                f.write_char(']')?;
                Ok(())
            }
        }
    }
}
