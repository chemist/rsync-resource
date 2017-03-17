extern crate rustc_serialize;

//use rustc_serialize::json;
use std::io;
use std::io::prelude::*;
use std::fmt;
use rustc_serialize::json::{self};
use std::process::Command;
use std::process::Output;
use std::env;
use std::path::Path;

macro_rules! log(
    ($($arg:tt)*) => { {
        let r = writeln!(&mut ::std::io::stderr(), $($arg)*);
        r.expect("failed printing to stderr");
    } }
);

//Automatically generate `RustcDecodable` and `RustcEncodable` trait
// implementations
#[derive(RustcDecodable,RustcEncodable,Debug)]
pub struct Source {
    server: String,
    base_dir: String,
}

#[derive(RustcDecodable,RustcEncodable,Debug,Ord,Eq,PartialEq,PartialOrd)]
pub struct Version {
    version: String,
}

#[derive(RustcDecodable,RustcEncodable,Debug)]
pub struct Params {
    static_version: Option<String>,
    skip_download: Option<bool>,
    identificator: String,
}

impl fmt::Display for Version {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "ref: {}", self.version)
    }
}

#[derive(RustcDecodable,RustcEncodable,Debug)]
pub struct Resource {
    source: Source,
    version: Version,
    params: Params,
}

fn main() {
    let zero: String = env::args().nth(0)
        .expect("Can't get env");
    let bin_name = Path::new(&zero).file_stem()
        .expect("Can't get binary name")
        .to_str()
        .expect("Can't convert bin name");
    log!("Name: {}", bin_name);
    match bin_name.as_ref() {
        "in" => concourse_in(),
        "out" => concourse_out(),
        "check" => concourse_check(),
        _ => panic!("binary name must be in | out | check"),
    }
}

fn concourse_out() {
    log!("Run out");
}
fn concourse_in() {
    log!("Run in");
    let destination = env::args().nth(1)
        .expect("Can't get destination");
    log!("Destination folder: {}", destination);
    let mut stdin = String :: new();
    io::stdin().read_to_string(&mut stdin)
        .expect("Can't read stdin");
    let resource: Resource = json::decode(&stdin)
        .expect("Can't decode json from stdin");
    let uri: String = format!("rsync://{}/{}/{}/",resource.source.server, resource.source.base_dir, resource.version.version);
    log!("Uri: {}", uri);
    let copy = Command::new("rsync")
        .arg("-av")
        .arg(uri)
        .arg(destination)
        .output()
        .expect("Can't copy files from rsync server");
    log!("Output: {}\nErrors: {}", String::from_utf8_lossy(&copy.stdout), String::from_utf8_lossy(&copy.stderr));
    println!("{}", json::encode(&resource.version).expect("Can't encode input version"));
}


fn concourse_check() {
    log!("Run check");
    let mut stdin = String :: new();
    io::stdin().read_to_string(&mut stdin)
        .expect("Can't read stdin");
    let resource: Resource = json::decode(&stdin).expect("cant decode json from stdin");
    log!("Input is: {:?}", resource);
    let uri: String = format!("rsync://{}/{}",resource.source.server, resource.source.base_dir);
    let ls = Command::new("rsync")
        .arg(uri)
        .output()
        .expect("Can't get listing from rsync server");
    let result = get_versions(&ls, &resource.version.version[0..4]);
    log!("rsync: {:?}", result);
    println!("{}",json::encode(&result).expect("Can't encode output versions"))
}

fn get_versions(rsync: &Output, mask: &str) -> Vec<Version> {
    let folders = String::from_utf8_lossy(&rsync.stdout);
    let mut result = Vec :: new();
    for line in folders.lines() {
        let ver = Version {
            version: line.split_whitespace().last().expect("cant split rsync line").to_string(),
        };
        if ver.version != "." && &ver.version[0..4] == mask {
            result.push(ver);
        }
    }
    result.sort();
    return result;
}

