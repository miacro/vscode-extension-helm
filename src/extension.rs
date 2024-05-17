use std::path::Path;
use std::collections::HashMap;

#[derive(Debug)]
pub struct Extension {
    publisher: String,
    package: String,
    version: String,
    platform: String,
}

pub fn list_extensions(extensions: &Vec<String>) -> Vec<Extension> {
    fn strip_suffix(line: &String, mark: &String) {

    }
    fn parse_ext_line(ext_line: &String) -> Extension{
        println!("test");
    }
    fn parse_ext_dict(ext_dict: &HashMap<String, String>) -> Extension{

    }
    for extension in extensions {
        if Path::new(&extension).exists() {
        } else if (true) {
        }
    }
    let res = vec![Extension {
        publisher: String::from("a"),
        package: String::from(""),
        version: String::from(""),
        platform: String::from(""),
    }];
    dbg!(&res);
    res
}
