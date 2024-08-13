extern crate cybele_core;

use std::fs;
use std::fs::OpenOptions;
use std::io::Write;
use std::path::Path;

use cybele_core::vault::Vault;
use rand::rngs::OsRng;
use rand::RngCore;

/// Load vault from a binary file.
pub fn load_plain_vault(filename: &str, master_password: &str) -> Result<Vault, ()> {
    load_file(filename).and_then(|serialized_vault| deserialize_vault(&serialized_vault, master_password))
}

fn load_file(filename: &str) -> Result<Vec<u8>, ()> {
    fs::read(filename).map_err(|_| println!("Could not read vault file."))
}

fn deserialize_vault(serialized_vault: &[u8], master_password: &str) -> Result<Vault, ()> {
    Vault::deserialize(serialized_vault, master_password).ok_or_else(|| println!("Could not decrypt vault file."))
}

/// Save vault to a binary file.
pub fn save_plain_vault(vault: &Vault, filename: &str, master_password: &str) {
    match vault.serialize(master_password) {
        Some(serialized) => save_vault_file(filename, &serialized),
        None => println!("Could not encrypt vault."),
    };
}

fn save_vault_file(filename: &str, data: &[u8]) {
    if !Path::new(filename).exists() {
        OpenOptions::new().read(true).write(true).create(true).truncate(true).open(filename).unwrap();
    }
    let mut vault_file = OpenOptions::new().write(true).truncate(true).open(filename).unwrap();
    vault_file.write_all(data).unwrap();
    vault_file.sync_all().unwrap();
    println!("  Vault successfully saved.");
}

/// Load vault embedded in a bitmap image.
pub fn load_image_vault(filename: &str, master_password: &str) -> Result<Vault, ()> {
    load_file(filename).and_then(|file_bytes| decrypt_image_vault(&file_bytes, master_password))
}

fn decrypt_image_vault(encoded: &[u8], master_password: &str) -> Result<Vault, ()> {
    let decoded = decode_image_vault(encoded);
    deserialize_vault(&decoded, master_password)
}

fn decode_image_vault(encoded: &[u8]) -> Vec<u8> {
    // The bitmap headers use 26 bytes that we don't need to read.
    let pixels = &encoded[26..];
    // We only keep the 6 low bits of each pixel bytes: we use 4 bytes to encode 3 bytes of data.
    let mut data: Vec<u8> = Vec::with_capacity(pixels.len() * 3 / 4);
    for i in 0..pixels.len() / 4 {
        data.push((pixels[4 * i] << 2) | ((pixels[4 * i + 1] >> 4) & 0x03));
        data.push((pixels[4 * i + 1] << 4) | ((pixels[4 * i + 2] >> 2) & 0x0f));
        data.push((pixels[4 * i + 2] << 6) | (pixels[4 * i + 3] & 0x3f));
    }
    // The first four bytes encode the vault's size, xor-ed with a magic value to look more random.
    let vault_size = ((((data[0] ^ 0x5c) as u32) << 24) + (((data[1] ^ 0xa1) as u32) << 16) + (((data[2] ^ 0x37) as u32) << 8) + ((data[3] ^ 0xd2) as u32)) as usize;
    // We drop the size bytes.
    for _i in 0..4 {
        data.remove(0);
    }
    // We drop padding data.
    data.resize(vault_size, 0u8);
    data
}

pub enum Color {
    Red,
    Green,
    Blue,
}

/// Save vault as a bitmap image using the dominant color provided.
pub fn save_image_vault(vault: &Vault, color: Color, filename: &str, master_password: &str) {
    match encrypt_image_vault(vault, color, master_password) {
        Some(serialized) => save_vault_file(filename, &serialized),
        None => println!("Could not encrypt vault."),
    };
}

fn encrypt_image_vault(vault: &Vault, color: Color, master_password: &str) -> Option<Vec<u8>> {
    vault.serialize(master_password).map(|serialized| encode_image_vault(&serialized, color))
}

