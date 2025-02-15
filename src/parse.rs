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
        duration: repeat(0.., '~').map(|n: usize| n as u32 + 1),
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

pub fn part(input: &mut &str) -> Result<Part> {
    repeat(0.., terminated(event, junk))
        .map(|events| Part { events })
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

fn junk(input: &mut &str) -> Result<()> {
    take_till(0.., b"ABCDEFGabcdefg0123456789|[/")
        .void()
        .parse_next(input)
}
