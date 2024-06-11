use env_logger;
use log::{self, error, info, warn};
use std::{env, vec};

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
    let mut failed: Vec<String> = vec![];
    for extension in &extensions {
        let result = extension.download(&args.download_dir, args.cached);
        let success = match result {
            Ok(success) => success,
            Err(e) => {
                error!("{}", e);
                false
            }
        };
        let ext_name = extension.get_extension_name();
        if !success {
            warn!("download extension {} failed", &ext_name);
            failed.push(ext_name);
        }
    }
    if failed.len() > 0 {
        error!("Download some failed:\n{}", failed.join(" "));
    } else {
        info!("Download all succeed");
    }
    return;
}
