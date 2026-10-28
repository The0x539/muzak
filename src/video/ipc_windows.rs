use interprocess::os::windows::named_pipe::{
    pipe_mode, PipeListener, PipeListenerOptions, WaitTimeout,
};
use std::os::windows::io::AsRawHandle;

use super::Ipc;

pub enum WindowsNamedPipes {}

impl Ipc for WindowsNamedPipes {
    type Intermediate = PipeListener<pipe_mode::None, pipe_mode::Bytes>;
    type Sink = std::io::PipeWriter;

    fn register_source(
        cmd: &mut std::process::Command,
        id: usize,
    ) -> crate::Result<Self::Intermediate> {
        let pid = std::process::id();
        let path = format!(r"\\.\pipe\muzak-video-ipc-{pid}-{id}");

        cmd.args(["-i", &path]);

        let listener = PipeListenerOptions::new()
            .path(path)
            .wait_timeout(WaitTimeout::FOREVER)
            .create()?;

        Ok(listener)
    }

    fn obtain_sink(listener: Self::Intermediate) -> crate::Result<Self::Sink> {
        // FFmpeg wants to read a little bit from each stream before opening the next one.
        // This would normally be reasonable, but really shouldn't be necessary for the rawvideo format.
        //
        // On the other hand, this program really wants to get all the writable file handles at once,
        // in order to hand them off to Uiua in the form of an array of file descriptors.
        //
        // To solve this extremely annoying impasse, we can fire off a tiny stupid thread.
        // We get a writable handle immediately, and the thread "tanks" the blocking .accept() call.
        //
        // I thought using this annoyingly complex named pipe setup instead of inheriting a handle would help.
        // It appears not really?
        let (mut rx, tx) = std::io::pipe()?;
        std::thread::spawn(move || {
            let mut ipc_tx = listener.accept().unwrap();
            std::io::copy(&mut rx, &mut ipc_tx).unwrap();
        });
        Ok(tx)
    }

    fn sink_as_raw(sink: &Self::Sink) -> u64 {
        sink.as_raw_handle().addr() as u64
    }
}
