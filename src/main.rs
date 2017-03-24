extern crate rustc_serialize;
extern crate time;

use std::io;
use std::io::prelude::*;
use std::fmt;
use std::process::Command;
use std::env;
use std::borrow::Cow;
use std::path::Path;
use rustc_serialize::json::{self, ToJson, Json};
use rustc_serialize::{Decodable, Decoder};
use std::collections::BTreeMap;

macro_rules! log(
    ($($arg:tt)*) => { {
        let r = writeln!(&mut ::std::io::stderr(), $($arg)*);
        r.expect("failed printing to stderr");
    } }
);


#[derive(RustcDecodable,Debug,Clone)]
struct Source<'a> {
    server: Cow<'a, str>,
    base_dir: Cow<'a, str>,
    static_identificator: Option<Cow<'a, str>>,
    resource_type: Cow<'a, str>
}

#[derive(Debug,Ord,Eq,PartialEq,PartialOrd,Clone)]
struct Version<'a> {
    version: Cow<'a, str>,
}

impl <'a> Version<'a> {
    fn new<S> (version: S) -> Version<'a>
        where S: Into<Cow<'a, str>>
    {
        Version { version: version.into() }
    }
}

impl <'a> ToJson for Version<'a> {
    fn to_json(&self) -> Json {
        let mut d = BTreeMap::new();
        d.insert("ref".to_string(), self.version.to_json());
        Json::Object(d)
    }
}

impl <'a, 'b>Decodable for Version<'a> {
    fn decode<D: Decoder>(d: &mut D) -> Result<Version<'a>, D::Error> {
        d.read_struct("ref", 1, |d| {
            let version = try!(d.read_struct_field("ref", 0, |d| d.read_str()));
            Ok(Version::new(version))
        })
    }
}

#[derive(RustcDecodable,Debug,Clone)]
struct Params<'a> {
    identificator: Option<Cow<'a, str>>,
    sync_dir: Option<Cow<'a, str>>,
}

impl <'a> fmt::Display for Version<'a> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "ref: {}", self.version)
    }
}

#[derive(Debug,Ord,Eq,PartialEq,PartialOrd,Clone)]
enum Out<'a> {
    One(Version<'a>),
    Many(Vec<Version<'a>>),
    Empty,
}

impl <'a> ToJson for Out<'a> {
    fn to_json(&self) -> Json {
        match self {
            &Out::Empty => Json::Object(BTreeMap::new()),
            &Out::One(ref v) => {
                let mut d = BTreeMap::new();
                d.insert("version".to_string(), v.to_json());
                Json::Object(d)
            }
            &Out::Many(ref v) => v.to_json(),
        }
    }
}

#[derive(RustcDecodable,Debug,Clone)]
struct Resource<'a> {
    source: Cow<'a, Source<'a>>,
    version: Option<Cow<'a, Version<'a>>>,
    params: Option<Cow<'a, Params<'a>>>,
}

fn main() {
    let zero: String = env::args().nth(0).expect("Can't get env");
    let bin_name = Path::new(&zero)
        .file_stem()
        .expect("Can't get binary name")
        .to_str()
        .expect("Can't convert bin name");
    let source: String = if bin_name != "check" {
        env::args().nth(1).expect("Can't get env")
    } else {
        "".to_string()
    };
    log!("Name: {}", bin_name);
    let mut stdin = String::new();
    io::stdin().read_to_string(&mut stdin).expect("Can't read stdin");
    log!("{}", stdin);
    let resource: Resource = json::decode(&stdin).expect("Can't decode json from stdin");
    let out = match bin_name.as_ref() {
        "out" => concourse_out(&source, &resource),
        "in" => concourse_in(&source, &resource),
        "check" => concourse_check(&resource),
        _ => Out::Empty,
    };
    //println!("{}", out.to_json().to_string())
    println!("{}", out.to_json().to_string())
}

fn concourse_out<'a>(source: &str, resource: &'a Resource) -> Out<'a> {
    log!("Run out");
    let now = time::now();
    let params = resource.params.clone().expect("Can't find params");
    let version: String = if resource.source.resource_type == Cow::from("w") &&
                     resource.source.static_identificator != None {
        resource.source.static_identificator.clone().unwrap().to_string()
    } else {
        format!("{}-{}",
                params.identificator.clone().expect("identificator must be"),
                now.rfc3339())
    };
    let uri: String = format!("rsync://{}/{}/{}/",
                              resource.source.server,
                              resource.source.base_dir,
                              version);
    let source_folder = format!("{}/{}/", source, params.sync_dir.clone().expect("sync_dir must be"));
    log!("{}", uri);
    let rsync = Command::new("rsync")
        .arg("-av")
        .arg(source_folder)
        .arg(uri)
        .output()
        .expect("Can't push files to rsync server");
    let out = Out::One(Version::new(version));
    log!("Output: {}\nErrors: {}\nList: {:?}",
         String::from_utf8_lossy(&rsync.stdout),
         String::from_utf8_lossy(&rsync.stderr),
         out);
    return out;

}

fn concourse_in<'a>(source: &String, resource: &'a Resource) -> Out<'a> {
    log!("Run in");
    let resouce_type = resource.source.resource_type.clone();
    if resouce_type == "w" {
        log!("Skip, write only");
        return Out::Empty;
    } else {
        let version = resource.version.clone().expect("Can't find input version").version.clone();
        let uri: String = format!("rsync://{}/{}/{}/",
                                  resource.source.server,
                                  resource.source.base_dir,
                                  &version);
        log!("Uri: {}", uri);
        let rsync = Command::new("rsync")
            .arg("-av")
            .arg(uri)
            .arg(source)
            .output()
            .expect("Can't pool files from rsync server");
        let out = Out::One(Version::new(version));
        log!("Output: {}\nErrors: {}\nList: {:?}",
             String::from_utf8_lossy(&rsync.stdout),
             String::from_utf8_lossy(&rsync.stderr),
             out);
        return out;
    }
}
//
//// if resource used as input only
//// it has only resource.source as json on input
//// if resource used as output only
//// it has resource.source
////        resource.params

fn concourse_check<'a>(resource: &'a Resource) -> Out<'a> {
    log!("Run check");
    let mut result: Vec<Version> = Vec::new();
    let uri: String = format!("rsync://{}/{}",
                              resource.source.server,
                              resource.source.base_dir);
    let (mask, version): (String, String) = match (resource.version.clone(), resource.source.static_identificator.clone()) {
        (Some(v), _) => (v.version[0..4].to_string(), v.version.to_string()),
        (None, Some(v)) => (v[0..4].to_string(), v.to_string()),
        _ => panic!("Can't find static_identificator or version"),
    };
    let ls = Command::new("rsync").arg(uri).output().expect("Can't get listing from rsync server");
    let folders = String::from_utf8(ls.stdout).expect("Check filesystem, bad utf8");
    for line in folders.lines() {
        let folder: &str = line.split_whitespace().last().expect("Can't split rsync lline");
        if folder != ".".to_string() && folder[0..4] == mask && folder.to_string() >= version {
            result.push(Version::new(folder.to_string()));
        }
    }
    result.sort();
    return Out::Many(result)
}
