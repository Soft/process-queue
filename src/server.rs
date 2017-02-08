use std;
use std::error::Error;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::mpsc::{channel, Sender};
use std::sync::{Arc, Mutex};
use std::thread;

use dbus::tree::Factory;
use dbus::{Connection, BusType, NameFlag};

use common::{get_dbus_name, DBUS_INTERFACE};
use templates::{Template, TemplateError};

#[derive(Debug)]
struct Args(Vec<String>);

pub struct Server {
    name: String,
    command: String,
    dir: PathBuf,
    template: Template,
    retries: usize
}

#[derive(Debug)]
pub enum ExecError {
    TemplateError(TemplateError),
    IOError(std::io::Error),
    RetryError
}

impl From<TemplateError> for ExecError {
    fn from(err: TemplateError) -> Self {
        ExecError::TemplateError(err)
    }
}

impl From<std::io::Error> for ExecError {
    fn from(err: std::io::Error) -> Self {
        ExecError::IOError(err)
    }
}

#[derive(Debug)]
enum ServerState {
    Running,
    Stopped
}

impl Server {
    pub fn new<N, C, T, P>(name: N,
                           command: C,
                           dir: P,
                           template: T,
                           retries: usize) -> Self
        where N: Into<String>,
              C: Into<String>,
              P: AsRef<Path>,
              T: Into<Template> {
        Server {
            name: name.into(),
            command: command.into(),
            dir: dir.as_ref().to_owned(),
            template: template.into(),
            retries: retries
        }
    }

    pub fn run(&self) {
        info!("starting pqueue server with name \"{}\"", &self.name);
        let (sender, receiver) = channel::<Args>();
        let name = self.name.clone();
        let state = Arc::new(Mutex::new(ServerState::Running));
        let state_clone = state.clone();

        thread::spawn(move || {
            setup_dbus_server(&name, state_clone, sender);
        });

        while let Ok(Args(args)) = receiver.recv() {
            match self.exec(&args) {
                Ok(_) =>
                    info!("succesfully executed \"{}\"", &self.command),
                Err(ExecError::TemplateError(TemplateError::ArgumentCountMismatch)) =>
                    error!("failed to execute \"{}\": arguments do not fit the template", &self.command),
                Err(ExecError::IOError(err)) =>
                    error!("failed to execute \"{}\": {}", &self.command, err.description()),
                Err(ExecError::RetryError) =>
                    error!("failed to execute \"{}\": retry count exceeded", &self.command)
            }

            match *state.lock().unwrap() {
                ServerState::Stopped => break,
                _ => {}
            }
        }
    }

    pub fn exec<S>(&self, args: &[S]) -> Result<(), ExecError>
        where S: AsRef<str> {
        let args = self.template.fill(args)?;
        for _ in 0..self.retries + 1 {
            info!("executing \"{}\" with arguments {:?}", &self.command, args);
            let mut child = Command::new(&self.command)
                .current_dir(&self.dir)
                .stdin(Stdio::null())
                .args(&args)
                .spawn()?;
            if child.wait()?.success() {
                return Ok(());
            } else {
                error!("non-zero status returned from {}", &self.command);
            }
        }
        Err(ExecError::RetryError)
    }
}

fn setup_dbus_server(name: &str, state: Arc<Mutex<ServerState>>, sender: Sender<Args>) {
    let name = get_dbus_name(name)
                .expect("invalid server name");
    let conn = Connection::get_private(BusType::Session)
        .expect("failed to connect DBus");
    conn.register_name(&name, NameFlag::ReplaceExisting as u32)
        .unwrap();
    let fact = Factory::new_fn::<()>();
    let state_clone = state.clone();
    let tree = fact.tree(()).add(
        fact.object_path("/", ()).introspectable().add(
            fact.interface(DBUS_INTERFACE, ()).add_m(
                fact.method("add", (), move |m| {
                    // TODO: remove unwrap
                    let args: Vec<String> = m.msg.get1().unwrap();
                    info!("received new task with arguments {:?}", &args);
                    let reply = m.msg.method_return();
                    sender.send(Args(args)).unwrap();
                    Ok(vec!(reply))
                }).inarg::<Vec<String>, _>("args")
            ).add_m(
                fact.method("stop", (), move |m| {
                    info!("received stop request");
                    let mut state = state_clone.lock().unwrap();
                    *state = ServerState::Stopped;
                    let reply = m.msg.method_return();
                    Ok(vec!(reply))
                })
            )
        )
    );
    tree.set_registered(&conn, true).unwrap();
    for _ in tree.run(&conn, conn.iter(1000)) {
        match *state.lock().unwrap() {
            ServerState::Stopped => break,
            _ => {}
        }
    }
}
