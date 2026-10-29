use clap::Parser;
use ecow::EcoVec;
use itertools::Itertools;
use muzak::types::{Note, Score};
use rayon::prelude::*;
use rodio::Source;
use std::io::Write;
use std::path::Path;
use std::process::{Command, Stdio};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::Duration;
use uiua::Uiua;

mod script_backend;

#[cfg(unix)]
mod ipc_unix;

#[cfg(windows)]
mod ipc_windows;

trait Ipc
where
    for<'a> &'a Self::Sink: Write,
{
    type Intermediate;
    type Sink: Write;

    fn register_source(cmd: &mut Command, id: usize) -> crate::Result<Self::Intermediate>;
    fn obtain_sink(intermediate: Self::Intermediate) -> crate::Result<Self::Sink>;
    fn sink_as_raw(sink: &Self::Sink) -> u64;
}

#[cfg(unix)]
type IpcImpl = ipc_unix::UnixAnonymousPipes;

#[cfg(windows)]
type IpcImpl = ipc_windows::WindowsNamedPipes;

type FrameSink = <IpcImpl as Ipc>::Sink;

#[derive(Parser, Debug, Clone)]
pub struct VideoOpts {
    #[command(flatten)]
    play_opts: super::PlayOpts,
    #[arg(long, value_parser = parse_note, default_value = "C4", num_args=1..)]
    strobe: Vec<f64>,
    #[arg(long, num_args = 2, default_values_t = [800, 100])]
    resolution: Vec<usize>,
    #[arg(long, default_value_t = 60)]
    framerate: usize,
}

pub fn render(score: Score, opts: VideoOpts, output_path: &Path) -> super::Result<()> {
    let parts = score.clone().isolate_parts();

    let resolution = (opts.resolution[0], opts.resolution[1]);

    let mut sources = vec![];
    let mut duration = Duration::ZERO;
    for part in parts {
        let (source, part_duration) = muzak::mix(&part, opts.play_opts.clone().into());
        duration = duration.max(part_duration);
        sources.push(source);
    }

    let num_tracks = sources.len();

    let estimated_length = duration.as_secs_f64() * sources[0].sample_rate().get() as f64;
    let prealloc_size = (estimated_length * 1.01 + resolution.0 as f64).ceil() as usize;

    println!("Generating individual track audio...");
    let max_track_len = AtomicUsize::new(0);
    let mut tracks = sources
        .into_par_iter()
        .map(|source| {
            let samples = source.into_iter().map(f64::from);
            let mut buf = EcoVec::<f64>::from_elem(0.0, prealloc_size);
            let len = resolution.0 + buf.make_mut()[resolution.0..].iter_mut().set_from(samples);
            assert!(len < buf.len());
            max_track_len.fetch_max(len, Ordering::Relaxed);
            buf
        })
        .collect::<Vec<_>>();

    let max_track_len = max_track_len.into_inner();
    for track in &mut tracks {
        track.truncate(max_track_len);
    }

    let uiua_tracks = tracks
        .into_iter()
        .map(|track| uiua::Boxed(uiua::Value::Num(uiua::Array::new(track.len(), track))))
        .collect::<uiua::Array<uiua::Boxed>>();

    let asr = muzak::instruments::SAMPLE_RATE.get();

    let (mut ffmpeg, frame_sinks) =
        spawn_ffmpeg(num_tracks, asr, resolution, opts.framerate, output_path)?;

    let mut backend = script_backend::MuzakSysBackend::new(asr);

    let uiua_frame_sinks = frame_sinks
        .into_iter()
        .map(|sink| backend.add_sink(sink).0 as f64)
        .collect_vec();

    let mut uiua = Uiua::with_backend(backend);

    uiua.push(uiua::Array::from_iter(uiua_frame_sinks));
    uiua.push_all([resolution.1, resolution.0]);
    uiua.push(uiua_tracks);
    uiua.push(muzak::instruments::SAMPLE_RATE.get() as f64);
    if opts.strobe.len() == 1 {
        uiua.push(opts.strobe[0]);
    } else {
        let num_strobe = opts.strobe.len();
        assert_eq!(num_strobe, num_tracks, "Must specify either 1 global strobe frequency or 1 per track, but {num_strobe} frequencies were specified for a score with {num_tracks} tracks.");
        uiua.push(uiua::Array::from_iter(opts.strobe));
    }
    uiua.push(opts.framerate);

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

    eprintln!("Launching Uiua visualizer...");
    let _compiler = uiua.run_str(include_str!("./video.ua"))?;
    uiua.print_reports();
    drop(uiua); // Close all the frame sink streams so that FFmpeg knows they're done

    audio_thread.join().unwrap();
    ffmpeg.wait()?;

    Ok(())
}

fn spawn_ffmpeg(
    num_tracks: usize,
    asr: u32,
    (width, height): (usize, usize),
    framerate: usize,
    out_path: &Path,
) -> super::Result<(std::process::Child, Vec<FrameSink>)> {
    let mut cmd = std::process::Command::new("ffmpeg");

    cmd.arg("-hide_banner").arg("-y");

    // PipeWriters on Unix, named pipe listeners on Windows.
    let mut intermediates = vec![];

    for i in 0..num_tracks {
        // Input video
        cmd.args(["-f", "rawvideo"])
            .args(["-s", &format!("{width}x{height}")])
            .args(["-pix_fmt", "monob"])
            .args(["-r", &framerate.to_string()]);

        intermediates.push(IpcImpl::register_source(&mut cmd, i)?);
    }

    // Input audio
    cmd.args(["-f", "f32le"])
        .args(["-ar", &asr.to_string()])
        .args(["-i", "-"]);

    // Compositing
    cmd.args(["-filter_complex", &format!("vstack=inputs={num_tracks}")]);

    let child = cmd
        .args(["-preset:v", "slower"])
        .args(["-crf", "25"])
        // .args(["-c:a", "pcm_s16le"])
        .arg(out_path)
        .stdin(Stdio::piped())
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit())
        .spawn()?;

    // On Windows, establish a connection with each named pipe.
    let sinks = intermediates
        .into_iter()
        .map(IpcImpl::obtain_sink)
        .collect::<Result<_, _>>()?;

    Ok((child, sinks))
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
