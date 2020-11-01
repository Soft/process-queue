use anyhow::Result;
use tokio::net::UnixListener;

use crate::args::{
    CreateQueueCommand, GlobalArgs, ListTasksCommand, RemoveQueueCommand, SendTaskCommand,
    StartServerCommand,
};
use crate::client::QueueClient;
use crate::daemon::{daemonize, Daemonize};
use crate::server::QueueServer;
use crate::utils;

pub fn start_server(args: GlobalArgs, command: StartServerCommand) -> Result<()> {
    let mut notifier = None;

    if !command.foreground {
        notifier = match daemonize()? {
            Daemonize::Parent(waiter) => {
                waiter.wait()?;
                return Ok(());
            }
            Daemonize::Child(notifier) => Some(notifier),
        };
    }

    if let Some(path) = command.log_file {
        let log_file = std::fs::OpenOptions::new()
            .append(true)
            .create(true)
            .open(path)?;
        utils::init_logger(command.log_level, log_file)?;
    } else if command.foreground {
        utils::init_logger(command.log_level, std::io::stderr())?;
    }

    let path = args.socket();
    let _ = std::fs::remove_file(&path);

    let rt = utils::get_runtime()?;
    rt.block_on(async {
        let listener = UnixListener::bind(&path)?;
        let _socket = utils::FileRemover::new(path);

        if let Some(notifier) = notifier {
            notifier.notify()?;
        }

        let server = QueueServer::new(listener)?;
        let shutdown = server.shutdown_notifer();
        utils::spawn_signal_handler(shutdown);
        server.serve().await
    })
}

pub async fn stop_server(args: GlobalArgs) -> Result<()> {
    let path = args.socket();
    let mut client = QueueClient::connect(path).await?;
    client.stop_server().await?;
    Ok(())
}

pub async fn create_queue(args: GlobalArgs, command: CreateQueueCommand) -> Result<()> {
    let path = args.socket();
    let mut client = QueueClient::connect(path).await?;
    client
        .create_queue(
            command.name,
            command.max_parallel,
            command.file,
            command.timeout,
            command.dir,
            command.template,
        )
        .await?;
    Ok(())
}

pub async fn remove_queue(args: GlobalArgs, command: RemoveQueueCommand) -> Result<()> {
    let path = args.socket();
    let mut client = QueueClient::connect(path).await?;
    client.remove_queue(command.name).await?;
    Ok(())
}

pub async fn send(args: GlobalArgs, command: SendTaskCommand) -> Result<()> {
    let path = args.socket();
    let mut client = QueueClient::connect(path).await?;
    client
        .send(command.name, command.timeout, command.dir, command.args)
        .await?;
    Ok(())
}

pub async fn list_queues(args: GlobalArgs) -> Result<()> {
    let path = args.socket();
    let mut client = QueueClient::connect(path).await?;
    for queue in client.list_queues().await?.queues {
        println!("{}", queue.name);
    }
    Ok(())
}

pub async fn list_tasks(args: GlobalArgs, command: ListTasksCommand) -> Result<()> {
    let path = args.socket();
    let mut client = QueueClient::connect(path).await?;
    for task in client.list_tasks(command.name).await?.tasks {
        println!(
            "{}",
            task.args
                .iter()
                .map(|s| shlex::quote(&s))
                .collect::<Vec<_>>()
                .join(" ")
        );
    }
    Ok(())
}
