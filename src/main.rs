use env_logger;
use log;
use std::env;
use std::error::Error;
use std::path::PathBuf;

mod cli;
mod extension;

fn main() {
    let args = cli::load_args();
    dbg!(&args);
    let extensions = extension::list_extensions(&args.extensions);
    dbg!(extensions);
    return;
}
