extern crate rustc_serialize;
extern crate time;

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
    version: Option<Version>,
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
    let source = env::args().nth(1)
        .expect("Can't get source");
    log!("Source folder: {}", source);
    let mut stdin = String :: new();
    io::stdin().read_to_string(&mut stdin)
        .expect("Can't read stdin");
    let resource: Resource = json::decode(&stdin)
        .expect("Can't decode json from stdin");
    let now = time::now();
    let version = match resource.params.static_version {
        Some(v) => format!("{}-{}",resource.params.identificator, v),
        _ => format!("{}-{}", resource.params.identificator, now.rfc3339()),
    };
    let uri: String = format!("rsync://{}/{}/{}/", resource.source.server, resource.source.base_dir, version);
    let source_folder = format!("{}/",source);
    log!("{}",uri);
    let rsync = Command::new("rsync")
        .arg("-av")
        .arg(source_folder)
        .arg(uri)
        .output()
        .expect("Can't push files to rsync server");
    log!("Output: {}\nErrors: {}", String::from_utf8_lossy(&rsync.stdout), String::from_utf8_lossy(&rsync.stderr));
    println!("{}", json::encode(&version).expect("Can't encode output version"));
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
    let version = resource.version.expect("Don't find version in input");
    let uri: String = format!("rsync://{}/{}/{}/",resource.source.server, resource.source.base_dir, &version.version);
    log!("Uri: {}", uri);
    let rsync = Command::new("rsync")
        .arg("-av")
        .arg(uri)
        .arg(destination)
        .output()
        .expect("Can't pool files from rsync server");
    log!("Output: {}\nErrors: {}", String::from_utf8_lossy(&rsync.stdout), String::from_utf8_lossy(&rsync.stderr));
    println!("{}", json::encode(&version).expect("Can't encode input version"));
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
    let version = &resource.version.expect("Don't find version in input").version;
    let result = get_versions(&ls, &version[0..4], version);
    log!("rsync: {:?}", result);
    println!("{}",json::encode(&result).expect("Can't encode output versions"))
}

fn get_versions(rsync: &Output, mask: &str, current_version: &str) -> Vec<Version> {
    let folders = String::from_utf8_lossy(&rsync.stdout);
    let mut result = Vec :: new();
    for line in folders.lines() {
        let ver = Version {
            version: line.split_whitespace().last().expect("cant split rsync line").to_string(),
        };
        if ver.version != "." && &ver.version[0..4] == mask && ver.version >= current_version.to_string() {
            result.push(ver);
        }
    }
    result.sort();
    return result;
}

