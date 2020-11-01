mod args;
mod client;
mod command;
mod connection;
mod daemon;
mod duration;
mod fd;
mod ffi;
mod output;
mod process;
mod queue;
mod request;
mod response;
mod server;
mod sync;
mod template;
mod utils;
mod worker;

use anyhow::Result;
use args::{Args, Command};
use structopt::StructOpt;

fn run() -> Result<()> {
    let args = Args::from_args();

    if let Command::StartServer(start) = args.command {
        return command::start_server(args.global, start);
    }

    let rt = utils::get_runtime()?;

    rt.block_on(async {
        match args.command {
            Command::StopServer => command::stop_server(args.global).await,
            Command::CreateQueue(create) => command::create_queue(args.global, create).await,
            Command::RemoveQueue(remove) => command::remove_queue(args.global, remove).await,
            Command::SendTask(send) => command::send(args.global, send).await,
            Command::ListQueues => command::list_queues(args.global).await,
            Command::ListTasks(list_tasks) => command::list_tasks(args.global, list_tasks).await,
            Command::StartServer(..) => unreachable!(),
        }
    })
}

fn main() {
    if let Err(err) = run() {
        eprintln!("pqueue: {}", err);
        std::process::exit(1)
    }
}
