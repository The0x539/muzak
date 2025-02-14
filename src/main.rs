use winnow::Parser;

mod fmt;
mod parse;
mod types;

fn main() {
    let buffer = std::fs::read_to_string("/home/the0x539/winhome/Music/balatro-r1.txt")
        .unwrap()
        .split_whitespace()
        .collect::<String>();

    let mut input = &*buffer;

    let score = parse::score.parse(&mut input).unwrap();

    if let Some(bpm) = score.bpm {
        println!("{bpm}");
    }

    for (i, part) in score.parts.iter().enumerate() {
        if i > 0 {
            println!("|")
        }
        for event in part {
            print!("{event}");
        }
        println!();
    }
}
