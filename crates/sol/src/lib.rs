mod derivation;

use derivation::derive_key;
use ed25519_dalek::SigningKey;
use thiserror::Error;

/// BIP-44 path indices for Solana: `m/44'/501'/0'/0'`
const SOLANA_PATH: &[u32] = &[44, 501, 0, 0];

#[derive(Debug, Error)]
pub enum SolError {
    #[error("BIP-32 / SLIP-0010 key derivation failed")]
    DerivationFailed,
    #[error("Ed25519 key generation failed")]
    KeyGenerationFailed,
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
