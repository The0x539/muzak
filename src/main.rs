use std::fs::File;
use std::io::{IsTerminal, Read, Write};
use std::num::NonZeroUsize;
use std::path::{Path, PathBuf};
use std::time::Duration;

use clap::Parser;
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
    Play(PlayOpts),
}

#[derive(Parser, Debug, Clone)]
struct PlayOpts {
    #[arg(short = 'd', long, value_name = "SECONDS")]
    max_duration: Option<f64>,
    #[arg(short = 't', long, value_name = "COUNT")]
    max_tracks: Option<NonZeroUsize>,
}

impl From<PlayOpts> for muzak::MixOptions {
    fn from(cli: PlayOpts) -> Self {
        muzak::MixOptions {
            max_duration: cli.max_duration.map(Duration::from_secs_f64),
            max_tracks: cli.max_tracks,
        }
    }
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
        Command::Play(options) => {
            // winnow errors don't impl std::error::Error for some reason
            let score = muzak::parse(&input).expect("Could not parse score");
            let (source, duration) = muzak::mix(&score, options.into());
            output_audio(args.output_file.as_deref(), source, duration)?;
        }
    }

    Ok(())
}

fn read_text(path: Option<&Path>) -> Result<String> {
    if let Some(path) = path {
        let buf = std::fs::read_to_string(path)?;
        Ok(buf)
    } else if !std::io::stdin().is_terminal() {
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

fn output_audio(
    path: Option<&Path>,
    source: impl rodio::Source<Item = f32> + Send + 'static,
    duration: Duration,
) -> Result<()> {
    if let Some(path) = path {
        write_wav(ask_before_overwriting(path)?, source)?;
    } else if !std::io::stdout().is_terminal() {
        write_wav(std::io::stdout(), source)?;
    } else {
        muzak::play(source, duration);
    }

    Ok(())
}

fn write_wav(
    mut writer: impl Write,
    source: impl rodio::Source<Item = f32> + Send + 'static,
) -> Result<()> {
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
