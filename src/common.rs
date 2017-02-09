use std;
use std::fs::Permissions;
use std::os::unix::fs::{MetadataExt, PermissionsExt};
use std::path::PathBuf;

use regex::Regex; 
use daemonize::{Daemonize, DaemonizeError};
use xdg::BaseDirectories;
use users::{get_current_uid, get_current_gid};

use dbus;
use dbus::{Connection, Message};

pub const DBUS_INTERFACE: &'static str = "org.ProcessQueue";
pub const DBUS_PATH: &'static str = "/";
pub const DBUS_METHOD_ADD: &'static str = "Add";
pub const DBUS_METHOD_STOP: &'static str = "Stop";

#[derive(Debug)]
pub struct NameError;

pub fn dbus_get_name(name: &str) -> Result<String, NameError> {
    let regex = Regex::new(r"^[a-zA-Z]+$").unwrap();
    if regex.is_match(name) {
        Ok(format!("org.ProcessQueue.{}", name))
    } else {
        Err(NameError)
    }
}

#[test]
fn test_dbus_get_name() {
    assert!(get_dbus_name("_/+").is_err());
}

#[derive(Debug)]
pub enum DaemonError {
    PidFileError(PidFileError),
    DaemonError(DaemonizeError)
}

pub fn daemonize(name: &str) -> Result<PathBuf, DaemonError> {
    let path = get_pid_file_path(name)
        .map_err(DaemonError::PidFileError)?;
    Daemonize::new()
        .pid_file(&path)
        .start()
        .map_err(DaemonError::DaemonError)
        .and(Ok(path))
}

#[derive(Debug)]
pub enum PidFileError {
    IOError(std::io::Error),
    OwnershipError,
    PermissionError
}

impl From<std::io::Error> for PidFileError {
    fn from(err: std::io::Error) -> Self {
        PidFileError::IOError(err)
    }
}

fn get_pid_file_path(name: &str) -> Result<PathBuf, PidFileError> {
    if let Ok(dirs) = BaseDirectories::new() {
        if dirs.has_runtime_directory() {
            if let Ok(path) =
                dirs.place_runtime_file(format!("pqueue-{}.pid", name)) {
                return Ok(path);
            }
        }
    }

    let uid = get_current_uid();
    let gid = get_current_gid();

    let temp = std::env::temp_dir();
    let dir = PathBuf::from(temp.join(format!("pqueue-{}", uid)));
    let perms = Permissions::from_mode(0o700);
    if !dir.is_dir() {
        std::fs::create_dir(&dir)?;
        std::fs::set_permissions(&dir, perms.clone())?;
    }

    let meta = std::fs::metadata(&dir)?;
    if meta.uid() != uid || meta.gid() != gid {
        return Err(PidFileError::OwnershipError);
    }
    if meta.permissions() != perms {
        return Err(PidFileError::PermissionError);
    }

    Ok(dir.join(format!("{}.pid", name)))
}

const DBUS_MANAGER_NAME: &'static str = "org.freedesktop.DBus";
const DBUS_MANAGER_PATH: &'static str = "/org/freedesktop/DBus";
const DBUS_MANAGER_INTERFACE: &'static str = "org.freedesktop.DBus";

fn dbus_list_names(connection: &Connection) -> Result<Vec<String>, dbus::Error> {
    let message = Message::new_method_call(DBUS_MANAGER_NAME,
                                           DBUS_MANAGER_PATH,
                                           DBUS_MANAGER_INTERFACE,
                                           "ListNames")
        .unwrap();
    let reply = connection.send_with_reply_and_block(message, 1000)?;
    Ok(reply.get1().expect("failed to get list of DBus names"))
}

pub fn dbus_name_exists(connection: &Connection, name: &str) -> Result<bool, dbus::Error> {
    dbus_list_names(connection).map(|v| v.into_iter().any(|n| n == name))
}


