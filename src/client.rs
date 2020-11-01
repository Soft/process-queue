use std::io;
use std::path::{Path, PathBuf};
use std::time::Duration;

use anyhow::{anyhow, Context, Result};
use tokio::net::UnixStream;

use crate::connection::Connection;
use crate::request::{self, Request};
use crate::response::{self, Response};
use crate::template::Template;

pub struct QueueClient {
    connection: Connection,
}

impl QueueClient {
    pub async fn connect<P>(path: P) -> Result<Self>
    where
        P: AsRef<Path>,
    {
        let stream = match UnixStream::connect(path).await {
            Ok(stream) => stream,
            Err(err)
                if err.kind() == io::ErrorKind::ConnectionRefused
                    || err.kind() == io::ErrorKind::NotFound =>
            {
                return Err(err).context("server is not running");
            }
            Err(err) => return Err(err.into()),
        };
        let connection = Connection::new(stream);
        Ok(Self { connection })
    }

    pub async fn stop_server(&mut self) -> Result<response::Empty> {
        self.request(Request::StopServer).await
    }

    pub async fn create_queue(
        &mut self,
        name: String,
        max_parallel: usize,
        output: Option<PathBuf>,
        timeout: Option<Duration>,
        dir: Option<PathBuf>,
        template: Option<Template>,
    ) -> Result<response::Empty> {
        let request = request::CreateQueue {
            name,
            max_parallel,
            output,
            timeout,
            dir,
            template,
        };
        self.request(request).await
    }

    pub async fn remove_queue(&mut self, name: String) -> Result<response::Empty> {
        let request = request::RemoveQueue { name };
        self.request(request).await
    }

    pub async fn send(
        &mut self,
        name: String,
        timeout: Option<Duration>,
        dir: Option<PathBuf>,
        args: Vec<String>,
    ) -> Result<response::Empty> {
        let request = request::Send {
            name,
            dir,
            timeout,
            args,
        };
        self.request(request).await
    }

    pub async fn list_queues(&mut self) -> Result<response::ListQueues> {
        self.request(Request::ListQueues).await
    }

    pub async fn list_tasks(&mut self, name: String) -> Result<response::ListTasks> {
        let request = request::ListTasks { name };
        self.request(request).await
    }

    async fn request<T, R>(&mut self, request: T) -> Result<R>
    where
        T: Into<Request>,
        R: serde::de::DeserializeOwned,
    {
        let request: Request = request.into();
        self.connection.write_message(&request).await?;
        self.connection
            .read_message::<Response<R>>()
            .await?
            .ok_or_else(|| anyhow!("server did not respond"))?
            .into()
    }
}
