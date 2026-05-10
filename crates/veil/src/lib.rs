mod keys;

use bech32::{Bech32, Hrp, encode_lower};
use ed25519_dalek::SigningKey;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum VeilError {
    #[error("key derivation failed")]
    DerivationFailed,
    #[error("address encoding failed")]
    AddressEncodingFailed,
}

/// Derives the spending key at index 0.
///
/// `sk_0 = H(seed || 0)` using Blake3.
pub fn derive_spending_key(seed: &[u8]) -> Result<[u8; 32], VeilError> {
    Ok(keys::derive_key(seed, 0))
}

/// Derives the viewing key at index 1.
///
/// `sk_1 = H(seed || 1)` using Blake3.
pub fn derive_viewing_key(seed: &[u8]) -> Result<[u8; 32], VeilError> {
    Ok(keys::derive_key(seed, 1))
}

/// Derives the nullifier key at index 2.
///
/// `sk_2 = H(seed || 2)` using Blake3.
pub fn derive_nullifier_key(seed: &[u8]) -> Result<[u8; 32], VeilError> {
    Ok(keys::derive_key(seed, 2))
}

/// Derives a deterministic privacy address from a BIP-39 seed.
///
/// The address encodes the Ed25519 public key derived from the spending key,
/// formatted as a Bech32 string with the `pvx` prefix.
///
/// Format: `pvx1...`
pub fn derive_private_address(seed: &[u8]) -> Result<String, VeilError> {
    let spending_key = keys::derive_key(seed, 0);

    let signing_key = SigningKey::from_bytes(&spending_key);
    let public_key = signing_key.verifying_key().to_bytes();

    let hrp = Hrp::parse("pvx").map_err(|_| VeilError::AddressEncodingFailed)?;
    let encoded: String =
        encode_lower::<Bech32>(hrp, &public_key).map_err(|_| VeilError::AddressEncodingFailed)?;

    Ok(encoded)
}
