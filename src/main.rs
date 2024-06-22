use cli::{ExtensionArgs, PortalSubcommand, ServerArgs};
use env_logger;
use log::{self, debug, error, info, warn};
use std::{env, vec};

mod cli;
mod extension;
mod server;
mod utils;

fn main() {
    let args = cli::load_args();
    let _ = env::var("RUST_LOG").map_err(|_| {
        let log_level = match args.verbose {
            true => "debug",
            false => "info",
        };
        env::set_var("RUST_LOG", log_level);
    });
    env_logger::init();
    debug!("args: {:#?}", &args);
    match &args.command {
        PortalSubcommand::Extension(v) => {
            download_extensions(&v);
        }
        PortalSubcommand::Server(v) => {
            download_server(&v);
        }
    }
    return;
}

fn download_extensions(args: &ExtensionArgs) {
    let extensions = extension::list_extensions(&args.extensions);
    let mut failed: Vec<String> = vec![];
    for extension in &extensions {
        let result = extension.download(&args.download_dir, args.cached);
        let success = match result {
            Ok(_) => true,
            Err(e) => {
                error!("caught error: {:#?}", e);
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
        error!("download some failed:\n{}", failed.join(" "));
    } else {
        info!("download all succeed");
    }
    return;
}

fn download_server(args: &ServerArgs) {
    let (platform, arch) = server::get_platform_info(&args.platform, &args.arch);
    let mut commit = String::from("");
    let mut prefix = String::from("");
    let mut archive_file = String::from("");
    let output_dir = args.output_dir.as_ref().map_or(".".into(), |x| x.clone());
    let res = args
        .commit
        .as_ref()
        .map_or_else(
            || server::get_latest_release(&platform, &arch),
            |x| Ok(x.into()),
        )
        .and_then(|v| {
            prefix = match platform.as_str() {
                "alpine" => format!("cli-{}", &platform),
                _ => format!("server-{}", &platform),
            };
            commit = v;
            Ok(())
        })
        .and_then(|_| {
            let result = server::download_release_file(&commit, &prefix, &arch, &output_dir);
            match result {
                Err(e) => Err(e),
                Ok(file_name) => {
                    archive_file = file_name;
                    Ok(())
                }
            }
        })
        .and_then(|_| server::prepare_release_dir(&commit, &archive_file, &output_dir));
    match res {
        Ok(_) => (),
        Err(e) => {
            error!("caught error: {:#?}", e);
            ()
        }
    }
}
