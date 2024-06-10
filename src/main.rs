use env_logger;
use log::{self, error, warn};
use std::env;

mod cli;
mod extension;

fn main() {
    let args = cli::load_args();
    dbg!(&args);
    let _ = env::var("RUST_LOG").map_err(|_| {
        let log_level = match args.verbose {
            true => "debug",
            false => "info",
        };
        env::set_var("RUST_LOG", log_level);
    });
    env_logger::init();
    let extensions = extension::list_extensions(&args.extensions);
    for extension in &extensions {
        let result = extension.download(&args.download_dir, args.cached);
        let success = match result {
            Ok(success) => success,
            Err(e) => {
                error!("{}", e);
                false
            }
        };
        if !success {
            warn!(
                "download extension {} failed",
                extension.get_extension_name()
            )
        }
    }
    return;
}
