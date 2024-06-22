use log::debug;
use serde_json::from_str as json_from_str;
use serde_json::value as json_value;
use std::env;
use std::error::Error;
use std::fs;
use std::path::PathBuf;
use std::process::Command;

use crate::utils;

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
    let prog_text = format!("{} {}", &prog_name, (&curl_args).join(" "));
    debug!("exec command: {}", &prog_text);
    let result = Command::new(&prog_name).args(&curl_args).output()?;
    if !result.status.success() {
        let error = String::from_utf8(result.stderr).map_or("".into(), |x| x);
        return Err(format!("exec command {} failed: {}", prog_text, { error }).into());
    }
    let data = String::from_utf8(result.stdout)?;
    let data: json_value::Value = json_from_str(&data)?;
    let commit = data
        .as_array()
        .and_then(|x| x.first())
        .and_then(|x| x.as_str());
    match commit {
        None => Err(format!("query vscode server commit id failed, url {}", &url).into()),
        Some(v) => Ok(v.into()),
    }
}

pub fn download_release_file(
    commit: &String,
    prefix: &String,
    arch: &String,
    output_dir: &String,
) -> Result<String, Box<dyn Error>> {
    let archive_path = format!("vscode-{}-{}-{}", &prefix, &arch, &commit);
    let archive_path = PathBuf::from(output_dir).join(archive_path);
    let archive_path = archive_path.to_str().unwrap();
    debug!("download vscode server release file to {}", archive_path);
    fs::create_dir_all(output_dir)?;
    let url = format!(
        "https://update.code.visualstudio.com/commit:{}/{}-{}/stable",
        commit, prefix, arch
    );
    let body_file = format!("{}.downloading", &archive_path);
    let head_file = format!("{}.header", &archive_path);
    let curl_args = vec![
        "-fSL",
        "-C",
        "-",
        &url,
        "-o",
        body_file.as_str(),
        "-D",
        head_file.as_str(),
    ];
    let prog_name = String::from("curl");
    let prog_text = format!("{} {}", prog_name, curl_args.join(" "));
    debug!("exec command:\n\t{}", prog_text);
    let status = Command::new(&prog_name).args(&curl_args).status();
    match status {
        Ok(status) => {
            if !status.success() {
                return Err(format!("exec command {} failed", prog_text).into());
            }
        }
        Err(e) => {
            return Err(e.into());
        }
    };
    let archive_ext = utils::parse_http_header_content_disposition(&head_file)
        .and_then(|x| match x.rfind(".") {
            None => None,
            Some(pos) => {
                let ext = if x[..pos].ends_with(".tar") {
                    format!(".tar{}", x[pos..].to_string())
                } else {
                    x[pos..].to_string()
                };
                Some(ext)
            }
        })
        .map_or(String::from(".tar.gz"), |x| x);
    let archive_file = format!("{}{}", archive_path, archive_ext);
    debug!("archive file {}", &archive_file);
    fs::rename(body_file, &archive_file)?;
    fs::remove_file(head_file)?;
    Ok(archive_file)
}

pub fn prepare_release_dir(
    commit: &String,
    archive_file: &String,
    output_dir: &String,
) -> Result<(), Box<dyn Error>> {
    debug!("{} {} {}", commit, archive_file, output_dir);
    let output_dir = PathBuf::from(output_dir);
    let bin_dir = output_dir.join("bin");
    let commit_dir = bin_dir.join(commit);
    if commit_dir.exists() {
        fs::remove_dir_all(&commit_dir)?;
    }
    if !commit_dir.exists() {
        fs::create_dir_all(&commit_dir)?;
    }
    let commit_dir = commit_dir.to_str().unwrap();
    debug!("extract files from {} to {}", archive_file, commit_dir);
    if archive_file.ends_with(".tar.gz") {
        utils::extract_tgz(&archive_file, &commit_dir, true)?;
    } else if archive_file.ends_with(".zip") {
        utils::extract_zip(&archive_file, &commit_dir, true)?;
    } else {
        return Err(format!("unable to extract file {}", &archive_file).into());
    }
    Ok(())
}
