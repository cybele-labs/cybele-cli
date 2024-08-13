extern crate cybele_core;
extern crate rpassword;
extern crate rprompt;

use std::env;
use std::path::Path;

use cybele_core::vault::Vault;

mod file;

enum VaultFormat {
    Plain,
    Image,
}

struct StartupArgs {
    vault_file: String,
    vault_format: VaultFormat,
}

fn parse_startup_args() -> Result<StartupArgs, ()> {
    if env::args().len() < 2 {
        println!("Cybele requires the path to an encrypted vault: `cybele-cli <path-to-vault-file>`.");
        return Err(());
    }
    let vault_file = env::args().nth(1).unwrap();
    let vault_format = if vault_file.ends_with(".bmp") { VaultFormat::Image } else { VaultFormat::Plain };
    Ok(StartupArgs { vault_file, vault_format })
}

fn prompt_master_password() -> String {
    rpassword::prompt_password("Enter your master password: ").unwrap()
}

fn prompt_password_repeat(prompt: &str) -> String {
    let password = rpassword::prompt_password(prompt).unwrap();
    let password2 = rpassword::prompt_password("Please enter password again: ").unwrap();
    if password != password2 {
        println!("The two passwords are not equal, let's try again...");
        prompt_password_repeat(prompt)
    } else {
        password
    }
}

fn generate_password(password_len: usize) -> String {
    let password = cybele_core::password::generate_password(password_len);
    let regen = rprompt::prompt_reply(format!("Password = {}, do you want to generate a new one (y/n)? ", password)).unwrap();
    match regen.as_str() {
        "y" => generate_password(password_len),
        _ => password,
    }
}

fn prompt_color() -> file::Color {
    match rprompt::prompt_reply("Color (red|green|blue): ").unwrap().to_lowercase().as_str() {
        "red" => file::Color::Red,
        "green" => file::Color::Green,
        "blue" => file::Color::Blue,
        _ => {
            println!("Unknown color: valid choices are red, green or blue.");
            prompt_color()
        }
    }
}

fn add_to_vault(vault: &mut Vault, name: &str, password: &str, master_password: &str) {
    match vault.add(name, password, master_password) {
        Some(_) => {
            println!("Password added for <{}>", &name);
            println!("Don't forget to use the `save` command to save your changes.");
        }
        None => {
            println!("Password could not be added for <{}>", &name);
        }
    };
}

fn help() {
    println!("Available commands:");
    println!("  - add");
    println!("  - generate");
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
        let password = prompt_master_password();
        let loading = match args.vault_format {
            VaultFormat::Plain => file::load_plain_vault(&args.vault_file, &password),
            VaultFormat::Image => file::load_image_vault(&args.vault_file, &password),
        };
        match loading {
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
        println!();
        match command.as_str() {
            "help" => help(),
            "add" => {
                let name = rprompt::prompt_reply("Name: ").unwrap();
                let hidden = rprompt::prompt_reply("Hide input (y/n): ").unwrap();
                let password = match hidden.as_str() {
                    "y" => prompt_password_repeat("Password: "),
                    _ => rprompt::prompt_reply("Password: ").unwrap(),
                };
                let master_password = prompt_master_password();
                add_to_vault(&mut vault, &name, &password, &master_password);
            }
            "generate" => {
                let name = rprompt::prompt_reply("Name: ").unwrap();
                let password_len: usize = rprompt::prompt_reply("Password length: ").unwrap().parse().unwrap();
                let password = generate_password(password_len);
                let master_password = prompt_master_password();
                add_to_vault(&mut vault, &name, &password, &master_password);
            }
            "remove" => {
                let name = rprompt::prompt_reply("Name: ").unwrap();
                vault.remove(name.as_str());
                println!("  Password for <{}> removed", name);
                println!("  Don't forget to use the `save` command to save your changes.");
            }
            "get" => {
                let items = vault.list();
                let id: usize = rprompt::prompt_reply("ID: ").unwrap().parse().unwrap();
                if id < items.len() {
                    let name: &str = &items[id];
                    let master_password = prompt_master_password();
                    match vault.get(name, &master_password) {
                        Some(password) => {
                            println!("  - name: {}", name);
                            println!("  - password: {}", &password);
                        }
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
                        println!("  - {}: {}", pos, item)
                    }
                });
            }
            "save" => {
                let master_password = prompt_master_password();
                match args.vault_format {
                    VaultFormat::Plain => file::save_plain_vault(&vault, &args.vault_file, &master_password),
                    VaultFormat::Image => {
                        let color = prompt_color();
                        file::save_image_vault(&vault, color, &args.vault_file, &master_password);
                    }
                }
            }
            "exit" => break,
            _ => println!("Unknown command: enter \"help\" to list available commands."),
        }
        println!();
    }
}
