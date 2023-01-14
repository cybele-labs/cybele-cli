extern crate cybele_core;
extern crate rpassword;
extern crate rprompt;

use std::env;
use std::path::Path;

use cybele_core::vault::Vault;

mod file;

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

fn prompt_password(prompt: &str) -> String {
    let password = rpassword::prompt_password(prompt).unwrap();
    let password2 = rpassword::prompt_password("Please enter password again: ").unwrap();
    if password != password2 {
        println!("The two passwords are not equal, let's try again...");
        prompt_password(prompt)
    } else {
        password
    }
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
        let password = prompt_password("Master password: ");
        match file::load_plain_vault(&args.vault_file, &password) {
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
                    "y" => prompt_password("Password: "),
                    _ => rprompt::prompt_reply("Password: ").unwrap(),
                };
                let master_password = prompt_password("Master password: ");
                match vault.add(&name, &password, &master_password) {
                    Some(_) => {
                        println!("Password added for <{}>", &name);
                        println!("Don't forget to use the `save` command to save your changes.");
                    }
                    None => {
                        println!("Password could not be added for <{}>", &name);
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
                let items = vault.list();
                let id: usize = rprompt::prompt_reply("ID: ").unwrap().parse().unwrap();
                if id < items.len() {
                    let name: &str = &items[id];
                    let master_password = prompt_password("Master password: ");
                    match vault.get(name, &master_password) {
                        Some(password) => println!("{}: {}", name, &password),
                        None => println!("Could not find or decrypt password for <{}>", name),
                    }
                } else {
                    println!("Invalid ID <{}>", id);
                }
            }
            "list" => {
                let filter = rprompt::prompt_reply("Filter: ").unwrap();
                vault.list().iter().enumerate().for_each(|(pos, item)| {
                    if item.contains(&filter) {
                        println!("{}: {}", pos, item)
                    }
                });
            }
            "save" => {
                let master_password = prompt_password("Master password: ");
                file::save_plain_vault(&vault, &args.vault_file, &master_password);
            }
            "exit" => break,
            _ => println!("Unknown command: enter \"help\" to list available commands."),
        }
    }
}
