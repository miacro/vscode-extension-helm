use anyhow::{Context, Result};
use flate2::read::GzDecoder;
use log::{debug, info};
use reqwest::{self};
use serde_json::from_str as json_from_str;
use serde_json::json;
use serde_json::value as json_value;
use shellexpand;
use std::error::Error;
use std::fs::read_to_string;
use std::fs::{self, File};
use std::io::Read;
use std::io::Write;
use std::path::Path;
use std::path::MAIN_SEPARATOR;
use std::process::Command;

#[derive(Debug)]
pub struct Extension {
    publisher: String,
    package: String,
    version: Option<String>,
    platform: Option<String>,
}
static QUERY_URL: &str = "https://marketplace.visualstudio.com/_apis/public/gallery/extensionQuery";
static DOWNLOAD_URL: &str = "https://marketplace.visualstudio.com/_apis/public/gallery/publishers/{}/vsextensions/{}/{}/vspackage";

impl Extension {
    pub fn get_extension_name(&self) -> String {
        get_extension_name(
            self.publisher.as_ref(),
            self.package.as_ref(),
            self.version.as_ref().map(|x| x.as_str()),
            self.platform.as_ref().map(|x| x.as_str()),
        )
    }

    pub fn check_platform(&self) -> Result<(), Box<dyn Error>> {
        let valid_platforms = vec![
            ("win32-x64", "Windows x64"),
            ("win32-ia32", "Windows ia32"),
            ("win32-arm64", "Windows ARM"),
            ("linux-x64", "Linux x64"),
            ("linux-arm64", "Linux ARM64"),
            ("linux-armhf", "Linux ARM32"),
            ("darwin-x64", "macOS Intel"),
            ("darwin-arm64", "macOS Apple Silicon"),
            ("alpine-x64", "Alpine Linux 64 bit"),
            ("web", "Web"),
            ("alpine-arm64", "Alpine Linux ARM64"),
        ];
        let passed = match &self.platform {
            None => true,
            Some(platform) => valid_platforms.iter().any(|x| platform == x.0),
        };
        if passed {
            Ok(())
        } else {
            let choices = valid_platforms
                .iter()
                .map(|x| x.0)
                .collect::<Vec<&str>>()
                .join(", ");
            let message = format!(
                "invalid platform {}, choices in ({})",
                self.platform.as_ref().unwrap(),
                choices
            );
            Err(message.into())
        }
    }

    pub fn download(
        &self,
        download_dir: &String,
        cached: Option<bool>,
    ) -> Result<bool, Box<dyn Error>> {
        let cached = match cached {
            Some(val) => val,
            None => true,
        };
        self.check_platform()?;
        let ext_name = self.get_extension_name();
        let (version, platform) = match &self.version {
            Some(v) => (v.clone(), self.platform.clone()),
            None => {
                let (v, p) = self.query_version()?;
                let v = match v {
                    Some(v) => v,
                    None => {
                        return Err(format!("query version for {} failed", &ext_name).into());
                    }
                };
                (v, p.clone())
            }
        };
        let output_file = format!("{}{}{}.vsix", download_dir, MAIN_SEPARATOR, &ext_name);
        if cached && Path::new(&output_file).exists() {
            info!("{output_file} already exists, skip downloading");
            return Ok(false);
        }
        fs::create_dir_all(download_dir)?;
        let result = download_extension(
            &self.publisher,
            &self.package,
            &version,
            platform.as_deref(),
            &output_file,
            cached,
        );
        match result {
            Ok(()) => Ok(true),
            Err(e) => Err(e),
        }
    }

