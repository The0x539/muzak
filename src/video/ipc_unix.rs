use command_fds::CommandFdExt;
use std::io::PipeWriter;
use std::os::fd::AsRawFd;

use super::Ipc;

pub enum UnixAnonymousPipes {}

impl Ipc for UnixAnonymousPipes {
    type Intermediate = PipeWriter;
    type Sink = PipeWriter;

    fn register_source(
        cmd: &mut std::process::Command,
        _id: usize,
    ) -> crate::Result<Self::Intermediate> {
        let (source, sink) = std::io::pipe()?;
        let raw = source.as_raw_fd();
        cmd.args(["-i", &format!("pipe:{raw}")]);
        cmd.preserved_fds(vec![source.into()]);
        Ok(sink)
    }

    fn obtain_sink(pipe: Self::Intermediate) -> crate::Result<Self::Sink> {
        Ok(pipe)
    }

    fn sink_as_raw(sink: &Self::Sink) -> u64 {
        sink.as_raw_fd() as u64
    }
}
