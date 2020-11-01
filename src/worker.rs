use std::io;
use std::path::PathBuf;
use std::string::ToString;
use std::sync::Arc;
use std::time::Duration;

use log::{debug, error};
use tokio::sync::Semaphore;

use crate::output::Output;
use crate::process::Process;
use crate::queue::Queue;
use crate::sync;

#[derive(Debug, Clone)]
pub struct Task {
    pub binary: String,
    pub timeout: Option<Duration>,
    pub dir: Option<PathBuf>,
    pub args: Vec<String>,
}

impl ToString for Task {
    fn to_string(&self) -> String {
        std::iter::once(&self.binary)
            .chain(self.args.iter())
            .map(|s| shlex::quote(&s))
            .collect::<Vec<_>>()
            .join(" ")
    }
}

pub type TaskQueue = Queue<Task>;

pub struct Worker {
    queue: Arc<TaskQueue>,
    output: Output,
    worker_shutdown: sync::Trigger,
    server_shutdown: sync::Trigger,
    max_parallel: Arc<Semaphore>,
    timeout: Option<Duration>,
    dir: Option<PathBuf>,
}

impl Worker {
    pub fn new(
        queue: Arc<TaskQueue>,
        output: Option<PathBuf>,
        server_shutdown: sync::Trigger,
        max_parallel: usize,
        timeout: Option<Duration>,
        dir: Option<PathBuf>,
    ) -> io::Result<Self> {
        let (worker_shutdown, _) = sync::condition();
        let output = match output {
            Some(path) => Output::file(path)?,
            None => Output::new(io::stdout()),
        };
        Ok(Worker {
            queue,
            output,
            worker_shutdown,
            server_shutdown,
            max_parallel: Arc::new(Semaphore::new(max_parallel)),
            timeout,
            dir,
        })
    }

    pub fn shutdown_notifer(&self) -> sync::Trigger {
        self.worker_shutdown.clone()
    }

    pub async fn process(&self) {
        let mut server_shutdown = self.server_shutdown.waiter();
        let mut worker_shutdown = self.worker_shutdown.waiter();
        tokio::select! {
            _ = self.process_inner() => {},
            _ = worker_shutdown.wait() => {},
            _ = server_shutdown.wait() => {},
        }
        debug!("queue worker shutting down");
    }

    pub async fn process_inner(&self) {
        loop {
            self.max_parallel.acquire().await.unwrap().forget();
            let mut task = self.queue.pop().await;
            task.timeout = task.timeout.or(self.timeout);
            task.dir = task.dir.or_else(|| self.dir.clone());
            let done = self.max_parallel.clone();
            let worker_shutdown = self.worker_shutdown.waiter();
            let server_shutdown = self.server_shutdown.waiter();
            // FIXME
            let stdout = self.output.add_source().unwrap();
            let stderr = self.output.add_source().unwrap();

            tokio::spawn(async move {
                let command = task.to_string();
                match Process::new(task, stdout, stderr, worker_shutdown, server_shutdown) {
                    Ok(mut process) => process.wait().await,
                    Err(err) => error!("error executing '{}': {}", command, err),
                }
                done.add_permits(1);
                debug!("process worker shutting down");
            });
        }
    }
}
