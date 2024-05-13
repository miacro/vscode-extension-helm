use clap::arg;
use clap::builder::TypedValueParser;
use clap::Command;
use clap::CommandFactory;
use clap::FromArgMatches;
use clap::Parser;
use env_logger;
use log;
use std::env;
use std::error::Error;
use std::path::PathBuf;

const HELP_EXTENSIONS: &str = "list of extensions to be downloaded, each is one of the following:
    1. in the format: '<publisher>.<package>[@version]';
    2. the vscode extensions.json;
    3. the output of `code --list-extensions --show-versions`";
const HELP_ALL: &str = "
Example:
1. To download all extensions for a specific version of vscode(e.g., in ./vscode_vxx), run:
    code --list-extensions | xargs -I FN ./vscode_vxx/bin/code --extensions-dir ./extensions --install-extension FN

    {} --extensions ./extensions/extensions.json
";

#[derive(Parser, Debug)]
#[command(version, about = "Download the vscode vsix extensions", after_help = &HELP_ALL)]
struct Args {
    #[arg(long, help = HELP_EXTENSIONS)]
    extensions: String,
    #[arg(long, help = "the download dir, default: ./vscode-vsix")]
    download_dir: String,
    #[arg(long, help = "use file cache or not, default: True")]
    cached: bool,
    #[arg(long, help = "show more debug messages")]
    verbose: bool,
}

fn main() {
    let args: Vec<String> = env::args().collect();
    let prog_name: String = String::from(&args[0]);
    let help_all = str::replace(HELP_ALL, "{}", &prog_name);
    let command = <Args as CommandFactory>::command().after_help(help_all);
    let mut matches = command.get_matches();
    let res = <Args as FromArgMatches>::from_arg_matches_mut(&mut matches).map_err(|err| err);
    match res {
        Ok(args) => {
            println!(
                "path: {}, download_dir: {}",
                args.extensions, args.download_dir
            );
            args;
        }
        Err(e) => {
            e.exit();
        }
    }
}
