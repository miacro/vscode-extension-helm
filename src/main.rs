use env_logger;
use log;
use std::env;
use std::error::Error;
use std::path::PathBuf;

mod cli;
mod extension;

fn main() {
    env_logger::init();
    let args = cli::load_args();
    dbg!(&args);
    let extensions = extension::list_extensions(&args.extensions);
    for extension in &extensions {
        extension.download(&args.download_dir, args.cached);
    }
    return;
}
