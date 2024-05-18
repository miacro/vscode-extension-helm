use once_cell::sync::Lazy;
use shellexpand;
use std::borrow::Borrow;
use std::collections::HashMap;
use std::env;
use std::fs::{self};
use std::path::Path;

#[derive(Debug)]
pub struct Extension {
    publisher: String,
    package: String,
    version: Option<String>,
    platform: Option<String>,
}
static HEADERS: Lazy<HashMap<&str, &str>> = Lazy::new(|| {
    HashMap::from([
        ("Content-Type", "application/json"),
        ("Accept", "application/json;api-version=3.0-preview.1"),
        ("User-Agent", "Offline VSIX/1.0"),
    ])
});
static QUERY_URL: &str = "https://marketplace.visualstudio.com/_apis/public/gallery/extensionQuery";

impl Extension {
    pub fn download(self, download_dir: &String, cached: bool) -> bool {
        true
    }
    pub fn query_info(self) -> (Option<String>, Option<String>) {
        (None, None)
    }
}

pub fn list_extensions(extensions: &Vec<String>) -> Vec<Extension> {
    fn strip_suffix<'a>(line: &'a str, mark: &str) -> (&'a str, Option<&'a str>) {
        let pos = line.find(mark);
        match pos {
            Some(pos) => (&line[0..pos], Some(&line[pos + mark.len()..])),
            None => (line, None),
        }
    }
    fn parse_ext_line(ext_line: &str) -> Extension {
        let ext_line = ext_line.trim();
        let (ext_prefix, platform) = strip_suffix(ext_line, "=");
        let (ext_prefix, version) = strip_suffix(ext_prefix, "@");
        let (publisher, package) = strip_suffix(ext_prefix, ".");
        let package = package.unwrap_or_else(|| {
            panic!("Invalid package: {}", ext_line);
        });
        Extension {
            package: package.to_string(),
            publisher: publisher.to_string(),
            platform: platform.map(str::to_string),
            version: version.map(str::to_string),
        }
    }
    fn parse_ext_dict(ext_dict: &serde_json::value::Value) -> Option<Extension> {
        let identifier = ext_dict.get("identifier")?;
        let ext_name = identifier.get("id")?.as_str().unwrap();
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
        Some(Extension {
            platform,
            version,
            ..ext
        })
    }
    let mut result: Vec<Extension> = vec![];
    for extension in extensions {
        let ext_path = shellexpand::full(extension)
            .map(|x| x.to_string())
            .expect(&extension);
        if !Path::new(&ext_path).exists() {
            let ext = parse_ext_line(&ext_path);
            result.push(ext);
            continue;
        }
        let content = fs::read_to_string(&extension).unwrap();
        if vec!["[", "{"].iter().any(|x| content.starts_with(x)) {
            let data: serde_json::value::Value = serde_json::from_str(&content).unwrap();
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
                result.push(ext);
            }
        }
    }
    result
}
