use anyhow::{anyhow, Result};
use std::fs::{self};
use std::io::{self, Read, Write};
use std::os::unix::io::{AsRawFd, IntoRawFd};

use crate::fd::Fd;
use crate::ffi::{dup2, fork, pipe, setsid, Fork, Pipe};

pub struct ReadinessWaiter {
    reader: Fd,
}

impl ReadinessWaiter {
    pub fn wait(mut self) -> Result<()> {
        let mut buf: [u8; 1] = [0];
        let bytes = self.reader.read(&mut buf)?;
        if bytes == 1 && buf[0] == b'\n' {
            Ok(())
        } else {
            Err(anyhow!("protocol error"))
        }
    }
}

pub struct ReadinessNotifier {
    writer: Fd,
}

impl ReadinessNotifier {
    pub fn notify(mut self) -> io::Result<()> {
        self.writer.write_all(&[b'\n'])
    }
}

pub enum Daemonize {
    Parent(ReadinessWaiter),
    Child(ReadinessNotifier),
}

pub fn daemonize() -> io::Result<Daemonize> {
    let Pipe { reader, writer } = pipe()?;
    let reader = Fd::new(reader);
    let writer = Fd::new(writer);
    match fork()? {
        Fork::Parent(..) => {
            drop(writer);
            Ok(Daemonize::Parent(ReadinessWaiter { reader }))
        }
        Fork::Child => {
            drop(reader);
            setsid()?;
            std::env::set_current_dir("/")?;
            let dev_null = fs::OpenOptions::new()
                .read(true)
                .write(true)
                .open("/dev/null")?
                .into_raw_fd();
            for fd in &[
                io::stdin().as_raw_fd(),
                io::stdout().as_raw_fd(),
                io::stderr().as_raw_fd(),
            ] {
                dup2(dev_null, *fd)?;
            }
            Ok(Daemonize::Child(ReadinessNotifier { writer }))
        }
    }
}
