extern crate clap;

use std::env;
use clap::Shell;

include!("src/cli.rs");

fn main() {
    let out_dir = env::var("OUT_DIR").unwrap();
    let mut app = setup_command_line();
    for shell in &[Shell::Bash, Shell::Fish, Shell::Zsh] {
        app.gen_completions("pqueue",
                            *shell,
                            &out_dir);
    }
}
