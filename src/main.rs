extern crate cybele_core;
extern crate rpassword;
extern crate rprompt;

use std::env;
use std::io::Write;
use std::path::Path;

use rand::rngs::OsRng;
use rand::RngCore;

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

/// The first time the master password is entered, we save a randomized hash of it for the duration
/// of the process. This allows detecting errors when the password is later entered again without
/// keeping the password itself in memory.
struct MasterPasswordHash {
    salt: [u8; 32],
    hash: Option<[u8; 32]>,
}

impl MasterPasswordHash {
    pub fn new() -> Self {
        // Initialize random salt.
        let mut csprng = OsRng {};
        let mut salt: [u8; 32] = [0u8; 32];
        csprng.fill_bytes(&mut salt);
        MasterPasswordHash { salt, hash: None }
    }

    pub fn compute_hash(&self, password: &str) -> [u8; 32] {
        let mut msg: Vec<u8> = Vec::with_capacity(32 + password.len());
        msg.write_all(&self.salt).unwrap();
        msg.write_all(password.as_bytes()).unwrap();
        cybele_core::hash::sha256::hash(&msg)
    }
}

fn prompt_master_password(master_hash: &mut MasterPasswordHash) -> String {
    let password = rpassword::prompt_password("  > Please enter your master password: ").unwrap();
    let hash = master_hash.compute_hash(&password);
    match master_hash.hash {
        None => {
            let password2 = rpassword::prompt_password("  > Please enter your master password again: ").unwrap();
            if password != password2 {
                println!();
                println!("    *** The two passwords are not equal, let's try again... ***");
                println!();
                prompt_master_password(master_hash)
            } else {
                master_hash.hash = Some(hash);
                password
            }
        }
        Some(expected_hash) => {
            if hash != expected_hash {
                println!();
                println!("    *** The password is different from the last master password, let's try again... ***");
                println!();
                prompt_master_password(master_hash)
            } else {
                password
            }
        }
    }
}

fn create_password() -> String {
    let password = rpassword::prompt_password("  > Password: ").unwrap();
    let password2 = rpassword::prompt_password("  > Please enter password again: ").unwrap();
    if password != password2 {
        println!();
        println!("    *** The two passwords are not equal, let's try again... ***");
        println!();
        create_password()
    } else {
        password
    }
}

fn generate_password(password_len: usize) -> String {
    let password = cybele_core::password::generate_password(password_len);
    let regen = rprompt::prompt_reply(format!("  > Password = {}, do you want to generate a new one (y/n)? ", password)).unwrap();
    match regen.as_str() {
        "y" => generate_password(password_len),
        _ => password,
    }
}

fn prompt_color() -> file::Color {
    match rprompt::prompt_reply("  > Color (red|green|blue): ").unwrap().to_lowercase().as_str() {
        "red" => file::Color::Red,
        "green" => file::Color::Green,
        "blue" => file::Color::Blue,
        _ => {
            println!();
            println!("    *** Unknown color: valid choices are red, green or blue. ***");
            println!();
            prompt_color()
        }
    }
}

fn add_to_vault(vault: &mut Vault, name: &str, password: &str, master_password: &str) {
    match vault.add(name, password, master_password) {
        Some(_) => {
            println!("    + Password added for <{}>.", &name);
            println!("    + Don't forget to use the \"save\" command to save your changes.");
        }
        None => {
            println!("    *** Password could not be added for <{}> ***", &name);
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

    println!("       ______ ____    ____ .______    _______  __       _______ ");
    println!("      /      |\\   \\  /   / |   _  \\  |   ____||  |     |   ____|");
    println!("     |  ,----' \\   \\/   /  |  |_)  | |  |__   |  |     |  |__   ");
    println!("     |  |       \\_    _/   |   _  <  |   __|  |  |     |   __|  ");
    println!("     |  `----.    |  |     |  |_)  | |  |____ |  `----.|  |____ ");
    println!("      \\______|    |__|     |______/  |_______||_______||_______|");
    println!();
    println!();

    let args = match parse_startup_args() {
        Ok(startup_args) => startup_args,
        Err(()) => return,
    };

    let mut master_password_hash = MasterPasswordHash::new();
    let mut vault: Vault = if !Path::new(&args.vault_file).exists() {
        println!("Vault file does not exist: creating a new vault...");
        Vault::create(None)
    } else {
        println!("Loading encrypted vault...");
        let password = prompt_master_password(&mut master_password_hash);
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

    println!();
    println!("Enter \"help\" to list available commands.");
    println!();
    loop {
        let command = rprompt::prompt_reply("> ").unwrap();
        let cmd_args: Vec<&str> = command.split(' ').collect();
        match cmd_args[0] {
            "help" => help(),
            "add" => {
                let name = rprompt::prompt_reply("  > Name: ").unwrap();
                let hidden = rprompt::prompt_reply("  > Hide input (y/n): ").unwrap();
                let password = match hidden.as_str() {
                    "y" => create_password(),
                    _ => rprompt::prompt_reply("  > Password: ").unwrap(),
                };
                let master_password = prompt_master_password(&mut master_password_hash);
                add_to_vault(&mut vault, &name, &password, &master_password);
            }
            "generate" => {
                let name = rprompt::prompt_reply("  > Name: ").unwrap();
                let password_len: usize = rprompt::prompt_reply("  > Password length: ").unwrap().parse().unwrap();
                let password = generate_password(password_len);
                let master_password = prompt_master_password(&mut master_password_hash);
                add_to_vault(&mut vault, &name, &password, &master_password);
            }
            "remove" => {
                let name = rprompt::prompt_reply("  > Name: ").unwrap();
                vault.remove(name.as_str());
                println!("    + Password for <{}> removed.", name);
                println!("    + Don't forget to use the \"save\" command to save your changes.");
            }
            "get" => {
                // If an ID was provided in the arguments, we use it, otherwise we prompt the user.
                let mut items = vault.list();
                items.sort();
                let id = match cmd_args.get(1) {
                    Some(id) => id.parse::<usize>().unwrap(),
                    None => rprompt::prompt_reply("  > ID: ").unwrap().parse::<usize>().unwrap(),
                };
                if id < items.len() {
                    let name: &str = &items[id];
                    let master_password = prompt_master_password(&mut master_password_hash);
                    match vault.get(name, &master_password) {
                        Some(password) => {
                            println!("    - name: {}", name);
                            println!("    - password: {}", &password);
                        }
                        None => println!("    *** Could not find or decrypt password for <{}>. ***", name),
                    }
                } else {
                    println!("    *** Invalid ID <{}>. ***", id);
                }
            }
            "list" => {
                let filter = cmd_args.get(1);
                let mut items = vault.list();
                items.sort();
                items.iter().enumerate().for_each(|(pos, item)| {
                    let display = match filter {
                        Some(f) => item.to_lowercase().contains(&f.to_lowercase()),
                        None => true,
                    };
                    if display {
                        println!("  - {}: {}", pos, item)
                    }
                });
            }
            "save" => {
                let master_password = prompt_master_password(&mut master_password_hash);
                match args.vault_format {
                    VaultFormat::Plain => file::save_plain_vault(&vault, &args.vault_file, &master_password),
                    VaultFormat::Image => {
                        let color = prompt_color();
                        file::save_image_vault(&vault, color, &args.vault_file, &master_password);
                    }
                }
            }
            "exit" => break,
            _ => println!("  *** Unknown command: enter \"help\" to list available commands. ***"),
        }
    }
}
