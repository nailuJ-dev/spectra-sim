#![cfg_attr(not(test), deny(clippy::unwrap_used, clippy::expect_used))]

#[path = "../cli/mod.rs"]
mod cli;

fn main() {
    std::process::exit(cli::run());
}
