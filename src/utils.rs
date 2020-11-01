use std::io;
use std::path::PathBuf;

use log::SetLoggerError;
use simplelog::{LevelFilter, WriteLogger};

#[macro_export]
macro_rules! impl_trivial_from {
    ($s:ty, $t:ty, $i:ident) => {
        impl From<$s> for $t {
            fn from(resp: $s) -> Self {
                Self::$i(resp)
            }
        }
    };
}

pub fn init_logger<W>(level_filter: LevelFilter, sink: W) -> Result<(), SetLoggerError>
where
    W: io::Write + std::marker::Send + 'static,
{
    WriteLogger::init(level_filter, Default::default(), sink)
}

pub fn get_runtime() -> io::Result<tokio::runtime::Runtime> {
    tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
}

pub fn spawn_signal_handler<F>(notify: F)
where
    F: Fn() + Send + 'static,
{
    tokio::spawn(async move {
        if tokio::signal::ctrl_c().await.is_ok() {
            notify();
        }
    });
}

pub struct FileRemover(PathBuf);

impl FileRemover {
    pub fn new<P: Into<PathBuf>>(path: P) -> FileRemover {
        FileRemover(path.into())
    }
}

impl Drop for FileRemover {
    fn drop(&mut self) {
        let _ = std::fs::remove_file(&self.0);
    }
}
