use std::fs::File;
use std::io::{Read, Write};
use std::path::{Path, PathBuf};

use clap::Parser;
use muzak::types::Score;

#[derive(Parser, Debug, Clone)]
#[command(version, about)]
struct Args {
    #[command(subcommand)]
    command: Command,

    #[arg(short)]
    input_file: Option<PathBuf>,

    #[arg(short)]
    output_file: Option<PathBuf>,
}

#[derive(Parser, Debug, Clone)]
enum Command {
    Compile,
    Play,
}

type Result<T, E = Box<dyn std::error::Error + Send + Sync>> = std::result::Result<T, E>;

fn main() -> Result<()> {
    let args = Args::parse();

    let input = read_text(args.input_file.as_deref())?;

    match args.command {
        Command::Compile => {
            let musicxml = read_text(args.input_file.as_deref())?;
            let bells = muzak::compile(&musicxml);
            println!("{bells}");
        }
        Command::Play => {
            // winnow errors don't impl std::error::Error for some reason
            let score = muzak::parse(&input).expect("Could not parse score");
            output_audio(args.output_file.as_deref(), &score)?;
        }
    }

    Ok(())
}

fn read_text(path: Option<&Path>) -> Result<String> {
    if let Some(path) = path {
        let buf = std::fs::read_to_string(path)?;
        Ok(buf)
    } else if atty::isnt(atty::Stream::Stdin) {
        let mut buf = String::new();
        std::io::stdin().read_to_string(&mut buf)?;
        Ok(buf)
    } else {
        eprintln!("Please specify an input path or pipe data via stdin");
        std::process::exit(1);
    }
}

fn output_audio(path: Option<&Path>, score: &Score) -> Result<()> {
    if let Some(path) = path {
        if path.exists() {
            let sure = dialoguer::Confirm::new()
                .with_prompt(format!("Really overwrite `{}`?", path.display()))
                .default(false)
                .interact()?;

            if !sure {
                std::process::exit(1);
            }
        }

        let f = File::create_new(path)?;
        write_wav(f, score)?;
    } else if atty::isnt(atty::Stream::Stdout) {
        write_wav(std::io::stdout(), score)?;
    } else {
        muzak::play(score);
    }

    Ok(())
}

fn write_wav(mut writer: impl Write, score: &Score) -> Result<()> {
    let source = muzak::mix(score);

    // If there's only 1 channel, hound seems to put it on the left. Annoying.
    let spec = hound::WavSpec {
        channels: 2,
        sample_rate: 48000,
        bits_per_sample: 32,
        sample_format: hound::SampleFormat::Float,
    };

    let mut buf = std::io::Cursor::new(Vec::new());

    let mut wav = hound::WavWriter::new(&mut buf, spec)?;
    for sample in source {
        wav.write_sample(sample)?;
        wav.write_sample(sample)?;
    }
    wav.finalize()?;

    writer.write_all(buf.get_ref())?;

    Ok(())
}
