/// Call the Solidity compiler
extern crate rustc_hex;

use std::fs::File;
use std::io::{BufReader, Read};
use std::path::PathBuf;
use std::str;

use rustc_hex::FromHex;

// TODO: add compile builder

#[derive(Debug)]
pub struct Solc<'a> {
    root: String,
    pub output_dir: Option<&'a str>,
    allow_paths: Vec<String>,
}

impl<'a> Solc<'a> {
    pub fn new(root: &str) -> Self {
        Solc {
            root: root.to_owned(),
            output_dir: None,
            allow_paths: Vec::<String>::new(),
        }
    }

    pub fn root(&self) -> &str {
        &self.root[..]
    }

    pub fn output_dir(&self) -> &str {
        self.output_dir.unwrap()
    }

    // load from <root>/<output_dir>/<name>
    // only load LINKED bytecode
    pub fn load_bytecode(&self, name: &str) -> Vec<u8> {
        match self.output_dir {
            Some(ref dir) => {
                let bytecode_path: PathBuf = [self.root.as_str(), dir, name].iter().collect();
                println!("bytecode at: {:?}", bytecode_path);
                // TODO: use combinators
                let path = format!("{}", bytecode_path.display());
                let bytes = load_bytes(&path[..]);
                let code = str::from_utf8(&bytes[..]).unwrap();
                // println!("CODE: {}", code);
                // bytecode_path.as_path()
                code.from_hex().unwrap()
                // code
            }
            None => panic!("No output path set"),
        }
    }

    // load from <root>/<output_dir>/<name>
    pub fn load_abi(&self, name: &str) -> Vec<u8> {
        match self.output_dir {
            Some(ref dir) => {
                let abi_path: PathBuf = [self.root.as_str(), dir, name].iter().collect();
                let path: &str = abi_path.to_str().unwrap();
                load_bytes(path)
                // let abi: &str = str::from_utf8(v.as_slice()).expect("Could not load string");
                // // let abi: &str = str::from_utf8(load_bytes(path));
                // // want string -> slice of bytes
                // abi.
                // v.as_slice()
            }
            None => panic!("No output path set"),
        }
    }

    // fn path(relative: &str) -> &Path {
    //     let p: PathBuf = [self.root.to_str(), relative].iter().collect();
    //     p.as_path()
    // }
}

// TODO: return Result
fn load_bytes(path: &str) -> Vec<u8> {
    match File::open(path) {
        Ok(file) => {
            let mut reader = BufReader::new(file);
            let mut contents: Vec<u8> = Vec::new();

            match reader.read_to_end(&mut contents) {
                Ok(_) => contents,
                Err(e) => panic!("Problem reading file {}", e),
            }
        }
        Err(e) => panic!("Could not open file {}: {}", path, e),
    }
}

#[cfg(test)]
mod test {
    // use super::*;

    // #[test]
    // check for solc exe
    // compile
    // load bytecode
    // load unlinked bytecode
}