fn encode_image_vault(serialized: &[u8], color: Color) -> Vec<u8> {
    // We encode data in the 6 low bits of every color byte of each pixel, which means that each group of 4 pixels can encode 9 bytes of data.
    // We check how many groups of 4 pixels we'll need to encode the vault data and its size (which is itself encoded using 4 bytes).
    let pixel_min_count: usize = (serialized.len() + 4) * 4 / 9;
    // We round it up to use standard image sizes.
    let (width, height): (usize, usize) = match pixel_min_count {
        count if count < 360 * 360 => (360, 360),
        count if count < 720 * 720 => (720, 720),
        count if count < 1080 * 1080 => (1080, 1080),
        count if count < 2048 * 2048 => (2048, 2048),
        count if count < 4096 * 4096 => (4096, 4096),
        count if count < 8192 * 8192 => (8192, 8192),
        _ => panic!("Vault size is too big: cannot encode in image"),
    };
    // We add the size header and pad vault data with random bytes.
    let mut data: Vec<u8> = Vec::with_capacity(width * height * 9 / 4);
    data.push(((serialized.len() >> 24) as u8) ^ 0x5c);
    data.push(((serialized.len() >> 16) as u8) ^ 0xa1);
    data.push(((serialized.len() >> 8) as u8) ^ 0x37);
    data.push((serialized.len() as u8) ^ 0xd2);
    for &b in serialized {
        data.push(b);
    }
    let mut csprng = OsRng {};
    data.resize(width * height * 9 / 4, 0u8);
    csprng.fill_bytes(&mut data[serialized.len() + 4..]);
    // Bitmap files start with a bitmap header and a dib header.
    let headers_size = 14 + 12;
    let file_size = headers_size + width * height * 3;
    let mut file_bytes: Vec<u8> = Vec::with_capacity(file_size);
    // The first 2 bytes of the bitmap header are "BM" in ascii.
    file_bytes.push(0x42);
    file_bytes.push(0x4d);
    // The next 4 bytes encode the total file size in bytes (little endian).
    file_bytes.push(file_size as u8);
    file_bytes.push((file_size >> 8) as u8);
    file_bytes.push((file_size >> 16) as u8);
    file_bytes.push((file_size >> 24) as u8);
    // The next 4 bytes are reserved.
    file_bytes.push(0x00);
    file_bytes.push(0x00);
    file_bytes.push(0x00);
    file_bytes.push(0x00);
    // The last 4 bytes contain the offset of the start of the pixel array (14 + 12 = 26).
    file_bytes.push(0x1a);
    file_bytes.push(0x00);
    file_bytes.push(0x00);
    file_bytes.push(0x00);
    // The first 4 bytes of the dib header contain the size of this header (little endian).
    file_bytes.push(0x0c);
    file_bytes.push(0x00);
    file_bytes.push(0x00);
    file_bytes.push(0x00);
    // The next 2 bytes contain the image width (little endian).
    file_bytes.push(width as u8);
    file_bytes.push((width >> 8) as u8);
    // The next 2 bytes contain the image height (little endian).
    file_bytes.push(height as u8);
    file_bytes.push((height >> 8) as u8);
    // The next 2 bytes are reserved.
    file_bytes.push(0x01);
    file_bytes.push(0x00);
    // The last 2 bytes contain the pixel size (24 bits: 1 byte per color).
    file_bytes.push(0x18);
    file_bytes.push(0x00);
    // We fill pixel bytes by iterating on every group of 4 bytes and setting:
    //  - the 6 lower bits using vault data
    //  - the 2 higher bits using the color chosen
    for i in 0..width * height * 3 / 4 {
        // Write vault data first.
        let chunk = [
            (data[3 * i] >> 2) & 0x3f,
            ((data[3 * i] << 4) & 0x30) | (data[3 * i + 1] >> 4 & 0x0f),
            ((data[3 * i + 1] << 2) & 0x3c) | (data[3 * i + 2] >> 6 & 0x03),
            data[3 * i + 2] & 0x3f,
        ];
        // Then write the color bits: a pixel contains its blue byte first, then green, then red.
        for (j, &b) in chunk.iter().enumerate() {
            match (4 * i + j) % 3 {
                0 => match color {
                    Color::Blue => file_bytes.push(b | 0xc0),
                    _ => file_bytes.push(b & 0x3f),
                },
                1 => match color {
                    Color::Green => file_bytes.push(b | 0xc0),
                    _ => file_bytes.push(b & 0x3f),
                },
                _ => match color {
                    Color::Red => file_bytes.push(b | 0xc0),
                    _ => file_bytes.push(b & 0x3f),
                },
            }
        }
    }
    file_bytes
}

#[cfg(test)]
mod tests {
    use cybele_core::vault::Vault;
    use cybele_core::Version;

    use crate::file;
    use crate::file::Color;

    #[test]
    fn encrypt_decrypt_image_vault() {
        let mut vault = Vault::create(None);
        vault.version = Version::Test;
        vault.add("item #1", "data #1", "password");
        vault.add("item #2", "data #2", "password");
        let encoded = file::encrypt_image_vault(&vault, Color::Red, "password").unwrap();
        let decoded = file::decrypt_image_vault(&encoded, "password").unwrap();
        assert_eq!(decoded.version, Version::Test);
        assert_eq!(decoded.list().len(), 2);
        assert_eq!(decoded.get("item #1", "password").unwrap(), "data #1");
        assert_eq!(decoded.get("item #2", "password").unwrap(), "data #2");
    }

    #[test]
    fn encode_decode_image_vault() {
        // We test the boundaries around the 360*360 image size.
        let data1 = vec![42u8; 291595];
        let encoded1 = file::encode_image_vault(&data1, Color::Blue);
        assert_eq!(encoded1.len(), 360 * 360 * 3 + 26);
        let decoded1 = file::decode_image_vault(&encoded1);
        assert_eq!(data1, decoded1);
        let data2 = vec![42u8; 291596];
        let encoded2 = file::encode_image_vault(&data2, Color::Green);
        assert_eq!(encoded2.len(), 720 * 720 * 3 + 26);
        let decoded2 = file::decode_image_vault(&encoded2);
        assert_eq!(data2, decoded2);
    }
}
