use aes_gcm::aead::{Aead, KeyInit};
use aes_gcm::{Aes256Gcm, Nonce};
use argon2::Argon2;
use rand::RngCore;
use std::fs;
use std::path::Path;
use thiserror::Error;
use wallet_core::Wallet;
use zeroize::Zeroizing;

const VERSION: u8 = 0x01;
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

/// Encrypts the wallet seed with AES-256-GCM (Argon2id key derivation)
/// and writes the vault to disk.
///
/// # Errors
///
/// Returns `StorageError::DecryptionFailed` if encryption or key
/// derivation fails. Returns `StorageError::IoError` for I/O failures.
pub fn save_wallet(
    path: impl AsRef<Path>,
    wallet: &Wallet,
    password: &str,
) -> Result<(), StorageError> {
    let seed = wallet.seed();

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
        .encrypt(nonce_ref, seed)
        .map_err(|_| StorageError::DecryptionFailed)?;

    let mut buf = Vec::with_capacity(HEADER_LEN + ciphertext.len());
    buf.push(VERSION);
    buf.extend_from_slice(&salt);
    buf.extend_from_slice(&nonce);
    buf.extend_from_slice(&ciphertext);
    fs::write(path.as_ref(), &buf)?;

    Ok(())
}

/// Decrypts a wallet vault file and reconstructs the wallet from its seed.
///
/// # Errors
///
/// Returns `StorageError::CorruptedFile` if the file format is invalid.
/// Returns `StorageError::DecryptionFailed` if the password is wrong or
/// the ciphertext has been tampered with.
/// Returns `StorageError::IoError` for I/O failures.
pub fn load_wallet(path: impl AsRef<Path>, password: &str) -> Result<Wallet, StorageError> {
    let data = fs::read(path.as_ref())?;

    if data.len() < MIN_FILE_LEN {
        return Err(StorageError::CorruptedFile);
    }
    if data[0] != VERSION {
        return Err(StorageError::CorruptedFile);
    }

    let salt = &data[1..=SALT_LEN];
    let nonce = &data[1 + SALT_LEN..HEADER_LEN];
    let ciphertext = &data[HEADER_LEN..];

    let key = derive_key(password, salt)?;

    let cipher =
        Aes256Gcm::new_from_slice(key.as_ref()).map_err(|_| StorageError::DecryptionFailed)?;
    #[allow(deprecated)]
    let nonce_ref = Nonce::from_slice(nonce);
    let seed = cipher
        .decrypt(nonce_ref, ciphertext)
        .map_err(|_| StorageError::DecryptionFailed)?;

    Ok(Wallet::from_seed(seed))
}
