use clap::Parser;
use muzak::types::Score;
use std::fs::Permissions;
use std::io::Write;
use std::os::unix::fs::{mkfifo, PermissionsExt};
use std::path::{Path, PathBuf};
use std::process::Stdio;
use uiua::Uiua;

#[derive(Parser, Debug, Clone)]
pub struct VideoOpts {
    #[command(flatten)]
    play_opts: super::PlayOpts,
    #[arg(long, short)]
    output_path: PathBuf,
}

pub fn render(score: Score, opts: VideoOpts) -> super::Result<()> {
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

    let uiua_tracks =
        uiua::Array::from_row_arrays(tracks.into_iter().map(uiua::Array::from_iter), &uiua)?;

    // this should probably be per-track
    // let c5 = 523.251;
    let g4 = 391.995;

    uiua.push(uiua::Array::from_iter(fifo_handles));
    uiua.push(uiua_tracks);
    uiua.push(muzak::instruments::SAMPLE_RATE.get() as f64);
    uiua.push(g4);
    uiua.push(60);

    let mut ffmpeg = spawn_ffmpeg(&fifo_paths, &opts.output_path)?;

    let mut ffmpeg_stdin = ffmpeg.stdin.take().unwrap();
    let audio_thread = std::thread::spawn(move || {
        let (audio, _duration) = muzak::mix(&score, opts.play_opts.into());
        // TODO: Feed ffmpeg big chunks but not the whole thing all at once
        let audio = audio.collect::<Vec<_>>();
        _ = ffmpeg_stdin.write_all(bytemuck::cast_slice(&audio[..]));
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

    cmd.args(["-preset:v", "medium"])
        .args(["-crf", "28"])
        // .args(["-c:a", "pcm_s16le"])
        .arg(out_path)
        .stdin(Stdio::piped())
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit())
        .spawn()
        .map_err(From::from)
}
