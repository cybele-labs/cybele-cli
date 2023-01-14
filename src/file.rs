extern crate cybele_core;

use std::fs;
use std::fs::OpenOptions;
use std::io::Write;
use std::path::Path;

use cybele_core::vault::Vault;

/// Load vault from a binary file.
pub fn load_plain_vault(filename: &str, password: &str) -> Result<Vault, ()> {
    let serialized_vault = match fs::read(filename) {
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

/// Save vault to a binary file.
pub fn save_plain_vault(vault: &Vault, filename: &str, master_password: &str) {
    match vault.serialize(master_password) {
        Some(serialized) => {
            if !Path::new(filename).exists() {
                OpenOptions::new().read(true).write(true).create(true).open(filename).unwrap();
            }
            let mut vault_file = OpenOptions::new().write(true).truncate(true).open(filename).unwrap();
            vault_file.write_all(&serialized).unwrap();
            vault_file.sync_all().unwrap();
            println!("Vault successfully saved.");
        }
        None => println!("Could not encrypt vault."),
    };
}
