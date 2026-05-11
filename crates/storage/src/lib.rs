use aes_gcm::aead::{Aead, KeyInit};
use aes_gcm::{Aes256Gcm, Nonce};
use argon2::Argon2;
use rand::RngCore;
use std::fs;
use std::path::Path;
use thiserror::Error;
use zeroize::Zeroizing;

const VERSION_LEGACY: u8 = 0x01;
const VERSION_MNEMONIC: u8 = 0x02;
const SALT_LEN: usize = 16;
const NONCE_LEN: usize = 12;
const HEADER_LEN: usize = 1 + SALT_LEN + NONCE_LEN;
const KEY_LEN: usize = 32;
const TAG_LEN: usize = 16;
const MIN_FILE_LEN: usize = HEADER_LEN + TAG_LEN;

#[cfg(test)]
const ARGON2_M_COST: u32 = 19456;
#[cfg(not(test))]
const ARGON2_M_COST: u32 = 65536;

const ARGON2_T_COST: u32 = 3;
const ARGON2_P_COST: u32 = 1;

#[derive(Debug, Error)]
pub enum StorageError {
    #[error("invalid password")]
    InvalidPassword,
    #[error("decryption failed")]
    DecryptionFailed,
    #[error("corrupted or tampered wallet file")]
    CorruptedFile,
    #[error("I/O error: {0}")]
    IoError(#[from] std::io::Error),
}

fn derive_key(password: &str, salt: &[u8]) -> Result<Zeroizing<[u8; KEY_LEN]>, StorageError> {
    let params = argon2::Params::new(ARGON2_M_COST, ARGON2_T_COST, ARGON2_P_COST, Some(KEY_LEN))
        .map_err(|_| StorageError::DecryptionFailed)?;

    let argon2 = Argon2::new(argon2::Algorithm::Argon2id, argon2::Version::V0x13, params);

    let mut key = Zeroizing::new([0u8; KEY_LEN]);
    argon2
        .hash_password_into(password.as_bytes(), salt, &mut *key)
        .map_err(|_| StorageError::DecryptionFailed)?;

    Ok(key)
}

/// Encrypts the wallet seed (and mnemonic, if available) with AES-256-GCM
/// and writes the vault to disk using the v0x02 format.
///
/// # Errors
///
/// Returns `StorageError::DecryptionFailed` if encryption or key
/// derivation fails. Returns `StorageError::IoError` for I/O failures.
pub fn save_wallet(
    path: impl AsRef<Path>,
    seed: &[u8],
    mnemonic: Option<&str>,
    password: &str,
) -> Result<(), StorageError> {

    // Encode: [seed_len:u32 LE][seed bytes][mnemonic_len:u32 LE][mnemonic bytes]
    let seed_len = u32::try_from(seed.len()).unwrap_or(u32::MAX);
    let mnemonic_str = mnemonic.unwrap_or("");
    let mnemonic_len = u32::try_from(mnemonic_str.len()).unwrap_or(u32::MAX);
    let mnemonic_bytes = mnemonic_str.as_bytes();

    let mut plaintext = Vec::with_capacity(4 + seed.len() + 4 + mnemonic_bytes.len());
    plaintext.extend_from_slice(&seed_len.to_le_bytes());
    plaintext.extend_from_slice(seed);
    plaintext.extend_from_slice(&mnemonic_len.to_le_bytes());
    plaintext.extend_from_slice(mnemonic_bytes);

    let mut salt = [0u8; SALT_LEN];
    let mut nonce = [0u8; NONCE_LEN];
    rand::rngs::OsRng.fill_bytes(&mut salt);
    rand::rngs::OsRng.fill_bytes(&mut nonce);

    let key = derive_key(password, &salt)?;

    let cipher =
        Aes256Gcm::new_from_slice(key.as_ref()).map_err(|_| StorageError::DecryptionFailed)?;
    #[allow(deprecated)]
    let nonce_ref = Nonce::from_slice(&nonce);
    let ciphertext = cipher
        .encrypt(nonce_ref, &*plaintext)
        .map_err(|_| StorageError::DecryptionFailed)?;

    let mut buf = Vec::with_capacity(HEADER_LEN + ciphertext.len());
    buf.push(VERSION_MNEMONIC);
    buf.extend_from_slice(&salt);
    buf.extend_from_slice(&nonce);
    buf.extend_from_slice(&ciphertext);
    fs::write(path.as_ref(), &buf)?;

    Ok(())
}

/// Decrypts a wallet vault file and returns the raw seed and optional mnemonic.
///
/// Supports both the legacy v0x01 (seed-only) and v0x02 (seed + mnemonic)
/// formats.
///
/// Returns `(seed_bytes, optional_mnemonic)`.
///
/// # Errors
///
/// Returns `StorageError::CorruptedFile` if the file format is invalid.
/// Returns `StorageError::DecryptionFailed` if the password is wrong or
/// the ciphertext has been tampered with.
/// Returns `StorageError::IoError` for I/O failures.
pub fn load_wallet(
    path: impl AsRef<Path>,
    password: &str,
) -> Result<(Vec<u8>, Option<String>), StorageError> {
    let data = fs::read(path.as_ref())?;

    if data.len() < MIN_FILE_LEN {
        return Err(StorageError::CorruptedFile);
    }
    match data[0] {
        VERSION_LEGACY | VERSION_MNEMONIC => {}
        _ => return Err(StorageError::CorruptedFile),
    }

    let salt = &data[1..=SALT_LEN];
    let nonce = &data[1 + SALT_LEN..HEADER_LEN];
    let ciphertext = &data[HEADER_LEN..];

    let key = derive_key(password, salt)?;

    let cipher =
        Aes256Gcm::new_from_slice(key.as_ref()).map_err(|_| StorageError::DecryptionFailed)?;
    #[allow(deprecated)]
    let nonce_ref = Nonce::from_slice(nonce);
    let plaintext = cipher
        .decrypt(nonce_ref, ciphertext)
        .map_err(|_| StorageError::DecryptionFailed)?;

    if data[0] == VERSION_LEGACY {
        // v0x01: the entire plaintext is the seed, no mnemonic
        return Ok((plaintext, None));
    }

    // v0x02: [seed_len:u32 LE][seed][mnemonic_len:u32 LE][mnemonic]
    if plaintext.len() < 8 {
        return Err(StorageError::CorruptedFile);
    }
    let seed_len = u32::from_le_bytes(
        plaintext[..4].try_into().map_err(|_| StorageError::CorruptedFile)?,
    ) as usize;
    let mnemonic_len = u32::from_le_bytes(
        plaintext[4 + seed_len..4 + seed_len + 4]
            .try_into()
            .map_err(|_| StorageError::CorruptedFile)?,
    ) as usize;

    if 4 + seed_len + 4 + mnemonic_len != plaintext.len() {
        return Err(StorageError::CorruptedFile);
    }

    let seed = plaintext[4..4 + seed_len].to_vec();
    let mnemonic = if mnemonic_len > 0 {
        Some(
            String::from_utf8(plaintext[4 + seed_len + 4..].to_vec())
                .map_err(|_| StorageError::CorruptedFile)?,
        )
    } else {
        None
    };

    Ok((seed, mnemonic))
}
