#[macro_use]
extern crate clap;

extern crate dbus;
extern crate regex;
extern crate daemonize;
extern crate users;
extern crate xdg;

#[macro_use]
extern crate log;
extern crate env_logger;

use std::process::exit;

mod cli;
mod client;
mod common;
mod server;
mod templates;

use cli::setup_command_line;
use server::Server;
use common::daemonize;

fn main() {
    env_logger::init().unwrap();
    let matches = setup_command_line().get_matches();

    match matches.subcommand_name() {
        Some("server") => {
            let matches = matches.subcommand_matches("server").unwrap();
            let name = matches.value_of("name").unwrap();
            let command = matches.value_of("command").unwrap();
            let template: Vec<&str> = matches.values_of("template")
                .map(|v| v.collect())
                .unwrap_or(vec![]);

            let dir = match matches.value_of("dir") {
                Some(dir) => dir.into(),
                None => std::env::current_dir().unwrap()
            };

            if !dir.is_dir() {
                error!("{} is not a directory", dir.display());
                exit(1);
            }

            let pid_file = if matches.is_present("daemon") {
               Some(daemonize(name).expect("failed to create a daemon"))
            } else {
                None
            };

            let retries = value_t_or_exit!(matches, "retries", usize);
            let server = Server::new(name, command, dir, &template as &[&str], retries);
            server.run();

            if let Some(pid_file) = pid_file {
                std::fs::remove_file(pid_file)
                    .expect("failed to remove pid-file");
            }
        },
        Some("send") => {
            let matches = matches.subcommand_matches("send").unwrap();
            let name = matches.value_of("name").unwrap();
            let args: Vec<&str> = matches.values_of("args")
                .map(|v| v.collect())
                .unwrap_or(vec![]);
            client::send(name, &args);
        },
        Some("stop") => {
            let matches = matches.subcommand_matches("stop").unwrap();
            let name = matches.value_of("name").unwrap();
            client::stop(name);
        }
        _ => {}
    }
}

