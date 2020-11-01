use std::collections::VecDeque;
use std::iter::FromIterator;
use tokio::sync::{Mutex, Semaphore};

pub struct Queue<T> {
    queue: Mutex<VecDeque<T>>,
    semaphore: Semaphore,
}

impl<T> Queue<T> {
    pub fn new() -> Self {
        Queue {
            queue: Mutex::new(VecDeque::new()),
            semaphore: Semaphore::new(0),
        }
    }

    pub async fn push(&self, task: T) {
        self.queue.lock().await.push_back(task);
        self.semaphore.add_permits(1);
    }

    pub async fn pop(&self) -> T {
        self.semaphore.acquire().await.unwrap().forget();
        self.queue.lock().await.pop_front().expect("empty queue")
    }
}

impl<T: Clone> Queue<T> {
    pub async fn collect<B>(&self) -> B
    where
        B: FromIterator<T>,
    {
        self.queue.lock().await.iter().cloned().collect()
    }
}
