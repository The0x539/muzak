use std::str::FromStr;

use strum::VariantArray;
use winnow::Parser;
use winnow::combinator::{
    alt, cut_err, delimited, opt, preceded, repeat, separated, seq, terminated,
};
use winnow::error::{StrContext, StrContextValue};
use winnow::token::{one_of, take_till, take_while};

use crate::types::*;

macro_rules! parsers {
    ($(
        $vis:vis $name:ident : $ty:ty = $body:expr;
    )*) => {$(
        $vis fn $name(input: &mut &str) -> winnow::ModalResult<$ty> {
            $body.parse_next(input)
        }
    )*}
}

macro_rules! literals {
    (
        $ty:ty;
        $($lit:literal => $variant:ident),*$(,)?
    ) => {
        alt((
            $($lit.value(<$ty>::$variant),)*
        ))
    }
}

parsers! {
    pub accidental: Accidental = literals! {
        Accidental;
        '♭' => Flat,
        '♮' => Natural,
        '♯' => Sharp,
        '#' => Sharp,
    };

    pub note: Note = seq! {Note{
        base: base_note,
        accidental: opt(accidental),
        octave: opt(integer),
        duration: repeat(0.., preceded(junk, '~')).map(|n: usize| n as u32 + 1),
    }};

    bed: () = seq!('🛏', opt('\u{fe0f}')).void(); // some U+FE0F Variant Selector nonsense

    pub event: Event = alt((
        '/'.value(Event::Rest(1)),
        preceded(bed, integer).map(Event::Rest),
        note.map(Event::Note),
        delimited('[', repeat(0.., note), cut_err(']'))
            .map(Event::Chord)
            .context(StrContext::Expected(StrContextValue::Description("unclosed chord"))),
    ));

    pub dynamic: Dynamic = literals! {
        Dynamic;
        "𝓹𝓹" => Pianissimo,
        "𝓯𝓯" => Fortissimo,
        "𝓶𝓹" => MezzoPiano,
        "𝓶𝓯" => MezzoForte,
        "𝓹" => Piano,
        "𝓯" => Forte,
    };

    pub part_item: PartItem = alt((
        event.map(PartItem::Event),
        // a dynamic MUST have whitespace or something after it,
        // so as to avoid cases such as 𝓯𝓯𝓯𝓯 being interpreted as two consecutive fortissimos
        terminated(dynamic, junk).map(PartItem::Dynamic),
    ));

    pub instrument: Instrument = literals! {
        Instrument;
        '∿' => Beep,
        '🎹' => Keyboard,
        '🔔' => Bell,
        '🌊' => Waterphone,
        '🥁' => Snare,
     };

    pub part: Part =  seq! {Part{
        instrument: opt(terminated(instrument, junk)),
        items: repeat(0.., terminated(part_item, junk)),
    }};

    pub score: Score = seq! {Score{
        _: junk,
        bpm: opt(integer),
        parts: separated(0.., preceded(junk, part), '|'),
    }};

    junk: () = separated(0.., unrecognized, comment.void());

    // Rather than only accepting whitespace between stuff,
    // ignore any characters that aren't recognized at all.
    // This makes implementation a decent bit more difficult,
    // but it makes it easier to add visual markers to a file.
    unrecognized: () = take_till(0.., |c| NOT_JUNK.contains(c)).void();

    comment: () = {
     let body = separated(0.., take_till(0.., ['(', ')']).void(), comment);
        delimited(
             '(',
             body,
             cut_err(')').context(StrContext::Label("unclosed comment"))
        )
    };
}

pub fn base_note(input: &mut &str) -> winnow::ModalResult<BaseNote> {
    let letter = one_of(b"ABCDEFGabcdefg").parse_next(input)?;

    let index = ('A'..letter.to_ascii_uppercase()).count();
    let note = Diatonic::VARIANTS[index];
    let high = letter.is_ascii_lowercase();

    Ok(BaseNote { note, high })
}

pub fn integer<T: FromStr>(input: &mut &str) -> winnow::ModalResult<T>
where
    T::Err: Send + Sync + std::error::Error + 'static,
{
    let signed = T::from_str("-1").is_ok();

    take_while(.., |c: char| c.is_ascii_digit() || (signed && c == '-'))
        .try_map(T::from_str)
        .parse_next(input)
}

const NOT_JUNK: &str = concat!(
    // Notes
    "ABCDEFG",
    "abcdefg",
    // BPM
    "0123456789",
    // Part dividers, chords, rests, and note-extensions on new lines
    "|[/~🛏",
    // Instruments
    "∿🎹🔔🌊🥁",
    // Dynamics
    "𝓹𝓶𝓯",
    // Comments
    "(",
);
