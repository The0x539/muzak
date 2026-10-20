use clap::Parser;
use muzak::types::{Note, Score};
use std::fs::Permissions;
use std::io::Write;
use std::os::unix::fs::{mkfifo, PermissionsExt};
use std::path::Path;
use std::process::Stdio;
use uiua::Uiua;

#[derive(Parser, Debug, Clone)]
pub struct VideoOpts {
    #[command(flatten)]
    play_opts: super::PlayOpts,
    #[arg(long, value_parser = parse_note, default_value = "C4", num_args=1..)]
    strobe: Vec<f64>,
}

pub fn render(score: Score, opts: VideoOpts, output_path: &Path) -> super::Result<()> {
    let parts = score.clone().isolate_parts();

    let mut fifo_handles = vec![];
    let mut fifo_paths = vec![];
    let pid = std::process::id();
    let perms = Permissions::from_mode(0o600);

    let mut uiua = Uiua::with_native_sys();

    for i in 0..parts.len() {
        let path = format!("/tmp/muzak-fifo-{pid}-{i:02}");
        mkfifo(&path, perms.clone())?;

        let handle = uiua.backend().open_file(path.as_ref(), true).unwrap();
        fifo_handles.push(handle.0 as f64);
        fifo_paths.push(path);
    }

    let mut tracks = Vec::<Vec<f64>>::new();
    for (i, mut part) in parts.into_iter().enumerate() {
        if i != 0 && part.parts[0].instrument.is_none() {
            part.parts[0].instrument = Some(muzak::types::Instrument::Keyboard);
        }
        let (source, _duration) = muzak::mix(&part, opts.play_opts.clone().into());
        tracks.push(source.into_iter().map(f64::from).collect());
    }
    let len = tracks.iter().map(|t| t.len()).max().unwrap();
    for track in &mut tracks {
        track.resize(len, 0.0);
    }

    let num_tracks = tracks.len();

    let uiua_tracks =
        uiua::Array::from_row_arrays(tracks.into_iter().map(uiua::Array::from_iter), &uiua)?;

    uiua.push(uiua::Array::from_iter(fifo_handles));
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

    let mut ffmpeg = spawn_ffmpeg(&fifo_paths, output_path)?;

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

    for path in fifo_paths {
        std::fs::remove_file(path)?;
    }

    audio_thread.join().unwrap();
    ffmpeg.wait()?;

    Ok(())
}

fn spawn_ffmpeg(
    fifo_paths: &[impl AsRef<Path>],
    out_path: &Path,
) -> super::Result<std::process::Child> {
    let mut cmd = std::process::Command::new("ffmpeg");

    cmd.arg("-hide_banner").arg("-y");

    for path in fifo_paths {
        // Input video
        cmd.args(["-f", "rawvideo"])
            .args(["-s", "800x100"])
            .args(["-pix_fmt", "monob"])
            .args(["-r", "60"])
            .arg("-i")
            .arg(path.as_ref());
    }

    // Input audio
    cmd.args(["-f", "f32le"])
        .arg("-ar")
        .arg(muzak::instruments::SAMPLE_RATE.get().to_string())
        .args(["-i", "-"]);

    cmd.arg("-filter_complex")
        .arg(format!("vstack=inputs={}", fifo_paths.len()));

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
