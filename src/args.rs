use simplelog::LevelFilter;
use std::borrow::Cow;
use std::path::{Path, PathBuf};
use std::time::Duration;
use structopt::StructOpt;

use crate::duration::parse_duration;
use crate::template::Template;

#[derive(StructOpt)]
pub struct StartServerCommand {
    #[structopt(short = "f", long, help = "Keep pqueue server in the foreground")]
    pub foreground: bool,
    #[structopt(short = "v", parse(from_occurrences = parse_log_level), help = "Log level")]
    pub log_level: LevelFilter,
    #[structopt(short = "l", long, help = "Log file")]
    pub log_file: Option<PathBuf>,
}

#[derive(StructOpt)]
pub struct CreateQueueCommand {
    #[structopt(short = "n", long, default_value = "default", help = "Queue name")]
    pub name: String,
    #[structopt(
        short = "p",
        long,
        default_value = "1",
        help = "Maximum number of parallel tasks"
    )]
    pub max_parallel: usize,
    #[structopt(short = "f", long, help = "Output to file")]
    pub file: Option<PathBuf>,
    #[structopt(short = "s", long, help = "Output to stdin", conflicts_with("output"))]
    pub stdout: bool,
    #[structopt(short = "d", long, help = "Default working directory")]
    pub dir: Option<PathBuf>,
    #[structopt(short = "T", long, help = "Default task timeout", parse(try_from_str = parse_duration))]
    pub timeout: Option<Duration>,
    #[structopt(short = "t", long, help = "Task template")]
    pub template: Option<Template>,
}

#[derive(StructOpt)]
pub struct RemoveQueueCommand {
    #[structopt(short = "n", long, default_value = "default", help = "Queue name")]
    pub name: String,
}

#[derive(StructOpt)]
pub struct SendTaskCommand {
    #[structopt(short = "n", long, default_value = "default", help = "Task name")]
    pub name: String,
    #[structopt(short = "d", long, help = "Working directory")]
    pub dir: Option<PathBuf>,
    #[structopt(short = "T", long, help = "Task timeout", parse(try_from_str = parse_duration))]
    pub timeout: Option<Duration>,
    pub args: Vec<String>,
}

#[derive(StructOpt)]
pub struct ListTasksCommand {
    #[structopt(short = "n", long, default_value = "default", help = "Task name")]
    pub name: String,
}

#[derive(StructOpt)]
pub enum Command {
    #[structopt(
        about = "Start queue server",
        visible_alias = "start",
        display_order = 0
    )]
    StartServer(StartServerCommand),
    #[structopt(about = "Stop queue server", visible_alias = "stop", display_order = 1)]
    StopServer,
    #[structopt(
        about = "Create new task queue",
        visible_alias = "create",
        display_order = 2
    )]
    CreateQueue(CreateQueueCommand),
    #[structopt(
        about = "Remove task queue",
        visible_alias = "remove",
        display_order = 3
    )]
    RemoveQueue(RemoveQueueCommand),
    #[structopt(about = "List queues", visible_alias = "queues", display_order = 4)]
    ListQueues,
    #[structopt(
        about = "Send task to a queue",
        visible_alias = "send",
        display_order = 5
    )]
    SendTask(SendTaskCommand),
    #[structopt(about = "List tasks in a queue", visible_alias = "tasks", display_order = 6)]
    ListTasks(ListTasksCommand),
}

#[derive(StructOpt)]
pub struct GlobalArgs {
    #[structopt(short = "s", long, help = "Server socket path")]
    pub socket: Option<PathBuf>,
}

#[derive(StructOpt)]
#[structopt(name = "pqueue", about = "Task queue")]
pub struct Args {
    #[structopt(flatten)]
    pub global: GlobalArgs,
    #[structopt(subcommand)]
    pub command: Command,
}

impl GlobalArgs {
    pub fn socket(&self) -> Cow<'_, Path> {
        self.socket
            .as_ref()
            .map(Cow::from)
            .unwrap_or_else(|| Cow::from(default_socket_path()))
    }
}

fn default_socket_path() -> PathBuf {
    let uid = unsafe { libc::getuid() };
    let mut path = std::env::temp_dir();
    path.push(format!("pqueue-{}", uid));
    path
}

fn parse_log_level(occurrences: u64) -> LevelFilter {
    match occurrences {
        0 => LevelFilter::Off,
        1 => LevelFilter::Error,
        2 => LevelFilter::Warn,
        3 => LevelFilter::Info,
        4 => LevelFilter::Debug,
        _ => LevelFilter::Trace,
    }
}
