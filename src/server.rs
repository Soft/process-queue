use std;
use std::error::Error;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::mpsc::{channel, Sender};
use std::thread;
use dbus::tree::Factory;
use dbus::{Connection, BusType, NameFlag};

use common::DBUS_NAME;
use templates::{Template, TemplateError};

#[derive(Debug)]
struct Args(Vec<String>);

pub struct Server {
    command: String,
    dir: PathBuf,
    template: Template,
    retries: usize
}

#[derive(Debug)]
pub enum ExecError {
    TemplateError(TemplateError),
    IoError(std::io::Error),
    RetryError
}

impl Server {
    pub fn new<C, T, P>(command: C, dir: P, template: T, retries: usize) -> Self
        where C: Into<String>, P: AsRef<Path>, T: Into<Template> {
        Server {
            command: command.into(),
            dir: dir.as_ref().to_owned(),
            template: template.into(),
            retries: retries
        }
    }

    pub fn run(&self) {
        info!("Starting pqueue server");
        let (sender, receiver) = channel::<Args>();
        thread::spawn(move || {
            setup_dbus_server(sender);
        });
        while let Ok(Args(args)) = receiver.recv() {
            match self.exec(&args) {
                Ok(_) =>
                    info!("succesfully executed \"{}\"", &self.command),
                Err(ExecError::TemplateError(TemplateError::ArgumentCountMismatch)) =>
                    error!("Failed to execute \"{}\": arguments do not fit the template", &self.command),
                Err(ExecError::IoError(err)) =>
                    error!("Failed to execute \"{}\": {}", &self.command, err.description()),
                Err(ExecError::RetryError) =>
                    error!("Failed to execute \"{}\": retry count exceeded", &self.command)
            }
        }
    }

    pub fn exec<S>(&self, args: &[S]) -> Result<(), ExecError>
        where S: AsRef<str> {
        let args = self.template.fill(args).map_err(ExecError::TemplateError)?;
        for _ in 0..self.retries + 1 {
            info!("executing \"{}\" with {:?}", &self.command, args);
            let mut child = Command::new(&self.command)
                .current_dir(&self.dir)
                .stdin(Stdio::null())
                .args(&args)
                .spawn()
                .map_err(ExecError::IoError)?;
            if child.wait().map_err(ExecError::IoError)?.success() {
                return Ok(());
            } else {
                error!("non-zero status returned from {}", &self.command);
            }
        }
        Err(ExecError::RetryError)
    }
}

fn setup_dbus_server(sender: Sender<Args>) {
    let conn = Connection::get_private(BusType::Session)
        .expect("Failed to connect DBUS");
    conn.register_name(DBUS_NAME, NameFlag::ReplaceExisting as u32).unwrap();
    let fact = Factory::new_fn::<()>();
    let tree = fact.tree(()).add(
        fact.object_path("/", ()).introspectable().add(
            fact.interface(DBUS_NAME, ()).add_m(
                fact.method("add", (), move |m| {
                    // TODO: remove unwrap
                    let args: Vec<String> = m.msg.get1().unwrap();
                    let reply = m.msg.method_return();
                    sender.send(Args(args)).unwrap();
                    Ok(vec!(reply))
                }).inarg::<Vec<String>, _>("args")
            )
        )
    );
    tree.set_registered(&conn, true).unwrap();
    for _ in tree.run(&conn, conn.iter(1000)) {}
}
