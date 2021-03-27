use anyhow::{bail, Result};
use log::{error, info};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::net::{UnixListener, UnixStream};
use tokio::sync::Mutex;

use crate::connection::Connection;
use crate::request::{self, Request};
use crate::response::{self, Response};
use crate::sync::{condition, DropGuard, DropWaiter, Trigger};
use crate::template::Template;
use crate::worker::{Task, TaskQueue, Worker};

struct WorkerHandle {
    queue: Arc<TaskQueue>,
    template: Option<Template>,
    shutdown: Trigger,
}

impl WorkerHandle {
    fn expand_args(&self, args: Vec<String>) -> Result<Vec<String>> {
        if let Some(template) = &self.template {
            template.instantiate(args)
        } else {
            Ok(args)
        }
    }
}

type QueueMap = Arc<Mutex<HashMap<String, WorkerHandle>>>;

struct ClientHandler {
    connection: Connection,
    queues: QueueMap,
    shutdown_requested: bool,
    shutdown: Trigger,
    _shutdown_sentinel: DropGuard,
}

impl ClientHandler {
    pub fn new(
        connection: UnixStream,
        queues: QueueMap,
        shutdown: Trigger,
        shutdown_sentinel: DropGuard,
    ) -> Self {
        let connection = Connection::new(connection);
        let shutdown_requested = false;
        ClientHandler {
            connection,
            queues,
            shutdown_requested,
            shutdown,
            _shutdown_sentinel: shutdown_sentinel,
        }
    }

    pub async fn serve(&mut self) -> Result<()> {
        let mut shutdown = self.shutdown.waiter();
        tokio::select! {
            ret = self.serve_inner() => {
                if self.shutdown_requested {
                    let _ = self.shutdown.set();
                }
                ret
            },
            _ = shutdown.wait() => Ok(()),
        }
    }

    async fn serve_inner(&mut self) -> Result<()> {
        while !self.shutdown_requested {
            let request = match self.connection.read_message().await? {
                Some(message) => message,
                None => return Ok(()),
            };
            let resp = self.handle_request(request).await;
            self.connection.write_message(&resp).await?;
        }
        Ok(())
    }

    async fn handle_request(&mut self, req: Request) -> Response {
        match req {
            Request::StopServer => self.handle_stop_server().await.into(),
            Request::CreateQueue(req) => self.handle_create_queue(req).await.into(),
            Request::RemoveQueue(req) => self.handle_remove_queue(req).await.into(),
            Request::Send(req) => self.handle_send(req).await.into(),
            Request::ListQueues => self.handle_list_queues().await.into(),
            Request::ListTasks(req) => self.handle_list_tasks(req).await.into(),
        }
    }

    async fn handle_stop_server(&mut self) -> Result<response::Empty> {
        info!("shutdown requested");
        self.shutdown_requested = true;
        response::ok()
    }

    async fn handle_create_queue(&self, req: request::CreateQueue) -> Result<response::Empty> {
        let mut map = self.queues.lock().await;
        if map.contains_key(&req.name) {
            bail!("queue '{}' already exists", &req.name);
        }

        info!("queue '{}' created", req.name);

        let queue = Arc::new(TaskQueue::new());
        let worker = Worker::new(
            queue.clone(),
            req.output,
            self.shutdown.clone(),
            req.max_parallel,
            req.timeout,
            req.dir,
        )?;
        let worker_handle = WorkerHandle {
            queue,
            template: req.template,
            shutdown: worker.shutdown_notifer(),
        };

        tokio::spawn(async move {
            worker.process().await;
        });

        map.insert(req.name, worker_handle);

        response::ok()
    }

    async fn handle_remove_queue(&self, req: request::RemoveQueue) -> Result<response::Empty> {
        let mut map = self.queues.lock().await;
        if let Some(worker) = map.remove(&req.name) {
            worker.shutdown.set();
            response::ok()
        } else {
            bail!("queue '{}' does not exist", &req.name);
        }
    }

    async fn handle_send(&self, req: request::Send) -> Result<response::Empty> {
        let map = self.queues.lock().await;
        if let Some(worker) = map.get(&req.name) {
            let mut args = worker.expand_args(req.args)?;
            if args.is_empty() {
                bail!("command cannot be empty");
            }
            let binary = args.remove(0);

            let task = Task {
                binary,
                timeout: req.timeout,
                dir: req.dir,
                args,
            };
            info!("received task '{}'", task.to_string());
            worker.queue.push(task).await;
            response::ok()
        } else {
            bail!("queue '{}' does not exist", &req.name);
        }
    }

    async fn handle_list_queues(&self) -> Result<response::ListQueues> {
        let queues = self
            .queues
            .lock()
            .await
            .keys()
            .cloned()
            .map(|name| response::Queue { name })
            .collect();
        Ok(response::ListQueues { queues })
    }

    async fn handle_list_tasks(&self, req: request::ListTasks) -> Result<response::ListTasks> {
        let map = self.queues.lock().await;
        if let Some(worker) = map.get(&req.name) {
            let tasks = worker
                .queue
                .collect::<Vec<Task>>()
                .await
                .into_iter()
                .map(|task| response::Task { args: task.args })
                .collect();
            Ok(response::ListTasks { tasks })
        } else {
            bail!("queue '{}' does not exist", &req.name);
        }
    }
}

pub struct QueueServer {
    listener: UnixListener,
    queues: QueueMap,
    shutdown: Trigger,
    shutdown_waiter: DropWaiter,
}

impl QueueServer {
    pub fn new(listener: UnixListener) -> Result<Self> {
        let queues = Arc::new(Mutex::new(HashMap::new()));
        let (shutdown, _) = condition();
        let shutdown_waiter = DropWaiter::new();
        Ok(Self {
            listener,
            queues,
            shutdown,
            shutdown_waiter,
        })
    }

    pub fn shutdown_notifer(&self) -> impl Fn() + Send + 'static {
        let shutdown = self.shutdown.clone();
        move || {
            shutdown.set();
        }
    }

    pub async fn serve(mut self) -> Result<()> {
        let mut shutdown = self.shutdown.waiter();

        let ret = tokio::select! {
            ret = self.serve_inner() => ret,
            _ = shutdown.wait() => Ok(()),
        };

        self.shutdown_waiter.wait().await;
        ret
    }

    async fn serve_inner(&mut self) -> Result<()> {
        loop {
            let (connection, _) = self.listener.accept().await?;
            let queues = self.queues.clone();
            let shutdown = self.shutdown.clone();
            let sentinel = self.shutdown_waiter.guard();
            tokio::spawn(async move {
                let mut client = ClientHandler::new(connection, queues, shutdown, sentinel);
                if let Err(err) = client.serve().await {
                    error!("client error: {}", err);
                }
            });
        }
    }
}
