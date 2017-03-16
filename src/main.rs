extern crate rustc_serialize;

//use rustc_serialize::json;
use std::io;
use std::io::prelude::*;
//use std::collections::BTreeMap;
use std::fmt;
use rustc_serialize::json::{self};

//Automatically generate `RustcDecodable` and `RustcEncodable` trait
// implementations
#[derive(RustcDecodable,RustcEncodable,Debug)]
pub struct Source {
    server: String,
    base_dir: String,
}

#[derive(RustcDecodable,RustcEncodable,Debug)]
pub struct Version {
    data: String,
}

impl fmt::Display for Version {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "ref: {}", self.data)
    }
}

#[derive(RustcDecodable,RustcEncodable,Debug)]
pub struct Check {
    source: Source,
    version: Version,
}

fn main() {
    concourse_check();
    concourse_in();
    concourse_out();
}

macro_rules! log(
    ($($arg:tt)*) => { {
        let r = writeln!(&mut ::std::io::stderr(), $($arg)*);
        r.expect("failed printing to stderr");
    } }
);

fn concourse_out() {
    log!("Run out");
}
fn concourse_in() {
    log!("Run in");
}


fn concourse_check() {
    log!("Run check");
    let mut input = String :: new();
    io::stdin().read_to_string(&mut input)
        .expect("Failed to read input");
    let check: Check = json::decode(&input).unwrap();
    log!("Input is: {:?}", check);
    println!("[]")
}
