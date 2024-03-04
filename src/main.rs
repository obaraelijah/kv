use std::env;
use std::process::Command;
use std::{collections::HashMap, str::FromStr};
use std::io::Write;
use serde::{Serialize, Deserialize};
use std::path::{Path, PathBuf};
use std::fs::OpenOptions;



use clap::{self, ArgMatches};
use dirs;
use human_panic;
use serde_json;
use tabwriter::TabWriter;

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
            let store_file_dir_path = Path::new(&home).join("kv");
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
                        print_err(&err_msg[..])
                    }
                }
            }
            Path::new(&store_file_dir_path).join("kv.json")
        }
        None => {
            print_err("Error! Cannot find the config directory!");
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

/// Lets you run a command
fn run_command(cmd_name: &str, cmd: &str) {
    let shell = match env::var("SHELL") {
        Ok(s) => s,
        Err(_) => "bash".to_owned(),
    };
    if let Err(e) = Command::new(shell).arg("-c").arg(&cmd).spawn() {
        let err_msg = format!(
            "Error! Failed to run '{}' with error:\n {:?}",
            cmd_name,
            e.to_string()
        );
        print_err(&err_msg[..]);
    }
}

fn run_hooks(key_name: &str, current_op: &OpType) {
    let kvstore: KVStore = get_store();
    let hooks_to_run: Vec<&Hook> = kvstore
        .hooks
        .iter()
        .filter(|&x| x.run_on == *current_op && x.key == key_name)
        .collect();
    for hook in hooks_to_run {
        match get_key(&hook.cmd_name[..], &kvstore.cmds) {
            Some(cmd) => run_command(&hook.cmd_name, &cmd),
            None => println!("Error! Bad hook! Hook {:?} has no cmd!", hook.name),
        }
    }
}

/// Get the store as KVStore
fn get_store() -> KVStore {
    match serde_json::from_reader(get_file()) {
        Ok(s) => s,
        Err(_) => Default::default(),
    }
}

fn add_hook(name: String, cmd_name: String, run_on: OpType, key: String) {
    let mut kvstore = get_store();
    if kvstore.hooks.iter().filter(|&x| x.name == name).count() > 0 {
        let err_msg = format!(
            "Error! {} already exists. To delete it try\n kv cmd del-hook {}",
            name, name
        );
        print_err(&err_msg[..]);
    }
    let new_hook = Hook {
        name,
        cmd_name,
        run_on,
        key,
    };

    kvstore.hooks.push(new_hook);
    write_file(&kvstore)
}


fn rm_hook(name: &str) {
    let mut kvstore = get_store();
    match kvstore.hooks.iter().position(|x| x.name == name) {
        Some(pos) => {
            kvstore.hooks.remove(pos);
        }
        None => {
            let err_msg = format!("Error! Hook {} does not exist!", name);
            print_err(&err_msg[..]);
        }
    }
    write_file(&kvstore);
}


fn get_key(s: &str, map: &KV) -> Option<String> {
    map.get(&s.to_owned()).cloned()
}

fn set_key(k: &str, v: &str, map: &mut KV) {
    map.insert(k.to_owned(), v.to_owned());
}

fn del_key(k: &str, map: &mut KV) -> Option<String> {
    map.remove(&k.to_owned())
}

fn print_res(s: Option<String>) {
    match s {
        Some(s) => println!("{}", s),
        None => println!(),
    }
}

fn print_aligned(v: Vec<String>) {
    let mut t = TabWriter::new(vec![]);
    write!(&mut t, "{}", v.join("\n")).unwrap();
    t.flush().unwrap();
    println!("{}", String::from_utf8(t.into_inner().unwrap()).unwrap());
}

fn print_err(s: &str) -> ! {
    println!("{}", s);
    std::process::exit(1);
}

fn run(matches: ArgMatches) {
    let mut kvstore = get_store();
    if let Some(get) = matches.subcommand_matches("get") {
        let key = get.value_of("key").unwrap();
        let value = get_key(key, &kvstore.kvs);
        print_res(value);
        run_hooks(key, &OpType::Get);
    }
    if let Some(set) = matches.subcommand_matches("set") {
        let key = set.value_of("key").unwrap();
        let value = set.value_of("val").unwrap();
        set_key(key, value, &mut kvstore.kvs);
        write_file(&kvstore);
        run_hooks(key, &OpType::Set);
    }
    if let Some(del) = matches.subcommand_matches("del") {
        let key = del.value_of("key").unwrap();
        let value = del_key(key, &mut kvstore.kvs);
        write_file(&kvstore);
        print_res(value);
        run_hooks(key, &OpType::Del);
    }
    if let Some(to_list) = matches.subcommand_matches("list") {
        let key = to_list.value_of("to-list");
        let kvstore = get_store();

        let print_cmds = |kvstore: &KVStore| {
            let mut start = vec!["Key\t--\tValue".to_owned()];
            let mut to_print = kvstore
                .cmds
                .iter()
                .map(|(key, val)| format!("{}\t--\t{}", key, val))
                .collect::<Vec<String>>();
            start.append(&mut to_print);
            print_aligned(start);
        };
        
        let print_keys = |kvstore: &KVStore| {
            let mut start = vec!["Key\t--\tValue".to_owned()];
            let mut to_print = kvstore
                .kvs
                .iter()
                .map(|(key, val)| format!("{}\t--\t{}", key, val))
                .collect::<Vec<String>>();
            start.append(&mut to_print);
            print_aligned(start);
        };

        let print_hooks = |kvstore: &KVStore| {
            let mut start = vec!["Hook Name\t--\tCmd Name\t--\tTrigger\t--\tKey".to_owned()];
            let mut to_print = kvstore
                .hooks
                .iter()
                .map(|hook| {
                    format!(
                        "{}\t--\t{}\t--\t{}\t--\t{}",
                        hook.name, hook.cmd_name, hook.run_on, hook.key
                    )
                })
                .collect::<Vec<String>>();
            start.append(&mut to_print);
            print_aligned(start);
        };
        match key {
            Some("cmds") => {
                print_cmds(&kvstore);
            }
            Some("keys") => {
                print_keys(&kvstore);
            }
            Some("hooks") => {
                print_hooks(&kvstore);
            }
            None => {
                print_keys(&kvstore);
                println!("-------------------");
                print_cmds(&kvstore);
                println!("-------------------");
                print_hooks(&kvstore);
            }
            _ => print_err("Error! Unknown subject to list!"),
        }
    }

}
