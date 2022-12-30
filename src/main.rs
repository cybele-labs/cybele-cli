extern crate cybele_core;
extern crate rpassword;
extern crate rprompt;

use std::fs::OpenOptions;
use std::io::Write;
use std::path::Path;
use std::{env, fs};

use cybele_core::vault::Vault;

struct StartupArgs {
    vault_file: String,
}

fn parse_startup_args() -> Result<StartupArgs, ()> {
    if env::args().len() < 2 {
        println!("Cybele requires the path to an encrypted vault: `cybele-cli <path-to-vault-file>`.");
        return Err(());
    }
    let vault_file = env::args().nth(1).unwrap();
    Ok(StartupArgs { vault_file })
}

fn load_vault(filename: &str, password: &str) -> Result<Vault, ()> {
    let serialized_vault = match fs::read(&filename) {
        Ok(s) => s,
        Err(_) => {
            println!("Could not read vault file.");
            return Err(());
        }
    };
    let vault = match Vault::deserialize(&serialized_vault, password) {
        Some(vault) => vault,
        None => {
            println!("Could not decrypt vault file.");
            return Err(());
        }
    };
    Ok(vault)
}

fn help() {
    println!("Available commands:");
    println!("  - add");
    println!("  - remove");
    println!("  - get");
    println!("  - list");
    println!("  - save");
    println!("  - exit");
}

fn main() {
    println!();
    println!("Welcome to the Cybele password manager.");
    let args = match parse_startup_args() {
        Ok(startup_args) => startup_args,
        Err(()) => return,
    };

    let mut vault: Vault = if !Path::new(&args.vault_file).exists() {
        println!("Vault file does not exist: creating a new vault...");
        Vault::create(None)
    } else {
        println!("Loading encrypted vault...");
        let password = rpassword::prompt_password("Master password: ").unwrap();
        match load_vault(&args.vault_file, &password) {
            Ok(vault) => {
                println!("Vault successfully loaded.");
                vault
            }
            Err(()) => return,
        }
    };

    println!("Enter \"help\" to list available commands.");
    println!();
    loop {
        let command = rprompt::prompt_reply("> ").unwrap();
        match command.as_str() {
            "help" => help(),
            "add" => {
                let name = rprompt::prompt_reply("Name: ").unwrap();
                let hidden = rprompt::prompt_reply("Hide input (y/n): ").unwrap();
                let password = match hidden.as_str() {
                    "y" => rpassword::prompt_password("Password: ").unwrap(),
                    _ => rprompt::prompt_reply("Password: ").unwrap(),
                };
                let master_password = rpassword::prompt_password("Master password: ").unwrap();
                match vault.add(&name, &password, &master_password) {
                    Some(_) => {
                        println!("Password added for <{}>", name.clone());
                        println!("Don't forget to use the `save` command to save your changes.");
                    }
                    None => {
                        println!("Password could not be added for <{}>", name.clone());
                    }
                };
            }
            "remove" => {
                let name = rprompt::prompt_reply("Name: ").unwrap();
                vault.remove(name.as_str());
                println!("Password for <{}> removed", name);
                println!("Don't forget to use the `save` command to save your changes.");
            }
            "get" => {
                let name = rprompt::prompt_reply("Name: ").unwrap();
                let master_password = rpassword::prompt_password("Master password: ").unwrap();
                match vault.get(&name, &master_password) {
                    Some(password) => println!("{}", &password),
                    None => println!("Could not find or decrypt password for <{}>", name.clone()),
                }
            }
            "list" => {
                vault.list().iter().for_each(|p| println!("{}", p));
            }
            "save" => {
                let master_password = rpassword::prompt_password("Master password: ").unwrap();
                match vault.serialize(&master_password) {
                    Some(serialized) => {
                        if !Path::new(&args.vault_file).exists() {
                            OpenOptions::new().read(true).write(true).create(true).open(&args.vault_file).unwrap();
                        }
                        let mut vault_file = OpenOptions::new().write(true).truncate(true).open(&args.vault_file).unwrap();
                        vault_file.write_all(&serialized).unwrap();
                        vault_file.sync_all().unwrap();
                        println!("Vault successfully saved.");
                    }
                    None => println!("Could not encrypt vault."),
                };
            }
            "exit" => break,
            _ => println!("Unknown command: enter \"help\" to list available commands."),
        }
    }
}
