extern crate rpassword;
extern crate rprompt;

use std::collections::HashMap;
use std::fs::OpenOptions;
use std::io::Write;
use std::path::Path;
use std::{env, fs};

struct StartupArgs {
    password_file: String,
}

fn parse_startup_args() -> Result<StartupArgs, ()> {
    if env::args().len() < 2 {
        println!("Cybele-cli requires the path to an encrypted password file: `cybele-cli <path-to-password-file>`.");
        return Err(());
    }
    let password_file = env::args().nth(1).unwrap();
    if !Path::new(&password_file).exists() {
        println!("Password file does not exist: creating it...");
        OpenOptions::new().read(true).write(true).create(true).open(&password_file).unwrap();
    }
    Ok(StartupArgs { password_file })
}

fn load_password_file(filename: &str) -> Result<HashMap<String, String>, ()> {
    let passwords_str = match fs::read_to_string(&filename) {
        Ok(s) => s,
        Err(_) => {
            println!("Could not load password file.");
            return Err(());
        }
    };
    let password_pairs: Vec<&str> = passwords_str.lines().collect();
    let mut passwords: HashMap<String, String> = HashMap::new();
    for p in password_pairs {
        let parts: Vec<&str> = p.split(':').collect();
        if parts.len() != 2 {
            println!("Invalid password file: wrong format.");
            return Err(());
        }
        passwords.insert(parts[0].to_string(), parts[1].to_string());
    }
    Ok(passwords)
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

    println!("Loading encrypted password file...");
    let mut passwords = match load_password_file(&args.password_file) {
        Ok(p) => p,
        Err(()) => return,
    };

    println!("Password file successfully loaded.");
    println!("Enter \"help\" to list available commands.");
    println!();
    loop {
        let command = rprompt::prompt_reply_stdout("> ").unwrap();
        match command.as_str() {
            "help" => help(),
            "add" => {
                let name = rprompt::prompt_reply_stdout("Name: ").unwrap();
                if name.contains(':') {
                    println!("Name cannot contain `:`");
                } else {
                    let hidden = rprompt::prompt_reply_stdout("Hide input (y/n): ").unwrap();
                    let password = match hidden.as_str() {
                        "y" => rpassword::prompt_password("Password: ").unwrap(),
                        _ => rprompt::prompt_reply_stdout("Password: ").unwrap(),
                    };
                    let master_password = rpassword::prompt_password("Master password: ").unwrap();
                    println!("Your master password is {}", master_password);
                    // TODO: encrypt with master password.
                    passwords.insert(name.clone(), password);
                    println!("Password added for <{}>", name.clone());
                    println!("Don't forget to use the `save` command to save your changes.");
                }
            }
            "remove" => {
                let name = rprompt::prompt_reply_stdout("Name: ").unwrap();
                passwords.remove(name.as_str());
                println!("Password for <{}> removed", name);
                println!("Don't forget to use the `save` command to save your changes.");
            }
            "get" => {
                let name = rprompt::prompt_reply_stdout("Name: ").unwrap();
                let password = rpassword::prompt_password("Master password: ").unwrap();
                // TODO: decrypt with master password.
                println!("Your master password is {} and name is {}", password, name);
            }
            "list" => passwords.keys().for_each(|p| println!("{}", p)),
            "save" => {
                let mut password_file = OpenOptions::new().write(true).truncate(true).open(&args.password_file).unwrap();
                passwords.iter().for_each(|(n, p)| writeln!(password_file, "{}:{}", n, p).unwrap());
                password_file.sync_all().unwrap();
                println!("Password file successfully saved.");
            }
            "exit" => break,
            _ => println!("Unknown command: enter \"help\" to list available commands."),
        }
    }
}
