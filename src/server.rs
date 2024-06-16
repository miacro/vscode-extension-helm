use log::debug;
use serde_json::from_str as json_from_str;
use serde_json::value as json_value;
use std::env;
use std::error::Error;
use std::process::Command;

pub fn get_platform_info(platform: &Option<String>, arch: &Option<String>) -> (String, String) {
    let valid_map_p = vec![
        ("linux", "linux"),
        ("windows", "win32"),
        ("macos", "darwin"),
        ("alpine", "alpine"),
    ];
    let valid_map_a = vec![("x86_64", "x64"), ("aarch64", "arm64"), ("arm", "armhf")];
    let platform = match platform {
        Some(v) => v,
        None => {
            let mut platform = env::consts::OS;
            for (k, v) in valid_map_p.iter() {
                if *k == platform {
                    platform = *v;
                    break;
                }
            }
            platform
        }
    };
    let arch = match arch {
        Some(v) => v,
        None => {
            let mut arch = env::consts::ARCH;
            for (k, v) in valid_map_a.iter() {
                if *k == arch {
                    arch = *v;
                    break;
                }
            }
            arch
        }
    };
    (platform.into(), arch.into())
}

pub fn get_latest_release(platform: &String, arch: &String) -> Result<String, Box<dyn Error>> {
    let url = format!(
        "https://update.code.visualstudio.com/api/commits/stable/{}-{}",
        platform, arch
    );
    let curl_args = vec!["-fsSL", &url];
    let prog_name = String::from("curl");
    let mut command = Command::new(&prog_name);
    command.args(&curl_args);
    let prog_text = format!("{} {}", &prog_name, (&curl_args).join(" "));
    debug!("exec command: {}", &prog_text);
    let result = command.output()?;
    if !result.status.success() {
        let error = String::from_utf8(result.stderr).map_or("".into(), |x| x);
        return Err(format!("exec command {} failed: {}", prog_text, { error }).into());
    }
    let data = String::from_utf8(result.stdout)?;
    let data: json_value::Value = json_from_str(&data)?;
    let commit = data
        .as_array()
        .map_or(None, |x| x.first())
        .map_or(None, |x| x.as_str());
    match commit {
        None => Err(format!("query vscode server commit id failed, url {}", &url).into()),
        Some(v) => Ok(v.into()),
    }
}

pub fn download_release_file(
    commit: &String,
    prefix: &String,
    arch: &String,
    archive_file: &String,
) -> Result<(), Box<dyn Error>> {
    Ok(())
}

pub fn prepare_release_dir(
    commit: &String,
    archive_file: &String,
    output_dir: &String,
) -> Result<(), Box<dyn Error>> {
    Ok(())
}
