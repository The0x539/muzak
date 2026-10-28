use std::collections::HashMap;
use std::io::{self, Write as _};

use super::{FrameSink, Ipc, IpcImpl};

pub struct MuzakSysBackend {
    pub audio_sample_rate: u32,
    frame_sinks: HashMap<u64, FrameSink>,
}

impl MuzakSysBackend {
    pub fn new(audio_sample_rate: u32) -> Self {
        Self {
            audio_sample_rate,
            frame_sinks: HashMap::new(),
        }
    }

    pub fn add_sink(&mut self, sink: FrameSink) -> uiua::Handle {
        let fd = IpcImpl::sink_as_raw(&sink);
        self.frame_sinks.insert(fd, sink);
        uiua::Handle(fd)
    }
}

impl uiua::SysBackend for MuzakSysBackend {
    fn allow_thread_spawning(&self) -> bool {
        true
    }
    fn audio_sample_rate(&self) -> u32 {
        self.audio_sample_rate
    }
    fn write(&self, handle: uiua::Handle, contents: &[u8]) -> Result<(), String> {
        match handle.0 {
            0 => Err("Cannot write to stdin".into()),
            1 => self.print_bytes_stdout(contents),
            2 => self.print_bytes_stderr(contents),
            n => self
                .frame_sinks
                .get(&n)
                .ok_or_else(|| format!("No file handle {n}"))
                .and_then(|sink| write_bytes(sink, contents)),
        }
    }

    // uninteresting boilerplate
    fn any(&self) -> &dyn std::any::Any {
        self
    }
    fn any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }
    fn print_str_stdout(&self, s: &str) -> Result<(), String> {
        self.print_bytes_stdout(s.as_bytes())
    }
    fn print_str_stderr(&self, s: &str) -> Result<(), String> {
        self.print_bytes_stderr(s.as_bytes())
    }
    fn print_bytes_stdout(&self, bytes: &[u8]) -> Result<(), String> {
        write_bytes(io::stdout().lock(), bytes)
    }
    fn print_bytes_stderr(&self, bytes: &[u8]) -> Result<(), String> {
        write_bytes(io::stderr().lock(), bytes)
    }
    fn print_str_trace(&self, s: &str) {
        eprint!("{s}");
        _ = io::stderr().flush();
    }
    fn term_size(&self) -> Result<(usize, usize), String> {
        let (w, h) = terminal_size::terminal_size().ok_or("Failed to get terminal size")?;
        Ok((w.0.into(), h.0.saturating_sub(1).into()))
    }
}

fn write_bytes(mut stream: impl io::Write, bytes: &[u8]) -> Result<(), String> {
    stream.write_all(bytes).map_err(|e| e.to_string())?;
    stream.flush().map_err(|e| e.to_string())?;
    Ok(())
}