    pub fn query_version(&self) -> Result<(Option<String>, Option<String>), Box<dyn Error>> {
        let all_data = query_extension(&self.publisher, &self.package, None)?;
        let (version, platform): (Option<String>, Option<String>) = all_data
            .get("versions")
            .map_or(None, |x| x.as_array())
            .map_or((None, None), |x| {
                for ver_data in x {
                    let version = ver_data.get("version").map_or(None, |x| x.as_str());
                    let platform = ver_data.get("targetPlatform").map_or(None, |x| x.as_str());
                    let version = match (self.version.as_ref(), version) {
                        (Some(v1), Some(v2)) => {
                            if v1 != v2 {
                                continue;
                            }
                            Some(v2.to_string())
                        }
                        (_, None) => {
                            continue;
                        }
                        (_, Some(v2)) => Some(v2.to_string()),
                    };
                    match (self.platform.as_ref(), platform) {
                        (Some(v1), Some(v2)) => {
                            if v1 != v2 {
                                continue;
                            }
                            return (version, Some(v2.to_string()));
                        }
                        (None, Some(_)) => {
                            continue;
                        }
                        _ => {
                            return (version, None);
                        }
                    }
                }
                return (None, None);
            });
        if let None = version {
            let ext_name = get_extension_name(&self.publisher, &self.package, None, None);
            let message = format!("query extension {} for version failed", ext_name);
            Err(message.into())
        } else {
            Ok((version, platform))
        }
    }
}

pub fn get_extension_name(
    publisher: &str,
    package: &str,
    version: Option<&str>,
    platform: Option<&str>,
) -> String {
    let ext_name = format!("{}.{}", publisher, package);
    let ext_name = match version {
        Some(val) => format!("{}@{}", ext_name, val),
        None => ext_name,
    };
    let ext_name = match platform {
        Some(val) => format!("{}={}", ext_name, val),
        None => ext_name,
    };
    ext_name
}

pub fn query_extension(
    publisher: &str,
    package: &str,
    flags: Option<usize>,
) -> Result<json_value::Value, Box<dyn Error>> {
    let flags = match flags {
        Some(flags) => flags,
        None => 0x55,
    };
    let ext_name = get_extension_name(publisher, package, None, None);
    let filters = json!([{
        "criteria": [{"filterType": 7, "value": ext_name}],
        "pageNumber": 1,
        "pageSize": 10,
    }]);
    let payload = json!({
        "flags": flags,
        "filters":filters,
    });
    let client = reqwest::blocking::Client::new();
    let mut request = client.post(QUERY_URL);
    let headers = vec![
        ("Content-Type", "application/json"),
        ("Accept", "application/json;api-version=3.0-preview.1"),
        ("User-Agent", "Offline VSIX/1.0"),
    ];
    for (key, val) in headers {
        request = request.header(key, val);
    }
    request = request.json(&payload);
    let response = request.send()?;
    let response = response.error_for_status();
    let data = response
        .map_or_else(|x| Err(x), |x| x.json().map(|x: json_value::Value| x))
        .context(format!("query extension {} info failed", &ext_name))?;
    let data = data
        .get("results")
        .map_or(None, |x| x.get(0))
        .map_or(None, |x| x.get("extensions"))
        .map_or(None, |x| x.get(0));
    match data {
        Some(val) => Ok(val.clone()),
        None => Err("no data found in query response".into()),
    }
}

pub fn download_extension(
    publisher: &str,
    package: &str,
    version: &str,
    platform: Option<&str>,
    output_file: &str,
    cached: bool,
) -> Result<(), Box<dyn Error>> {
    let ext_name = get_extension_name(publisher, package, Some(version), platform);
    let download_url = DOWNLOAD_URL.replacen("{}", publisher, 1);
    let download_url = download_url.replacen("{}", package, 1);
    let mut download_url = download_url.replacen("{}", version, 1);
    if let Some(val) = platform {
        download_url = format!("{}?targetPlatform={}", download_url, val);
    }
    debug!("Downloading {}:\nURL: {}", &ext_name, &download_url);
    let head_file = format!("{}.header", output_file);
    let body_file = format!("{}.downloading", output_file);
    let mut curl_args = vec!["-fSL"];
    if cached {
        curl_args.extend(["-C", "-"]);
    }
    curl_args.extend([
        &download_url,
        "-o",
        body_file.as_str(),
        "-D",
        head_file.as_str(),
    ]);
    let prog_name = String::from("curl");
    let mut command = Command::new(&prog_name);
    command.args(curl_args);
    let status = command.status();
    match status {
        Ok(status) => {
            if !status.success() {
                let args = command.get_args().map(|x| x.to_str().map_or("", |x| x));
                let args: Vec<&str> = args.collect();
                let args = args.join(" ");
                return Err(format!("exec command {} {} failed", prog_name, args).into());
            }
        }
        Err(e) => {
            return Err(e.into());
        }
    };
    let mut encoding = String::from("");
    for line in read_to_string(&head_file)?.lines() {
        let line = line.trim().to_lowercase();
        if line.starts_with("content-encoding") {
            let data: Vec<&str> = line.split(":").collect();
            encoding = data[data.len() - 1].to_string();
        }
    }
    let mut f_i = File::open(&body_file)?;
    let mut data = vec![];
    f_i.read_to_end(&mut data)?;
    if encoding.contains("gzip") {
        let mut gz = GzDecoder::new(&data[..]);
        let mut decoded = vec![];
        gz.read_to_end(&mut decoded)?;
        data = decoded;
    }
    let mut f_o = File::create(&output_file)?;
    f_o.write_all(&data)?;
    fs::remove_file(body_file)?;
    fs::remove_file(head_file)?;
    Ok(())
}

