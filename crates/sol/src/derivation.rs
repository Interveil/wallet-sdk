//! SLIP-0010 Ed25519 hardened child key derivation.
//!
//! References:
//! - SLIP-0010: <https://github.com/satoshilabs/slips/blob/master/slip-0010.md>
//! - BIP-44:    <https://github.com/bitcoin/bips/blob/master/bip-0044.mediawiki>
//! - Ed25519:   RFC 8032
//!
//! # Design notes
//!
//! Ed25519 does not support non-hardened derivation because its key schedule
//! does not expose public key point addition (unlike secp256k1). All BIP-44
//! indices for Ed25519 MUST use hardened (`'`) derivation.
//!
//! Unlike secp256k1 BIP-32, SLIP-0010 Ed25519 child keys are **not** added
//! to the parent key. The HMAC output directly becomes the child key seed.

use hmac::{Hmac, Mac};
use sha2::Sha512;
use thiserror::Error;

type HmacSha512 = Hmac<Sha512>;

/// Domain separator used by SLIP-0010 for Ed25519 master key generation.
const ED25519_SEED_KEY: &[u8] = b"ed25519 seed";

#[derive(Debug, Error)]
pub(crate) enum DerivationError {
    #[error("HMAC initialization failed")]
    HmacInitFailed,
    #[error("seed must be 64 bytes, got {0}")]
    InvalidSeedLength(usize),
}

/// Derives a 32-byte Ed25519 private key seed at the given BIP-44 path.
///
/// Implements SLIP-0010 hardened-only Ed25519 derivation.
///
/// # Arguments
///
/// * `seed` - BIP-39 seed (must be exactly 64 bytes)
/// * `path` - BIP-44 path indices (e.g. `&[44, 501, 0, 0]` for `m/44'/501'/0'/0'`)
///
/// # Returns
///
/// A 32-byte Ed25519 private key seed that can be used with `SigningKey::from_bytes()`.
///
/// # Errors
///
/// Returns `DerivationError::InvalidSeedLength` if `seed` is not 64 bytes.
/// Returns `DerivationError::HmacInitFailed` if HMAC-SHA512 cannot be initialized.
pub(crate) fn derive_key(seed: &[u8], path: &[u32]) -> Result<[u8; 32], DerivationError> {
    if seed.len() != 64 {
        return Err(DerivationError::InvalidSeedLength(seed.len()));
    }

    let (key, chain_code) = master_key(seed)?;

    let (mut key, mut chain_code) = (key, chain_code);
    for &index in path {
        let next = child_key(&key, &chain_code, index)?;
        key = next.0;
        chain_code = next.1;
    }

    Ok(key)
}

/// SLIP-0010 master key generation.
///
/// I = HMAC-SHA512(key = "ed25519 seed", data = seed)
/// IL = I[0..32]   → master private key seed
/// IR = I[32..64]  → master chain code
///
/// # Note
///
/// `master_key` does NOT validate seed length — it is a private function
/// called exclusively from [`derive_key`], which performs the 64-byte
/// seed length check. Callers MUST ensure the seed is 64 bytes.
fn master_key(seed: &[u8]) -> Result<([u8; 32], [u8; 32]), DerivationError> {
    let mut mac = HmacSha512::new_from_slice(ED25519_SEED_KEY)
        .map_err(|_| DerivationError::HmacInitFailed)?;
    mac.update(seed);
    let result = mac.finalize().into_bytes();

    let mut key = [0u8; 32];
    let mut chain_code = [0u8; 32];
    key.copy_from_slice(&result[..32]);
    chain_code.copy_from_slice(&result[32..]);

    Ok((key, chain_code))
}

/// SLIP-0010 hardened child key derivation for Ed25519.
///
/// Derives a hardened child at `index`:
///
/// I = HMAC-SHA512(
///     key  = `parent_chain_code`,
///     data = 0x00 || `parent_key_seed` || `ser32(i')`
/// )
///
/// where `ser32(i') = (0x80000000 | index)` encoded as 4-byte big-endian.
///
/// IL = I[0..32]   → child private key seed
/// IR = I[32..64]  → child chain code
///
/// Ed25519 does not support non-hardened derivation; the `index` is always
/// hardened with the 0x80000000 bit set.
fn child_key(
    key: &[u8; 32],
    chain_code: &[u8; 32],
    index: u32,
) -> Result<([u8; 32], [u8; 32]), DerivationError> {
    let mut mac =
        HmacSha512::new_from_slice(chain_code).map_err(|_| DerivationError::HmacInitFailed)?;

    mac.update(&[0x00]);
    mac.update(key);
    mac.update(&(0x8000_0000 | index).to_be_bytes());

    let result = mac.finalize().into_bytes();

    let mut child_key = [0u8; 32];
    let mut child_chain_code = [0u8; 32];
    child_key.copy_from_slice(&result[..32]);
    child_chain_code.copy_from_slice(&result[32..]);

    Ok((child_key, child_chain_code))
}

#[cfg(test)]
mod tests {
    use super::{child_key, derive_key, master_key, DerivationError};

    /// SLIP-0010 test vector 1 (Ed25519, master key).
    /// seed = 00010203...0f (16 bytes).
    #[test]
    fn test_slip10_vector1_master() {
        let seed: [u8; 16] = [
            0x00, 0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x08, 0x09, 0x0a, 0x0b, 0x0c, 0x0d,
            0x0e, 0x0f,
        ];

        let (master_key, _) = master_key(&seed).unwrap();
        let expected: [u8; 32] = [
            0x2b, 0x4b, 0xe7, 0xf1, 0x9e, 0xe2, 0x7b, 0xbf, 0x30, 0xc6, 0x67, 0xb6, 0x42, 0xd5,
            0xf4, 0xaa, 0x69, 0xfd, 0x16, 0x98, 0x72, 0xf8, 0xfc, 0x30, 0x59, 0xc0, 0x8e, 0xba,
            0xe2, 0xeb, 0x19, 0xe7,
        ];
        assert_eq!(master_key, expected);
    }

    #[test]
    fn test_derive_key_empty_path() {
        let seed = [0u8; 64];
        let result = derive_key(&seed, &[]).unwrap();
        assert_eq!(result.len(), 32);
    }

    #[test]
    fn test_derive_key_invalid_seed_length() {
        let result = derive_key(&[0u8; 32], &[44]);
        assert!(matches!(
            result,
            Err(DerivationError::InvalidSeedLength(32))
        ));
    }

    #[test]
    fn test_child_key_hardened_bit_set() {
        let key = [0u8; 32];
        let chain_code = [0u8; 32];
        // index 0 should use 0x80000000
        let result = child_key(&key, &chain_code, 0).unwrap();
        assert_eq!(result.0.len(), 32);
        assert_eq!(result.1.len(), 32);
    }
}
