use std::{collections::HashMap, str::FromStr};
use std::io::Write;
use serde::{Serialize, Deserialize};

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

fn main() {
    println!("Hello World");
}