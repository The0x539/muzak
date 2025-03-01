use std::fs::File;
use std::io::{Read, Write};
use std::path::{Path, PathBuf};

use clap::Parser;
use muzak::types::Score;
use wavers::WavHeader;

#[derive(Parser, Debug, Clone)]
#[command(version, about)]
struct Args {
    #[command(subcommand)]
    command: Command,

    #[arg(short, global = true)]
    input_file: Option<PathBuf>,

    #[arg(short, global = true)]
    output_file: Option<PathBuf>,
}

#[derive(Parser, Debug, Clone)]
enum Command {
    /// Convert MusicXML to bells-text.
    Compile {
        /// Insert some number of blank tracks before the first track.
        /// Possibly useful for instrument selection.
        #[arg(short, long = "pad", default_value_t)]
        padding: u8,
        /// Rotate the track list forward by some amount.
        /// Possibly useful for instrument selection.
        #[arg(short, long = "rotate", default_value_t)]
        rotation: u8,
    },
    /// Convert bells-text to audio, either over speakers or as WAV data.
    Play,
}

type Result<T, E = Box<dyn std::error::Error + Send + Sync>> = std::result::Result<T, E>;

fn main() -> Result<()> {
    let args = Args::parse();

    let input = read_text(args.input_file.as_deref())?;

    match args.command {
        Command::Compile { padding, rotation } => {
            let bells = muzak::compile(&input, padding, rotation);
            if let Some(path) = args.output_file.as_deref() {
                let mut f = ask_before_overwriting(path)?;
                write!(f, "{bells}")?;
            } else {
                println!("{bells}");
            }
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

fn ask_before_overwriting(path: &Path) -> Result<File> {
    if path.exists() {
        let sure = dialoguer::Confirm::new()
            .with_prompt(format!("Really overwrite `{}`?", path.display()))
            .default(false)
            .interact()?;

        if !sure {
            std::process::exit(1);
        }
    }

    let f = File::create(path)?;
    Ok(f)
}

fn output_audio(path: Option<&Path>, score: &Score) -> Result<()> {
    if let Some(path) = path {
        write_wav(ask_before_overwriting(path)?, score)?;
    } else if atty::isnt(atty::Stream::Stdout) {
        write_wav(std::io::stdout(), score)?;
    } else {
        muzak::play(score);
    }

    Ok(())
}

fn write_wav(mut writer: impl Write, score: &Score) -> Result<()> {
    let source = muzak::mix(score);

    let mut samples: Vec<f32> = source.collect();

    if cfg!(target_endian = "big") {
        for sample in &mut samples {
            *sample = f32::from_le_bytes(sample.to_be_bytes());
        }
    }

    let header = WavHeader::new_header::<f32>(48000, 1, samples.len())?;

    let sample_bytes: &[u8] = bytemuck::cast_slice(&samples);

    writer.write_all(&header.as_extended_bytes())?;
    writer.write_all(b"data")?;
    writer.write_all(&(sample_bytes.len() as u32).to_le_bytes())?;
    writer.write_all(sample_bytes)?;

    Ok(())
}
