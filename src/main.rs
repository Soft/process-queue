#[macro_use]
extern crate clap;

extern crate dbus;
extern crate regex;
extern crate daemonize;
extern crate users;
extern crate xdg;

#[macro_use]
extern crate slog;
extern crate slog_stream;
extern crate slog_json;
extern crate slog_term;

use std::env;
use std::fs::OpenOptions;
use std::fs;
use std::io;
use std::marker::{Send, Sync};
use std::process::exit;

use slog::Logger;
use slog::Drain;
use slog::DrainExt;

mod cli;
mod client;
mod common;
mod server;
mod templates;

use cli::setup_command_line;
use server::Server;
use common::daemonize;

fn main() {
    let matches = setup_command_line().get_matches();

    let drain = matches.subcommand_matches("server")
        .and_then(|m| m.value_of("log"))
        .map(|path| {
            let file = OpenOptions::new()
                .create(true)
                .append(true)
                .open(&path)
                .expect("failed to open log file");
            let stream = Box::new(slog_stream::stream(file, slog_json::default()))
                as Box<Drain<Error=io::Error> + Send + Sync>;
            stream.fuse()
        })
        .unwrap_or(slog_term::streamer()
                   .compact()
                   .build()
                   .fuse());
    let log = Logger::root(drain, None);

    match matches.subcommand_name() {
        Some("server") => {
            let matches = matches.subcommand_matches("server").unwrap();
            let name = matches.value_of("name").unwrap();
            let command = matches.value_of("command").unwrap();
            let template: Vec<&str> = matches.values_of("template")
                .map(|v| v.collect())
                .unwrap_or(vec![]);

            let dir = match matches.value_of("dir") {
                Some(dir) => fs::canonicalize(dir)
                    .expect("failed to get the canonical representation for path"),
                None => env::current_dir().unwrap()
            };

            if !dir.is_dir() {
                error!(log, "path is not a directory";
                       "path" => format!("{}", dir.display()));
                exit(1);
            }

            let pid_file = if matches.is_present("daemon") {
               Some(daemonize(name).expect("failed to create a daemon"))
            } else {
                None
            };

            let retries = value_t_or_exit!(matches, "retries", usize);
            let server = Server::new(name,
                                     command,
                                     dir,
                                     &template as &[&str],
                                     retries,
                                     &log);
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
            client::send(name, &args, log);
        },
        Some("stop") => {
            let matches = matches.subcommand_matches("stop").unwrap();
            let name = matches.value_of("name").unwrap();
            client::stop(name, log);
        },
        Some("has") => {
            let matches = matches.subcommand_matches("has").unwrap();
            let name = matches.value_of("name").unwrap();
            client::has_server(name, log);
        },
        _ => {}
    }
}

