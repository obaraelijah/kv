use std::error::Error;
use std::{collections::HashMap, str::FromStr};
use std::io::Write;
use serde::{Serialize, Deserialize};
use std::path::{Path, PathBuf};
use std::fs::OpenOptions;



use clap;
use dirs;
use human_panic;
use serde_json;
use tabwriter;

type KV = HashMap<String, String>;

#[derive(Serialize, Deserialize, PartialEq, Debug)]
enum OpType {
    Get,
    Set,
    Del,
}

#[derive(Serialize, Deserialize, Debug)]
struct Hook {
    name: String,
    cmd_name: String,
    run_on: OpType,
    key: String,
}

#[derive(Serialize, Deserialize, Default)]
struct KVStore {
    kvs: KV,
    cmds: KV,
    hooks: Vec<Hook>,
}


impl std::fmt::Display for OpType {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        let str_rep = match self {
            OpType::Get => "get",
            OpType::Set => "set",
            OpType::Del => "del",
        };
        write!(f, "{}", str_rep)
    }
}

impl FromStr for OpType {
    type Err = &'static str;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "get" => Ok(OpType::Get),
            "set" => Ok(OpType::Set),
            "del" => Ok(OpType::Del),
            _ => Err("No match found!"),
        }
    }
}

fn get_file_location() -> PathBuf {
    match dirs::config_dir() {
        Some(home) => {
            let store_file_dir_path = home.join("kv");
            if !store_file_dir_path.exists() {
                match std::fs::create_dir_all(&store_file_dir_path) {
                    Ok(_) => {
                        println!(
                            "Created config dir path {}",
                            store_file_dir_path.to_string_lossy()
                        );
                    }
                    Err(e) => {
                        let err_msg = format!(
                            "Error! Cannot create path {}, error {}",
                            store_file_dir_path.to_string_lossy(),
                            e.to_string()
                        );
                        eprintln!("{}", err_msg);
                    }
                }
            }
            store_file_dir_path.join("kv.json")
        }
        None => {
            eprintln!("Error! Cannot find the config directory!");
            PathBuf::new()
        }
    }
}

fn get_file() -> std::fs::File {
    OpenOptions::new()
        .read(true)
        .write(true)
        .create(true)
        .append(false)
        .open(get_file_location())
        .unwrap()
}

fn write_file(m: &KVStore) {
    let mut file = get_file();
    file.set_len(0).unwrap();
    let s = serde_json::to_string_pretty(m).unwrap();
    file.write_all(s.as_bytes()).unwrap();
}

fn main() {
    println!("Hello World");
}
