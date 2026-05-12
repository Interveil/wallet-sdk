mod derivation;

use derivation::derive_key;
use ed25519_dalek::{Signer, SigningKey};
use thiserror::Error;

/// BIP-44 path indices for Solana: `m/44'/501'/0'/0'`
const SOLANA_PATH: &[u32] = &[44, 501, 0, 0];

#[derive(Debug, Error)]
pub enum SolError {
    #[error("BIP-32 / SLIP-0010 key derivation failed")]
    DerivationFailed,
    #[error("Ed25519 key generation failed")]
    KeyGenerationFailed,
    #[error("signing failed")]
    SigningFailed,
}

/// Derives the raw Ed25519 private key bytes (SLIP-0010 path m/44'/501'/0'/0').
///
/// # Errors
///
/// Returns `SolError::DerivationFailed` if SLIP-0010 key derivation fails.
pub fn derive_sol_private_key(seed: &[u8]) -> Result<[u8; 32], SolError> {
    derive_key(seed, SOLANA_PATH).map_err(|_| SolError::DerivationFailed)
}

/// Derives a Solana address from a BIP-39 seed (64 bytes).
///
/// Uses BIP-44 path: `m/44'/501'/0'/0'`
/// Implements SLIP-0010 hardened-only Ed25519 derivation.
///
/// # Arguments
///
/// * `seed` — BIP-39 seed (64 bytes), typically from `wallet_core::Wallet::seed()`
///
/// # Returns
///
/// Base58-encoded Solana public key (typically 44 characters).
///
/// # Errors
///
/// Returns `SolError::DerivationFailed` if SLIP-0010 derivation fails.
/// Returns `SolError::KeyGenerationFailed` if Ed25519 key creation fails.
pub fn derive_sol_address(seed: &[u8]) -> Result<String, SolError> {
    let key_bytes = derive_key(seed, SOLANA_PATH).map_err(|_| SolError::DerivationFailed)?;

    let signing_key = SigningKey::from_bytes(&key_bytes);

    let public_key = signing_key.verifying_key().to_bytes();

    Ok(bs58::encode(public_key).into_string())
}

/// Signs a 32-byte SHA-256 hash with an Ed25519 private key.
///
/// Returns the 64-byte raw Ed25519 signature.
///
/// # Errors
///
/// Returns `SolError::KeyGenerationFailed` if the key bytes are invalid.
/// Returns `SolError::SigningFailed` if the signing operation fails.
pub fn sign_sol(key: &[u8; 32], hash: &[u8; 32]) -> Result<[u8; 64], SolError> {
    let signing_key =
        SigningKey::from_bytes(key);
    let signature = signing_key.sign(hash.as_ref());
    Ok(signature.to_bytes())
}
