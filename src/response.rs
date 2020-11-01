use crate::impl_trivial_from;
use anyhow::anyhow;

use serde::{Deserialize, Serialize};

pub fn ok<E>() -> Result<Empty, E> {
    Ok(Empty {})
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Empty {}

#[derive(Debug, Serialize, Deserialize)]
pub struct Queue {
    pub name: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ListQueues {
    pub queues: Vec<Queue>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Task {
    pub args: Vec<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ListTasks {
    pub tasks: Vec<Task>,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(untagged)]
pub enum Any {
    Empty(Empty),
    ListQueues(ListQueues),
    ListTasks(ListTasks),
}

pub trait ToAny: Into<Any> {}
impl ToAny for Empty {}
impl ToAny for ListQueues {}
impl ToAny for ListTasks {}

impl_trivial_from!(Empty, Any, Empty);
impl_trivial_from!(ListQueues, Any, ListQueues);
impl_trivial_from!(ListTasks, Any, ListTasks);

#[derive(Debug, Serialize, Deserialize)]
pub struct Error {
    pub message: String,
}

impl<E> From<E> for Error
where
    E: ToString,
{
    fn from(err: E) -> Error {
        Error {
            message: err.to_string(),
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(tag = "status", rename_all = "snake_case")]
pub enum Response<T = Any> {
    Success(T),
    Error(Error),
}

impl<T> From<Response<T>> for Result<T, anyhow::Error> {
    fn from(resp: Response<T>) -> Self {
        match resp {
            Response::Success(v) => Ok(v),
            Response::Error(err) => Err(anyhow!(err.message)),
        }
    }
}

impl<T, E> From<Result<T, E>> for Response<Any>
where
    T: ToAny,
    E: Into<Error>,
{
    fn from(res: Result<T, E>) -> Response<Any> {
        match res {
            Ok(v) => Response::Success(v.into()),
            Err(err) => Response::Error(err.into()),
        }
    }
}
