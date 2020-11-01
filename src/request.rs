use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::time::Duration;

use crate::impl_trivial_from;
use crate::template::Template;

#[derive(Debug, Serialize, Deserialize)]
pub struct CreateQueue {
    pub name: String,
    pub max_parallel: usize,
    pub output: Option<PathBuf>,
    pub timeout: Option<Duration>,
    pub dir: Option<PathBuf>,
    pub template: Option<Template>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct RemoveQueue {
    pub name: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Send {
    pub name: String,
    pub timeout: Option<Duration>,
    pub dir: Option<PathBuf>,
    pub args: Vec<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ListTasks {
    pub name: String,
}
#[derive(Debug, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Request {
    StopServer,
    CreateQueue(CreateQueue),
    RemoveQueue(RemoveQueue),
    Send(Send),
    ListQueues,
    ListTasks(ListTasks),
}

impl_trivial_from!(CreateQueue, Request, CreateQueue);
impl_trivial_from!(RemoveQueue, Request, RemoveQueue);
impl_trivial_from!(Send, Request, Send);
impl_trivial_from!(ListTasks, Request, ListTasks);
