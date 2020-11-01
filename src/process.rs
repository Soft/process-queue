use std::collections::HashSet;
use std::io;
use std::os::unix::io::FromRawFd;
use std::os::unix::io::RawFd;
use std::process::Stdio;
use std::string::ToString;

use anyhow::Result;
use log::{error, info, warn};
use tokio::process::{Child, Command};

use crate::ffi;
use crate::output::Source;
use crate::sync::Waiter;
use crate::worker::Task;

pub struct Process {
    child: Child,
    task: Task,
    worker_shutdown: Waiter,
    server_shutdown: Waiter,
}

#[cfg(target_os = "linux")]
const FD_DIR: &str = "/proc/self/fd";

#[cfg(all(target_family = "unix", not(target_os = "linux")))]
const FD_DIR: &str = "/dev/fd";

fn close_fds(keep: HashSet<RawFd>) -> io::Result<()> {
    // FIXME: This might be problematic on some platforms as this likely
    // allocates which might lead to deadlocks on rare occasions if fork left
    // some allocator mutex locked. I have not observed this behavior on Linux.
    let open_fds: io::Result<Vec<RawFd>> = std::fs::read_dir(FD_DIR)?
        .map(|entry| {
            entry
                .map(|path| path.file_name())
                .and_then(|name| {
                    name.into_string()
                        .map_err(|_| io::Error::new(io::ErrorKind::Other, "invalid fd"))
                })
                .and_then(|name| {
                    name.parse()
                        .map_err(|_| io::Error::new(io::ErrorKind::Other, "invalid fd"))
                })
        })
        .collect();
    for fd in open_fds?.into_iter() {
        if !keep.contains(&fd) {
            let _ = ffi::close(fd);
        }
    }
    Ok(())
}

impl Process {
    pub fn new(
        task: Task,
        mut stdout: Source,
        mut stderr: Source,
        worker_shutdown: Waiter,
        server_shutdown: Waiter,
    ) -> Result<Self> {
        let mut command = Command::new(&task.binary);
        command.stdin(Stdio::null());
        command.stdout(unsafe { Stdio::from_raw_fd(stdout.take_writer().unwrap().into()) });
        command.stderr(unsafe { Stdio::from_raw_fd(stderr.take_writer().unwrap().into()) });
        unsafe {
            command.pre_exec(|| close_fds([0 as RawFd, 1, 2].iter().cloned().collect()));
        };
        command.args(&task.args);
        if let Some(ref dir) = task.dir {
            command.current_dir(dir);
        }
        let child = command.spawn()?;
        // We have never polled child so id() should never return None
        let pid = child.id().map(|pid| pid.to_string()).unwrap();
        stdout.set_prefix(format!("[{}:stdout]: ", pid)).unwrap();
        stderr.set_prefix(format!("[{}:stderr]: ", pid)).unwrap();
        Ok(Self {
            child,
            task,
            worker_shutdown,
            server_shutdown,
        })
    }

    pub async fn wait(&mut self) {
        if let Some(duration) = self.task.timeout {
            if tokio::time::timeout(duration, self.wait_inner())
                .await
                .is_err()
            {
                warn!("execution of '{}' timed out", self.task.to_string());
                let _ = self.child.start_kill();
            }
        } else {
            self.wait_inner().await
        }
    }

    async fn wait_inner(&mut self) {
        tokio::select! {
            ret = self.child.wait() => match ret {
                Ok(ret) => info!("execution of '{}' finished: {}", self.task.to_string(), ret),
                Err(err) => error!("error executing '{}': {}", self.task.to_string(), err),
            },
            _ = self.worker_shutdown.wait() => {
                let _ = self.child.start_kill();
            },
            _ = self.server_shutdown.wait() => {
                let _ = self.child.start_kill();
            },
        }
    }
}
