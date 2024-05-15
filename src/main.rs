use env_logger;
use log;
use std::env;
use std::error::Error;
use std::path::PathBuf;

mod cli;

fn main() {
    let args = cli::load_args();
    dbg!(args);
}
