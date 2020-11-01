use tokio::sync::broadcast::{self, Receiver, Sender};
use tokio::sync::mpsc;

#[derive(Clone)]
pub struct Trigger(Sender<()>);

impl Trigger {
    pub fn waiter(&self) -> Waiter {
        Waiter::new(self.0.subscribe())
    }

    pub fn set(&self) {
        let _ = self.0.send(());
    }
}

pub struct Waiter {
    receiver: Receiver<()>,
    set: bool,
}

impl Waiter {
    fn new(receiver: Receiver<()>) -> Self {
        Self {
            receiver,
            set: false,
        }
    }

    pub async fn wait(&mut self) {
        if self.set {
            return;
        }
        let _ = self.receiver.recv().await;
        self.set = true;
    }
}

pub fn condition() -> (Trigger, Waiter) {
    let (tx, rx) = broadcast::channel(1);
    (Trigger(tx), Waiter::new(rx))
}

#[derive(Clone)]
pub struct DropGuard(mpsc::Sender<()>);

pub struct DropWaiter {
    sender: mpsc::Sender<()>,
    receiver: mpsc::Receiver<()>,
}

impl DropWaiter {
    pub fn new() -> Self {
        let (sender, receiver) = mpsc::channel(1);
        Self { sender, receiver }
    }

    pub fn guard(&self) -> DropGuard {
        DropGuard(self.sender.clone())
    }

    pub async fn wait(mut self) {
        drop(self.sender);
        self.receiver.recv().await;
    }
}
