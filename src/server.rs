use std;
use std::error::Error;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::mpsc::{channel, Sender};
use std::sync::{Arc, Mutex};
use std::thread;

use dbus::tree::Factory;
use dbus::{Connection, BusType, NameFlag};
use slog::Logger;

use common::{dbus_get_name, dbus_name_exists,
             DBUS_INTERFACE, DBUS_PATH, DBUS_METHOD_ADD, DBUS_METHOD_STOP};
use templates::{Template, TemplateError};

#[derive(Debug)]
struct Args(Vec<String>);

pub struct Server {
    name: String,
    command: String,
    dir: PathBuf,
    template: Template,
    retries: usize,
    log: Logger
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
                           retries: usize,
                           log: &Logger) -> Self
        where N: Into<String>,
              C: Into<String>,
              P: AsRef<Path>,
              T: Into<Template> {
        Server {
            name: name.into(),
            command: command.into(),
            dir: dir.as_ref().to_owned(),
            template: template.into(),
            retries: retries,
            log: log.new(None)
        }
    }

    pub fn run(&self) {
        info!(self.log, "starting pqueue server"; "name" => self.name);
        let (sender, receiver) = channel::<Args>();
        let name = self.name.clone();
        let state = Arc::new(Mutex::new(ServerState::Running));
        let state_clone = state.clone();
        let log_clone = self.log.clone();

        thread::spawn(move || {
            setup_dbus_server(&name, state_clone, sender, log_clone);
        });

        while let Ok(Args(args)) = receiver.recv() {
            match self.exec(&args) {
                Ok(_) =>
                    info!(self.log, "program finished successfully"; "name" => self.command),
                Err(ExecError::TemplateError(TemplateError::ArgumentCountMismatch)) =>
                    error!(self.log, "execution failure";
                           "name" => self.command,
                           "reason" => "arguments do not fit the template"),
                Err(ExecError::IOError(err)) =>
                    error!(self.log, "execution failure";
                           "name" => self.command,
                           "reason" => err.description()),
                Err(ExecError::RetryError) =>
                    error!(self.log, "execution failure";
                           "name" => self.command,
                           "reason" => "retry count exceeded")
            }

            if let ServerState::Stopped = *state.lock().unwrap() {
                info!(self.log, "stopping"; "name" => self.name);
                break;
            }
        }
    }

    pub fn exec<S>(&self, args: &[S]) -> Result<(), ExecError>
        where S: AsRef<str> {
        let args = self.template.fill(args)?;
        for _ in 0..self.retries + 1 {
            info!(self.log, "executing a program";
                  "name" => self.command,
                  "arguments" => format!("{:?}", args));
            let mut child = Command::new(&self.command)
                .current_dir(&self.dir)
                .stdin(Stdio::null())
                .args(&args)
                .spawn()?;
            let status = child.wait()?;
            if status.success() {
                return Ok(());
            } else {
                error!(self.log, "non-zero status returned from {}", &self.command);
                error!(self.log, "program finished with a non-zero status";
                       "name" => self.command,
                       "status" => status.code()
                       .map(|c| c.to_string())
                       .unwrap_or("<missing>".to_owned()))
            }
        }
        Err(ExecError::RetryError)
    }
}

fn setup_dbus_server(name: &str,
                     state: Arc<Mutex<ServerState>>,
                     sender: Sender<Args>,
                     log: Logger) {
    let full_name = dbus_get_name(name)
                .expect("invalid server name");

    let conn = Connection::get_private(BusType::Session)
        .expect("failed to connect DBus");

    if dbus_name_exists(&conn, &full_name)
        .expect("failed to check if the name exists") {
            error!(log, "server name is already in use"; "name" => name);
            return;
    }

    conn.register_name(&full_name, NameFlag::ReplaceExisting as u32)
        .unwrap();

    let fact = Factory::new_fn::<()>();

    let state_clone = state.clone();
    let log_add = log.clone();
    let log_stop = log.clone();

    let tree = fact.tree(()).add(
        fact.object_path(DBUS_PATH, ()).introspectable().add(
            fact.interface(DBUS_INTERFACE, ()).add_m(
                fact.method(DBUS_METHOD_ADD, (), move |m| {
                    // TODO: remove unwrap
                    let args: Vec<String> = m.msg.get1().unwrap();
                    info!(log_add, "new task received"; "arguments" => format!("{:?}", args));
                    let reply = m.msg.method_return();
                    sender.send(Args(args)).unwrap();
                    Ok(vec!(reply))
                }).inarg::<Vec<String>, _>("args")
            ).add_m(
                fact.method(DBUS_METHOD_STOP, (), move |m| {
                    info!(log_stop, "received a stop request");
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
        if let ServerState::Stopped = *state.lock().unwrap() {
            break;
        }
    }
}
