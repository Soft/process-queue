#[macro_use]
extern crate clap;

extern crate dbus;
extern crate regex;

#[macro_use]
extern crate log;
extern crate env_logger;

mod cli;
mod client;
mod common;
mod server;
mod templates;

use cli::setup_command_line;
use server::Server;

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
            let retries = value_t_or_exit!(matches, "retries", usize);
            let server = Server::new(name, command, dir, &template as &[&str], retries);
            server.run();
        },
        Some("send") => {
            let matches = matches.subcommand_matches("send").unwrap();
            let name = matches.value_of("name").unwrap();
            let args: Vec<&str> = matches.values_of("args")
                .map(|v| v.collect())
                .unwrap_or(vec![]);
            client::run(name, &args);
        },
        _ => {}
    }
}

