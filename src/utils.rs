use std::fs::File;
use std::io::BufReader;
use std::path::PathBuf;
use std::process::Command;
use std::{error::Error, fs};
use zip_extract;

pub fn parse_http_header_content_encoding(header_file: &str) -> Option<String> {
    let header_data = fs::read_to_string(&header_file);
    let header_data = match header_data {
        Ok(v) => v,
        Err(_) => {
            return None;
        }
    };
    for line in header_data.lines() {
        let line = line.trim().to_lowercase();
        if line.starts_with("content-encoding") {
            match line.find(":") {
                None => {
                    continue;
                }
                Some(pos) => {
                    let encoding = line[pos + 1..].trim();
                    return Some(encoding.to_string());
                }
            }
        }
    }
    return None;
}

pub fn parse_http_header_content_disposition(header_file: &str) -> Option<String> {
    let header_data = fs::read_to_string(&header_file);
    let header_data = match header_data {
        Ok(v) => v,
        Err(_) => {
            return None;
        }
    };

    for line in header_data.lines() {
        let line = line.trim().to_lowercase();
        if !line.starts_with("content-disposition") {
            continue;
        };
        let line = match line.find(":") {
            Some(pos) => line[pos + 1..].to_string(),
            None => {
                continue;
            }
        };
        let names = line
            .split(";")
            .map(|x| x.trim())
            .filter_map(|x| match x.find("=") {
                None => None,
                Some(pos) => Some((x[..pos].trim(), x[pos + 1..].trim())),
            })
            .filter_map(|x| match x.0 {
                "filename" => Some((x.1, 4)),
                "filename*" => Some((x.1, 1)),
                _ => None,
            });
        let mut names: Vec<(&str, i32)> = names.collect();
        names.sort_by(|a, b| a.1.cmp(&b.1));
        let name = match names.first() {
            None => {
                return None;
            }
            Some(v) => v.0,
        };
        let name = match name.find("''") {
            None => name.to_string(),
            Some(pos) => name[pos + 2..].to_string(),
        };
        return Some(name);
    }
    None
}

pub fn extract_zip(
    archive_file: &str,
    output_dir: &str,
    strip_toplevel: bool,
) -> Result<(), Box<dyn Error>> {
    let f_in = File::open(&archive_file)?;
    let reader = BufReader::new(f_in);
    let output_path = PathBuf::from(output_dir);
    zip_extract::extract(reader, &output_path, strip_toplevel)?;
    Ok(())
}

pub fn extract_tgz(
    archive_file: &str,
    output_dir: &str,
    strip_toplevel: bool,
) -> Result<(), Box<dyn Error>> {
    let strip_arg = format!("--strip-components={}", strip_toplevel as u32);
    let tar_args = vec![
        "--no-same-owner",
        "-xz",
        strip_arg.as_str(),
        "-C",
        output_dir,
        "-f",
        &archive_file,
    ];
    let prog_name = String::from("tar");
    let prog_text = format!("{} {}", prog_name, tar_args.join(" "));
    let status = Command::new(prog_name).args(tar_args).status();
    match status {
        Ok(status) => {
            if !status.success() {
                return Err(format!("exec command {} failed", prog_text).into());
            }
            Ok(())
        }
        Err(e) => return Err(e.into()),
    }
}
