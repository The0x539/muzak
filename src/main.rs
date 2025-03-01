use std::fs::File;
use std::io::{Read, Write};
use std::path::{Path, PathBuf};

use clap::Parser;
use muzak::rodio::Source;
use muzak::types::Score;

#[derive(Parser, Debug, Clone)]
#[command(version, about)]
struct Args {
    #[command(subcommand)]
    command: Command,
}

#[derive(Parser, Debug, Clone)]
enum Command {
    Compile {
        musicxml_path: Option<PathBuf>,
    },
    Play {
        score_path: Option<PathBuf>,
    },
    Render {
        score_path: Option<PathBuf>,
        #[arg(short, long)]
        output: Option<PathBuf>,
    },
}

type Result<T, E = Box<dyn std::error::Error + Send + Sync>> = std::result::Result<T, E>;

fn main() -> Result<()> {
    let args = Args::parse();

    if let Command::Compile { musicxml_path } = args.command {
        compile(musicxml_path.as_deref())?;
        return Ok(());
    }

    let (Command::Play { score_path } | Command::Render { score_path, .. }) = &args.command else {
        // TODO: Unify the play and render commands
        unreachable!()
    };
    let score_text: String = match score_path {
        None if atty::is(atty::Stream::Stdin) => {
            eprintln!("Please specify an input path or use a redirect to pass data through stdin");
            return Ok(());
        }
        None => {
            let mut buf = String::new();
            std::io::stdin().read_to_string(&mut buf)?;
            buf
        }
        Some(path) => std::fs::read_to_string(path)?,
    };

    // winnow errors don't impl std::error::Error for some reason
    let score = muzak::parse(&score_text).expect("Could not parse score");

    match args.command {
        Command::Play { .. } => muzak::play(&score),
        Command::Render { output, .. } => {
            let out_path = output.as_deref().unwrap_or("-".as_ref());
            export(out_path, &score)?;
        }
        _ => unreachable!(),
    }

    Ok(())
}

fn compile(path: Option<&Path>) -> Result<()> {
    let musicxml = read_input(path)?;
    let bells = muzak::compile::compile(&musicxml);
    println!("{bells}");
    Ok(())
}

fn read_input(path: Option<&Path>) -> Result<String> {
    if let Some(path) = path {
        let buf = std::fs::read_to_string(path)?;
        Ok(buf)
    } else if atty::isnt(atty::Stream::Stdin) {
        let mut buf = String::new();
        std::io::stdin().read_to_string(&mut buf)?;
        Ok(buf)
    } else {
        eprintln!("Please specify an input path or use a redirect to pass data through stdin");
        std::process::exit(1);
    }
}

fn export(path: impl AsRef<Path>, score: &Score) -> Result<()> {
    let source = muzak::mix(score);
    let path = path.as_ref();
    if path == Path::new("-") {
        if atty::is(atty::Stream::Stdout) {
            let sure = dialoguer::Confirm::new()
                .with_prompt("Really output WAVE data to terminal?")
                .default(false)
                .interact()?;

            if !sure {
                return Ok(());
            }
        }

        // hound's writer impl needs to seek back to the start to write the data length,
        // so we can't just write directly to stdout without switching crates again
        // TODO: use WavSpec::into_header_for_infinite_file in both cases
        let mut buf = std::io::Cursor::new(Vec::new());
        export_impl(&mut buf, source)?;
        std::io::stdout().write_all(buf.get_ref())?;
    } else {
        if path.exists() {
            let sure = dialoguer::Confirm::new()
                .with_prompt(format!("Overwrite {}?", path.display()))
                .default(true)
                .interact()?;

            if !sure {
                return Ok(());
            }
        }

        let file = std::io::BufWriter::new(File::create(path)?);
        export_impl(file, source)?;
    }

    Ok(())
}

fn export_impl<W: Write + std::io::Seek>(
    writer: W,
    source: impl Source<Item = f32>,
) -> Result<(), hound::Error> {
    // If there's only 1 channel, hound seems to put it on the left. Annoying.
    let spec = hound::WavSpec {
        channels: 2,
        sample_rate: 48000,
        bits_per_sample: 32,
        sample_format: hound::SampleFormat::Float,
    };
    let mut wav = hound::WavWriter::new(writer, spec)?;
    for sample in source {
        wav.write_sample(sample)?;
        wav.write_sample(sample)?;
    }
    wav.finalize()?;
    Ok(())
}
