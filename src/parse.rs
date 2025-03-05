use std::str::FromStr;

use strum::VariantArray;
use winnow::combinator::{alt, delimited, opt, preceded, repeat, separated, seq, terminated};
use winnow::token::{one_of, take_till, take_while};
use winnow::{Parser, Result};

use crate::types::*;

pub fn base_note(input: &mut &str) -> Result<BaseNote> {
    let letter = one_of(b"ABCDEFGabcdefg").parse_next(input)?;

    let index = ('A'..letter.to_ascii_uppercase()).count();
    let note = Diatonic::VARIANTS[index];
    let high = letter.is_ascii_lowercase();

    Ok(BaseNote { note, high })
}

pub fn accidental(input: &mut &str) -> Result<Accidental> {
    alt((
        '♭'.value(Accidental::Flat),
        '♮'.value(Accidental::Natural),
        one_of(['#', '♯']).value(Accidental::Sharp),
    ))
    .parse_next(input)
}

pub fn integer<T: FromStr>(input: &mut &str) -> Result<T>
where
    T::Err: Send + Sync + std::error::Error + 'static,
{
    let signed = T::from_str("-1").is_ok();

    take_while(.., |c: char| c.is_ascii_digit() || (signed && c == '-'))
        .try_map(T::from_str)
        .parse_next(input)
}

pub fn note(input: &mut &str) -> Result<Note> {
    seq! {Note{
        base: base_note,
        accidental: opt(accidental),
        octave: opt(integer),
        duration: repeat(0.., preceded(junk, '~')).map(|n: usize| n as u32 + 1),
    }}
    .parse_next(input)
}

pub fn event(input: &mut &str) -> Result<Event> {
    alt((
        '/'.value(Event::Rest),
        note.map(Event::Note),
        delimited('[', repeat(0.., note), ']').map(Event::Chord),
    ))
    .parse_next(input)
}

pub fn dynamic(input: &mut &str) -> Result<Dynamic> {
    alt((
        "𝓹𝓹".value(Dynamic::Pianissimo),
        "𝓹".value(Dynamic::Piano),
        "𝓶𝓹".value(Dynamic::MezzoPiano),
        "𝓶𝓯".value(Dynamic::MezzoForte),
        "𝓯".value(Dynamic::Forte),
        "𝓯𝓯".value(Dynamic::Fortissimo),
    ))
    .parse_next(input)
}

pub fn part_item(input: &mut &str) -> Result<PartItem> {
    alt((event.map(PartItem::Event), dynamic.map(PartItem::Dynamic))).parse_next(input)
}

pub fn instrument(input: &mut &str) -> Result<Instrument> {
    alt((
        '∿'.value(Instrument::Beep),
        '🎹'.value(Instrument::Keyboard),
        '🔔'.value(Instrument::Bell),
        '🌊'.value(Instrument::Waterphone),
    ))
    .parse_next(input)
}

pub fn part(input: &mut &str) -> Result<Part> {
    let instrument = opt(terminated(instrument, junk)).parse_next(input)?;
    repeat(0.., terminated(part_item, junk))
        .map(|items| Part { instrument, items })
        .parse_next(input)
}

pub fn score(input: &mut &str) -> Result<Score> {
    seq! {Score{
        _: junk,
        bpm: opt(integer),
        parts: separated(0.., preceded(junk, part), '|'),
    }}
    .parse_next(input)
}

// Rather than only accepting whitespace between stuff,
// ignore any characters that aren't recognized at all.
// This makes implementation a decent bit more difficult,
// but it makes it easier to add visual markers to a file.
fn junk(input: &mut &str) -> Result<()> {
    const NOT_JUNK: &str = concat!(
        // Notes
        "ABCDEFG",
        "abcdefg",
        // BPM
        "0123456789",
        // Part dividers, chords, rests, and note-extensions on new lines
        "|[/~",
        // Instruments
        "∿🎹🔔🌊",
        "𝓯"
    );
    take_till(0.., |c| NOT_JUNK.contains(c))
        .void()
        .parse_next(input)
}