pub fn list_extensions(extensions: &Vec<String>) -> Vec<Extension> {
    fn strip_suffix<'a>(line: &'a str, mark: &str) -> (&'a str, Option<&'a str>) {
        let pos = line.find(mark);
        match pos {
            Some(pos) => (&line[0..pos], Some(&line[pos + mark.len()..])),
            None => (line, None),
        }
    }
    fn parse_ext_line(ext_line: &str) -> Option<Extension> {
        let ext_line = ext_line.trim();
        let ext_line = match ext_line.strip_suffix(".vsix") {
            Some(v) => v,
            None => ext_line,
        };
        let (ext_prefix, platform) = strip_suffix(ext_line, "=");
        let (ext_prefix, version) = strip_suffix(ext_prefix, "@");
        let (publisher, package) = strip_suffix(ext_prefix, ".");
        let package = package?;
        Some(Extension {
            package: package.to_string(),
            publisher: publisher.to_string(),
            platform: platform.map(str::to_string),
            version: version.map(str::to_string),
        })
    }
    fn parse_ext_dict(ext_dict: &json_value::Value) -> Option<Extension> {
        let identifier = ext_dict.get("identifier")?;
        let ext_name = identifier.get("id")?.as_str()?;
        let version = match ext_dict.get("version") {
            Some(ver) => ver.as_str().map(str::to_string),
            None => None,
        };
        let mut platform = Some(ext_dict);
        for key in vec!["metadata", "targetPlatform"] {
            platform = match platform {
                Some(value) => match value.get(key) {
                    Some(value) => Some(value),
                    None => break,
                },
                None => break,
            };
        }
        let platform = match platform {
            Some(value) => match value.as_str() {
                Some("undefined") | Some("none") => None,
                value => value.map(str::to_string),
            },
            None => None,
        };
        let ext = parse_ext_line(&String::from(ext_name));
        ext.map(|x| Extension {
            platform,
            version,
            ..x
        })
    }
    let mut result: Vec<Extension> = vec![];
    for extension in extensions {
        let ext_path = shellexpand::full(extension);
        let ext_path = ext_path.map_or(extension.clone(), |x| x.to_string());
        if !Path::new(&ext_path).exists() {
            let ext = parse_ext_line(&ext_path);
            if let Some(ext) = ext {
                result.push(ext)
            }
            continue;
        }
        let content = fs::read_to_string(&ext_path)
            .expect(format!("read file {} failed", &ext_path).as_str());
        if vec!["[", "{"].iter().any(|x| content.starts_with(x)) {
            let data: json_value::Value = json_from_str(&content)
                .expect(format!("parse json failed from {}", ext_path).as_str());
            if let Some(data) = data.as_array() {
                for item in data {
                    let ext = parse_ext_dict(item);
                    if let Some(ext) = ext {
                        result.push(ext);
                    }
                }
            } else if data.is_object() {
                let ext = parse_ext_dict(&data);
                if let Some(ext) = ext {
                    result.push(ext);
                }
            } else {
                continue;
            }
        } else {
            for line in content.split("\n") {
                let ext = parse_ext_line(line);
                if let Some(ext) = ext {
                    result.push(ext);
                }
            }
        }
    }
    result.sort_by(|a, b| a.get_extension_name().cmp(&b.get_extension_name()));
    result.dedup_by_key(|x| x.get_extension_name());
    result
}
