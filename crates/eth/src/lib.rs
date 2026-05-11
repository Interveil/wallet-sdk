use bip32::{DerivationPath, XPrv};
use k256::ecdsa::SigningKey;
use std::str::FromStr;
use thiserror::Error;
use tiny_keccak::{Hasher, Keccak};

#[derive(Debug, Error)]
pub enum EthError {
    #[error("BIP-32 derivation failed")]
    DerivationFailed,
    #[error("Ethereum address generation failed")]
    AddressGenerationFailed,
}

/// Derives the raw secp256k1 private key bytes (BIP-44 path m/44'/60'/0'/0/0).
///
/// # Errors
///
/// Returns `EthError::DerivationFailed` if BIP-32 key derivation fails.
pub fn derive_eth_private_key(seed: &[u8]) -> Result<[u8; 32], EthError> {
    let path =
        DerivationPath::from_str("m/44'/60'/0'/0/0").map_err(|_| EthError::DerivationFailed)?;

    let child = XPrv::derive_from_path(seed, &path).map_err(|_| EthError::DerivationFailed)?;

    let signing_key: &SigningKey = child.private_key();
    let field_bytes = signing_key.to_bytes();
    let mut raw = [0u8; 32];
    raw.copy_from_slice(&field_bytes);
    Ok(raw)
}

/// Derives an Ethereum address from a BIP-39 seed (64 bytes).
///
/// Uses BIP-44 path: `m/44'/60'/0'/0/0`
///
/// # Errors
///
/// Returns `EthError::DerivationFailed` if the BIP-32 key derivation fails.
/// Returns `EthError::AddressGenerationFailed` if the address computation fails.
pub fn derive_eth_address(seed: &[u8]) -> Result<String, EthError> {
    let path =
        DerivationPath::from_str("m/44'/60'/0'/0/0").map_err(|_| EthError::DerivationFailed)?;

    let child = XPrv::derive_from_path(seed, &path).map_err(|_| EthError::DerivationFailed)?;

    let signing_key: &SigningKey = child.private_key();
    let verifying_key = signing_key.verifying_key();
    let encoded_point = verifying_key.to_encoded_point(false);
    let public_key_bytes = encoded_point.as_bytes();

    let mut keccak = Keccak::v256();
    keccak.update(&public_key_bytes[1..]);
    let mut hash = [0u8; 32];
    keccak.finalize(&mut hash);

    let address = &hash[12..];
    Ok(format!("0x{}", hex::encode(address)))
}
