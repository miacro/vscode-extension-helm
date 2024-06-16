use clap::arg;
use clap::builder::{ArgAction, BoolishValueParser};
use clap::{Args, CommandFactory, FromArgMatches, Parser, Subcommand};
use std::env;

const HELP_EXT_ARG: &str = "list of extensions to be downloaded, each is one of the following:
    1. in the format: '<publisher>.<package>[@version][=platform]';
    2. the vscode extensions.json;
    3. the output of `code --list-extensions --show-versions`";
const HELP_EXT_ALL: &str = "
Example:
1. To download all extensions for a specific version of vscode(e.g., in ./vscode_vxx), run:
    code --list-extensions | xargs -I FN ./vscode_vxx/bin/code --extensions-dir ./extensions --install-extension FN

    {} extension --extensions ./extensions/extensions.json
";

#[derive(Args, Debug)]
#[command(about = "Download the vscode vsix extensions", after_help = &HELP_EXT_ALL)]
pub struct ExtensionArgs {
    #[arg(
        long,
        required = true,
        num_args = 1..,
        action = ArgAction::Append,
        help = HELP_EXT_ARG,
    )]
    pub extensions: Vec<String>,
    #[arg(
        long,
        default_value = "./vscode-vsix",
        help = "the download dir, default: ./vscode-vsix"
    )]
    pub download_dir: String,
    #[arg(
        long,
        value_parser = BoolishValueParser::new(),
        default_value = "true", 
        help = "use file cache or not, default: True",
    )]
    pub cached: Option<bool>,
}

#[derive(Args, Debug)]
#[command(about = "Download the vscode server")]
pub struct ServerArgs {
    #[arg(
        long,
        value_parser = ["linux", "win32", "darwin", "alpine"],
    )]
    pub platform: Option<String>,
    #[arg(
        long,
        value_parser = ["x64", "arm64", "armhf"],
    )]
    pub arch: Option<String>,
    #[arg(long, help = "the commit id")]
    pub commit: Option<String>,
    #[arg(long, help = "the output dir", default_value = "./")]
    pub output_dir: Option<String>,
}

#[derive(Subcommand, Debug)]
pub enum PortalSubcommand {
    #[command()]
    Server(ServerArgs),
    #[command()]
    Extension(ExtensionArgs),
}

#[derive(Parser, Debug)]
#[command(
    version,
    about = "Download the vscode vsix extensions and the vscode server"
)]
pub struct PortalArgs {
    #[arg(long, default_value = "false", help = "show more debug messages")]
    pub verbose: bool,
    #[command(subcommand)]
    pub command: PortalSubcommand,
}

pub fn load_args() -> PortalArgs {
    let args: Vec<String> = env::args().collect();
    let prog_name: String = String::from(&args[0]);
    let help_ext_all = str::replace(HELP_EXT_ALL, "{}", &prog_name);
    let command = <PortalArgs as CommandFactory>::command();
    let command = command.mut_subcommand("extension", |x| x.after_help(&help_ext_all));
    let mut matches = command.get_matches();
    let res = <PortalArgs as FromArgMatches>::from_arg_matches_mut(&mut matches);
    match res {
        Ok(args) => args,
        Err(e) => {
            e.exit();
        }
    }
}
