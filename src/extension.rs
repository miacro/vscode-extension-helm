use anyhow::{Context, Result};
use log::{debug, error, info};
use reqwest::{self, Version};
use serde_json::from_str as json_from_str;
use serde_json::json;
use serde_json::value as json_value;
use shellexpand;
use std::error::Error;
use std::fs::{self};
use std::path::Path;

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
    pub fn download(
        &self,
        download_dir: &String,
        cached: Option<bool>,
    ) -> Result<bool, Box<dyn Error>> {
        let cached = match cached {
            Some(val) => val,
            None => true,
        };
        let ext_name = self.get_extension_name();
        let (version, platform) = self.query_version()?;
        let version = match version {
            Some(ver) => ver,
            None => {
                return Err(format!("query version for {} failed", &ext_name).into());
            }
        };
        let output_file = format!("{}/{}.vsix", download_dir, &ext_name);
        download_extension(
            &self.publisher,
            &self.package,
            &version,
            platform.as_ref().map(|x| x.as_str()),
            &output_file,
            cached,
        );
        Ok(true)
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
) {
    let ext_name = get_extension_name(publisher, package, Some(version), platform);
    let download_url = DOWNLOAD_URL.replacen("{}", publisher, 1);
    let download_url = download_url.replacen("{}", package, 1);
    let mut download_url = download_url.replacen("{}", version, 1);
    if let Some(val) = platform {
        download_url = format!("{}?targetPlatform={}", download_url, val);
    }
    //info!("Downloading {}:\nURL: {}", &ext_name, &download_url);
    println!("Downloading {}:\nURL: {}", &ext_name, &download_url);
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
    result
}
