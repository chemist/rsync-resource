extern crate rustc_serialize;
extern crate time;

use std::io;
use std::io::prelude::*;
use std::fmt;
use std::process::Command;
use std::process::Output;
use std::env;
use std::path::Path;
use rustc_serialize::json::{self, ToJson, Json};
use rustc_serialize::{Decodable,Decoder};
use std::collections::BTreeMap;

macro_rules! log(
    ($($arg:tt)*) => { {
        let r = writeln!(&mut ::std::io::stderr(), $($arg)*);
        r.expect("failed printing to stderr");
    } }
);


#[derive(RustcDecodable,Debug)]
pub struct Source {
    server: String,
    base_dir: String,
    static_identificator: Option<String>,
    resource_type: String,
}

#[derive(Debug,Ord,Eq,PartialEq,PartialOrd)]
pub struct Version {
    version: String,
}

impl ToJson for Version {
    fn to_json(&self) -> Json {
        let mut d = BTreeMap::new();
        d.insert("ref".to_string(), self.version.to_json());
        Json::Object(d)
    }
}

impl Decodable for Version {
    fn decode<D: Decoder>(d: &mut D) -> Result<Version, D::Error> {
        d.read_struct("ref", 1, |d| {
            let version = try!(d.read_struct_field("ref", 0, |d| { d.read_str()}));
            Ok(Version{ version: version})
        })
    }
}

#[derive(RustcDecodable,Debug)]
pub struct Params {
    identificator: Option<String>,
    sync_dir: Option<String>,
}

impl fmt::Display for Version {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "ref: {}", self.version)
    }
}

#[derive(Debug,Ord,Eq,PartialEq,PartialOrd)]
enum Out {
    One(Version) ,
    Many(Vec<Version>),
    Empty,
}

impl ToJson for Out {
    fn to_json(&self) -> Json {
        match self {
            &Out::Empty => Json::Object(BTreeMap::new()),
            &Out::One(ref v) => {
                let mut d = BTreeMap::new();
                d.insert("version".to_string(),v.to_json());
                Json::Object(d)
            },
            &Out::Many(ref v) => {
                v.to_json()
            }
        }
    }
}

#[derive(RustcDecodable,Debug)]
pub struct Resource {
    source: Source,
    version: Option<Version>,
    params: Option<Params>,
}

fn main() {
    let zero: String = env::args().nth(0)
        .expect("Can't get env");
    let bin_name = Path::new(&zero).file_stem()
        .expect("Can't get binary name")
        .to_str()
        .expect("Can't convert bin name");
    let source: String = if bin_name != "check" {
        env::args().nth(1)
            .expect("Can't get env")
    } else { "".to_string() };
    log!("Name: {}", bin_name);
    let mut stdin = String :: new();
    io::stdin().read_to_string(&mut stdin)
        .expect("Can't read stdin");
    log!("{}", stdin);
    let resource: Resource = json::decode(&stdin)
        .expect("Can't decode json from stdin");
    let out = match bin_name.as_ref() {
        "out" => concourse_out(source, resource),
        "in" => concourse_in(source, resource),
        "check" => concourse_check(source, resource),
        _ => Out::Empty
    };
    //println!("{}", out.to_json().to_string()) 
    println!("{}", out.to_json().to_string()) 
}

fn concourse_out(source: String, resource: Resource) -> Out {
    log!("Run out");
    let now = time::now();
    let params = resource.params
        .expect("Can't find params");
    let version = if resource.source.resource_type == "w" && resource.source.static_identificator != None {
        resource.source.static_identificator.unwrap()
    } else {
        format!("{}-{}", params.identificator.expect("identificator must be"), now.rfc3339())
    };
    let uri: String = format!("rsync://{}/{}/{}/", resource.source.server, resource.source.base_dir, version);
    let source_folder = format!("{}/{}/", source, params.sync_dir.expect("sync_dir must be"));
    log!("{}",uri);
    let rsync = Command::new("rsync")
        .arg("-av")
        .arg(source_folder)
        .arg(uri)
        .output()
        .expect("Can't push files to rsync server");
    let out = Out::One(Version { version: version});
    log!("Output: {}\nErrors: {}\nList: {:?}", String::from_utf8_lossy(&rsync.stdout), String::from_utf8_lossy(&rsync.stderr), out);
    return out

}

fn concourse_in(source: String, resource: Resource) -> Out {
    log!("Run in");
    let resouce_type = resource.source.resource_type;
    if resouce_type == "w" {
        log!("Skip, write only");
        return Out::Empty 
    } else {
      let version = resource.version.expect("Can't find input version").version;
      let uri: String = format!("rsync://{}/{}/{}/",resource.source.server, resource.source.base_dir, &version);
      log!("Uri: {}", uri);
      let rsync = Command::new("rsync")
          .arg("-av")
          .arg(uri)
          .arg(source)
          .output()
          .expect("Can't pool files from rsync server");
      let out = Out::One(Version { version: version});
      log!("Output: {}\nErrors: {}\nList: {:?}", String::from_utf8_lossy(&rsync.stdout), String::from_utf8_lossy(&rsync.stderr), out);
      return out
    }
}

// if resource used as input only
// it has only resource.source as json on input
// if resource used as output only
// it has resource.source
//        resource.params
fn concourse_check(_: String, resource: Resource) -> Out {
    log!("Run check");
    let uri: String = format!("rsync://{}/{}",resource.source.server, resource.source.base_dir);
    let ls = Command::new("rsync")
        .arg(uri)
        .output()
        .expect("Can't get listing from rsync server");
    match (resource.version, resource.source.static_identificator) {
        (Some(v), _)  => {
            let version = v.version;
            let result = get_versions(&ls, &version[0..4], &version);
            log!("Have version");
            log!("rsync: {:?}", result);
            return Out::Many(result)
        },
        (None, Some(si)) => {
            let result = get_versions(&ls, &si[0..4], &si);
            log!("Dont have version");
            log!("rsync: {:?}", result);
            return Out::Many(result)
        }
        _ => Out::Empty
    }
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

