use musicxml::datatypes::NoteTypeValue;

pub trait NoteTypeValueExt: Sized {
    fn double(&self) -> Self;
    fn half(&self) -> Self;
    fn as_float(&self) -> f64;
    fn from_denominator(value: u8) -> Option<Self>;
}

impl NoteTypeValueExt for NoteTypeValue {
    fn double(&self) -> Self {
        match self {
            Self::Maxima => panic!(),
            Self::Long => Self::Maxima,
            Self::Breve => Self::Long,
            Self::Whole => Self::Breve,
            Self::Half => Self::Whole,
            Self::Quarter => Self::Half,
            Self::Eighth => Self::Quarter,
            Self::Sixteenth => Self::Eighth,
            Self::ThirtySecond => Self::Sixteenth,
            Self::SixtyFourth => Self::ThirtySecond,
            Self::OneHundredTwentyEighth => Self::SixtyFourth,
            Self::TwoHundredFiftySixth => Self::OneHundredTwentyEighth,
            Self::FiveHundredTwelfth => Self::TwoHundredFiftySixth,
            Self::OneThousandTwentyFourth => Self::FiveHundredTwelfth,
        }
    }

    fn half(&self) -> Self {
        match self {
            Self::Maxima => Self::Long,
            Self::Long => Self::Breve,
            Self::Breve => Self::Whole,
            Self::Whole => Self::Half,
            Self::Half => Self::Quarter,
            Self::Quarter => Self::Eighth,
            Self::Eighth => Self::Sixteenth,
            Self::Sixteenth => Self::ThirtySecond,
            Self::ThirtySecond => Self::SixtyFourth,
            Self::SixtyFourth => Self::OneHundredTwentyEighth,
            Self::OneHundredTwentyEighth => Self::TwoHundredFiftySixth,
            Self::TwoHundredFiftySixth => Self::FiveHundredTwelfth,
            Self::FiveHundredTwelfth => Self::OneThousandTwentyFourth,
            Self::OneThousandTwentyFourth => panic!(),
        }
    }

    fn as_float(&self) -> f64 {
        let n: f64 = [
            0.125, 0.25, 0.5, 1.0, 2.0, 4.0, 8.0, 16.0, 32.0, 64.0, 128.0, 256.0, 512.0, 1024.0,
        ][*self as usize];
        n.recip()
    }

    fn from_denominator(value: u8) -> Option<Self> {
        Some(match value {
            1 => Self::Whole,
            2 => Self::Half,
            4 => Self::Quarter,
            8 => Self::Eighth,
            16 => Self::Sixteenth,
            32 => Self::ThirtySecond,
            64 => Self::SixtyFourth,
            128 => Self::OneHundredTwentyEighth,
            _ => return None,
        })
    }
}
