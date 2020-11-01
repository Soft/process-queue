use log::{debug, error};
use std::fs::OpenOptions;
use std::io::{self, Write};
use std::path::Path;

use anyhow::{bail, Result};
use tokio::io::{AsyncBufReadExt, BufReader, Lines};
use tokio::sync::mpsc::{unbounded_channel, UnboundedReceiver, UnboundedSender};
use tokio::sync::oneshot;

use crate::fd::{AsyncFd, Fd};
use crate::ffi::{pipe, Pipe};

pub struct Output {
    line_sender: UnboundedSender<String>,
}

pub struct Source {
    writer: Option<Fd>,
    prefix_sender: oneshot::Sender<String>,
}

impl Source {
    pub fn take_writer(&mut self) -> Option<Fd> {
        self.writer.take()
    }

    pub fn set_prefix(self, prefix: String) -> Result<()> {
        if self.prefix_sender.send(prefix).is_err() {
            bail!("setting source name failed");
        }
        Ok(())
    }
}

struct Forwarder {
    name: String,
    line_sender: UnboundedSender<String>,
    reader: Lines<BufReader<AsyncFd>>,
}

impl Forwarder {
    fn new(name: String, reader: AsyncFd, line_sender: UnboundedSender<String>) -> Self {
        let reader = BufReader::new(reader).lines();
        Self {
            name,
            line_sender,
            reader,
        }
    }

    async fn serve(&mut self) -> io::Result<()> {
        debug!("output forwarder '{}' started", self.name);
        while let Some(line) = self.reader.next_line().await? {
            let line = format!("{}{}", self.name, line);
            if self.line_sender.send(line).is_err() {
                debug!("error forwarding output from '{}'", self.name);
                break;
            }
        }
        debug!("output forwarder '{}' shutting down", self.name);
        Ok(())
    }
}

struct Consumer<W: Write> {
    sink: W,
    line_receiver: UnboundedReceiver<String>,
}

impl<W> Consumer<W>
where
    W: Write,
{
    fn new(sink: W, line_receiver: UnboundedReceiver<String>) -> Self {
        Self {
            sink,
            line_receiver,
        }
    }

    async fn consume(&mut self) {
        debug!("output consumer started");
        while let Some(line) = self.line_receiver.recv().await {
            let _ = writeln!(self.sink, "{}", line);
            let _ = self.sink.flush();
        }
        debug!("output consumer shutting down");
    }
}

impl Output {
    pub fn file<P>(path: P) -> io::Result<Self>
    where
        P: AsRef<Path>,
    {
        let sink = OpenOptions::new()
            .append(true)
            .create(true)
            .open(path.as_ref())?;
        let sink = io::BufWriter::new(sink);
        Ok(Self::new(sink))
    }

    pub fn new<W>(sink: W) -> Self
    where
        W: Write + Send + 'static,
    {
        let (line_sender, line_receiver) = unbounded_channel();
        tokio::spawn(async move {
            let mut consumer = Consumer::new(sink, line_receiver);
            consumer.consume().await;
        });
        Self { line_sender }
    }

    pub fn add_source(&self) -> io::Result<Source> {
        let Pipe { reader, writer } = pipe()?;
        let reader = Fd::new(reader);
        let writer = Fd::new(writer);
        let reader = AsyncFd::from_blocking(reader)?;
        let (prefix_sender, prefix_receiver) = oneshot::channel();
        let line_sender = self.line_sender.clone();
        tokio::spawn(async move {
            let name = match prefix_receiver.await {
                Ok(name) => name,
                Err(_) => {
                    error!("getting source prefix failed");
                    return;
                }
            };
            let mut forwarder = Forwarder::new(name, reader, line_sender);
            if let Err(err) = forwarder.serve().await {
                error!("output forwarding error: {}", err);
            }
        });
        Ok(Source {
            writer: Some(writer),
            prefix_sender,
        })
    }
}
