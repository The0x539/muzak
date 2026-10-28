use clap::Parser;
use muzak::types::{Note, Score};
use std::io::{self, Write};
use std::os::fd::AsRawFd;
use std::path::Path;
use std::process::Stdio;
use uiua::Uiua;

mod script_backend;

#[derive(Parser, Debug, Clone)]
pub struct VideoOpts {
    #[command(flatten)]
    play_opts: super::PlayOpts,
    #[arg(long, value_parser = parse_note, default_value = "C4", num_args=1..)]
    strobe: Vec<f64>,
}

pub fn render(score: Score, opts: VideoOpts, output_path: &Path) -> super::Result<()> {
    let parts = score.clone().isolate_parts();

    let asr = muzak::instruments::SAMPLE_RATE.get();
    let mut backend = script_backend::MuzakSysBackend::new(asr);

    let mut frame_sources = vec![]; // io::PipeReader values to pass to ffmpeg
    let mut frame_sinks = vec![]; // uiua::Handle values (which map to io::PipeWriter values) to pass to Uiua
    for _ in 0..parts.len() {
        let (source, sink) = std::io::pipe()?;
        frame_sources.push(source);
        frame_sinks.push(backend.add_sink(sink));
    }

    let mut tracks = Vec::<Vec<f64>>::new();
    for part in parts {
        let (source, _duration) = muzak::mix(&part, opts.play_opts.clone().into());
        tracks.push(source.into_iter().map(f64::from).collect());
    }
    let len = tracks.iter().map(|t| t.len()).max().unwrap();
    for track in &mut tracks {
        track.resize(len, 0.0);
    }

    let num_tracks = tracks.len();
    let mut uiua = Uiua::with_backend(backend);

    let uiua_tracks =
        uiua::Array::from_row_arrays(tracks.into_iter().map(uiua::Array::from_iter), &uiua)?;

    uiua.push(uiua::Array::from_iter(
        frame_sinks.into_iter().map(|n| n.0 as f64),
    ));
    uiua.push(100);
    uiua.push(800);
    uiua.push(uiua_tracks);
    uiua.push(muzak::instruments::SAMPLE_RATE.get() as f64);
    if opts.strobe.len() == 1 {
        uiua.push(opts.strobe[0]);
    } else {
        let num_strobe = opts.strobe.len();
        assert_eq!(num_strobe, num_tracks, "Must specify either 1 global strobe frequency or 1 per track, but {num_strobe} frequencies were specified for a score with {num_tracks} tracks.");
        uiua.push(uiua::Array::from_iter(opts.strobe));
    }
    uiua.push(60);

    let mut ffmpeg = spawn_ffmpeg(&frame_sources, asr, output_path)?;

    let mut ffmpeg_stdin = ffmpeg.stdin.take().unwrap();
    let audio_thread = std::thread::spawn(move || {
        let (mut audio, _duration) = muzak::mix(&score, opts.play_opts.into());
        const CHUNK_SIZE: usize = 8192;
        let mut buf = Vec::<f32>::with_capacity(CHUNK_SIZE);
        loop {
            buf.clear();
            buf.extend(audio.by_ref().take(CHUNK_SIZE));
            _ = ffmpeg_stdin.write_all(bytemuck::cast_slice(&buf[..]));
            if buf.len() < CHUNK_SIZE {
                break;
            }
        }
    });

    uiua.run_str(include_str!("./video.ua"))?;
    drop(uiua); // Close the frame sink handles
    drop(frame_sources); // Close the frame source handles

    audio_thread.join().unwrap();
    ffmpeg.wait()?;

    Ok(())
}

fn spawn_ffmpeg(
    frame_sources: &[io::PipeReader],
    asr: u32,
    out_path: &Path,
) -> super::Result<std::process::Child> {
    let mut cmd = std::process::Command::new("ffmpeg");

    cmd.arg("-hide_banner").arg("-y");

    for source in frame_sources {
        // TODO: inherit file descriptors (hopefully the "pipe protocol" works as desired on all platforms)
        let pid = std::process::id();
        let fd = source.as_raw_fd();
        let procfs_path = format!("/proc/{pid}/fd/{fd}");

        // Input video
        cmd.args(["-f", "rawvideo"])
            .args(["-s", "800x100"])
            .args(["-pix_fmt", "monob"])
            .args(["-r", "60"])
            .arg("-i")
            .arg(procfs_path);
    }

    // Input audio
    cmd.args(["-f", "f32le"])
        .arg("-ar")
        .arg(asr.to_string())
        .args(["-i", "-"]);

    cmd.arg("-filter_complex")
        .arg(format!("vstack=inputs={}", frame_sources.len()));

    cmd.args(["-preset:v", "slower"])
        .args(["-crf", "25"])
        // .args(["-c:a", "pcm_s16le"])
        .arg(out_path)
        .stdin(Stdio::piped())
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit())
        .spawn()
        .map_err(From::from)
}

#[derive(Debug)]
struct Erm(winnow::error::ContextError);

impl std::fmt::Display for Erm {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.0.fmt(f)
    }
}

impl std::error::Error for Erm {}

fn parse_note(arg: &str) -> winnow::Result<f64, Erm> {
    use muzak::types::Accidental;
    use winnow::ascii::{dec_int, dec_uint};
    use winnow::combinator::{alt, empty, opt, seq};
    use winnow::Parser as _;

    fn accidental(input: &mut &str) -> winnow::ModalResult<Accidental> {
        alt((
            '♭'.value(Accidental::Flat),
            'b'.value(Accidental::Flat),
            '♮'.value(Accidental::Natural),
            '♯'.value(Accidental::Sharp),
            '#'.value(Accidental::Sharp),
        ))
        .parse_next(input)
    }

    fn note(input: &mut &str) -> winnow::ModalResult<Note> {
        seq! {Note{
            base: muzak::parse::base_note,
            accidental: opt(accidental),
            octave: opt(dec_int),
            duration: empty.value(1),
        }}
        .parse_next(input)
    }

    alt((
        note.map(|n| n.to_frequency() as f64),
        dec_uint.map(|n: u32| n as f64),
    ))
    .parse(arg)
    .map_err(|e| Erm(e.into_inner()))
}
