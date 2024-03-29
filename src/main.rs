use std::env;
use std::process::Command;
use std::{collections::HashMap, str::FromStr};
use std::io::Write;
use serde::{Serialize, Deserialize};
use std::path::{Path, PathBuf};
use std::fs::OpenOptions;

use clap::{self, value_t, App, AppSettings, Arg, ArgMatches, SubCommand};
use human_panic::{self, setup_panic};
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
                            e
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
    if let Err(e) = Command::new(shell).arg("-c").arg(cmd).spawn() {
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

    if let Some(cmd) = matches.subcommand_matches("cmd") {
        if let Some(m_run) = cmd.subcommand_matches("run") {
            let cmd_name = m_run.value_of("cmd-name").unwrap();
            let cmd_value = get_key(cmd_name, &kvstore.cmds);
            match cmd_value {
                Some(v) => run_command(cmd_name, &v),
                None => println!("Error! Command {} does not exist!", cmd_name),
            }
        } 

        if let Some(m_add) = cmd.subcommand_matches("add") {
            let cmd_name = m_add.value_of("cmd-name").unwrap();
            let cmd_value = m_add.value_of("cmd-value").unwrap();
            set_key(cmd_name, cmd_value, &mut kvstore.cmds);
            write_file(&kvstore);
        }

        if let Some(m_del_hook) = cmd.subcommand_matches("del-hook") {
            let hook_name = m_del_hook.value_of("hook-name").unwrap();
            rm_hook(hook_name);
        }

        if let Some(m_add_hook) = cmd.subcommand_matches("add-hook") {
            let hook_name = m_add_hook.value_of("hook-name").unwrap();
            let cmd_name = m_add_hook.value_of("cmd-name").unwrap();
            let trigger_op = value_t!(m_add_hook, "trigger", OpType).unwrap();
            let key = m_add_hook.value_of("key").unwrap();
            add_hook(
                hook_name.to_owned(),
                cmd_name.to_owned(),
                trigger_op,
                key.to_owned(),
            )
        }
    }
}

/// Fooar
fn main() {
    setup_panic!();
    let matches = App::new("kv")
        .version("0.2")
        .author("Elijah Samson(elijahobara357@gmail.com)")
        .about("Simple key, value storage with hooks.")
        .setting(AppSettings::SubcommandRequiredElseHelp)
        .about("Key-Value Storage with bash command hooks. Add hooks to run commands on variable update.")
        .subcommand(SubCommand::with_name("list")
                    .about("List keys, cmds, or hooks.")
                    .arg(Arg::with_name("to-list")
                         .takes_value(true)
                         .required(false)
                    .possible_values(&["keys", "cmds", "hooks"])))
        .subcommand(
            SubCommand::with_name("cmd")
                .setting(AppSettings::SubcommandRequiredElseHelp)
                .about("Add, and Run bash commands. Add hooks to run commands on variable update.")
                .subcommand(
                    SubCommand::with_name("run")
                        .about("Run commands <cmd-name>")
                        .arg(Arg::with_name("cmd-name").takes_value(true).required(true)),
                )
                .subcommand(
                    SubCommand::with_name("add")
                        .about("Add command with name <cmd-name>, and value <cmd-value>")
                        .arg(Arg::with_name("cmd-name").takes_value(true).required(true))
                        .arg(Arg::with_name("cmd-value").takes_value(true).required(true)),
                )
            .subcommand(
                SubCommand::with_name("add-hook")
                    .about("Add hook with name <hook-name> to run <cmd-name> when [key] is updated (kv get, kv set, kv del)")
                    .arg(Arg::with_name("hook-name").takes_value(true).required(true))
                    .arg(Arg::with_name("cmd-name").takes_value(true).required(true))
                    .arg(Arg::with_name("trigger").takes_value(false).required(true).possible_values(&["get", "set", "del"]))
                    .arg(Arg::with_name("key").takes_value(true).required(true))
            )
            .subcommand(
                SubCommand::with_name("del-hook")
                    .about("Remove hook with name <hook-name>")
                    .arg(Arg::with_name("hook-name").takes_value(true).required(true))
            )
        )
        .subcommand(
            SubCommand::with_name("get")
                .about("Get key from storage")
                .help(
                    r#"kv get <key>

Get the value of <key> from storage

Example:
~> kv set my-key my-key-value
~> kv get my-key
my-key-value
"#,
                )
                .arg(
                    Arg::with_name("key")
                        .help("key to get from storage")
                        .takes_value(true)
                        .required(true),
                ),
        )
        .subcommand(
            SubCommand::with_name("del")
                .help(
                    r#"kv del <key>

Delete <key> in storage (and its value)

Example:
~> kv set my-key my-key-value
~> kv del my-key
~> kv get my-key

~>
"#,
                )
                .about("Delete key and value from storage")
                .arg(
                    Arg::with_name("key")
                        .help("key to delete from storage")
                        .takes_value(true)
                        .required(true),
                ),
        )
        .subcommand(
            SubCommand::with_name("set")
                .about("set key to value in storage")
                .help(
                    r#"kv set <key> <val>

Set <key> to <val> in storage.

Example:
~> kv set my-key my-key-value
~> kv get my-key
my-key-value
"#,
                )
                .arg(
                    Arg::with_name("key")
                        .help("key to set in storage")
                        .takes_value(true)
                        .required(true),
                )
                .arg(
                    Arg::with_name("val")
                        .help("<val> you wish to set <key> to.")
                        .takes_value(true)
                        .required(true),
                ),
        )
        .get_matches();
    run(matches);
}