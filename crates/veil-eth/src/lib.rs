use bip32::{DerivationPath, XPrv};
use k256::ecdsa::signature::hazmat::PrehashSigner;
use k256::ecdsa::{RecoveryId, Signature, SigningKey, VerifyingKey};
use std::str::FromStr;
use thiserror::Error;
use tiny_keccak::{Hasher, Keccak};

#[derive(Debug, Error)]
pub enum EthError {
    #[error("BIP-32 derivation failed")]
    DerivationFailed,
    #[error("Ethereum address generation failed")]
    AddressGenerationFailed,
    #[error("signing failed")]
    SigningFailed,
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

/// Signs a 32-byte SHA-256 hash with a secp256k1 private key (ECDSA, RFC 6979).
///
/// Uses deterministic `sign_prehash` — no randomness, no extra hashing.
/// Returns the 65-byte `(r || s || v)` signature with Ethereum recovery ID
/// (`v = 27 + recid`, where `recid` is 0 or 1).
///
/// # Errors
///
/// Returns `EthError::SigningFailed` if key construction or signing fails.
pub fn sign_eth(key: &[u8; 32], hash: &[u8; 32]) -> Result<[u8; 65], EthError> {
    let signing_key =
        SigningKey::from_slice(key.as_ref()).map_err(|_| EthError::SigningFailed)?;
    let signature: Signature = signing_key
        .sign_prehash(hash.as_ref())
        .map_err(|_| EthError::SigningFailed)?;

    let verifying_key = signing_key.verifying_key();

    let recid_0 = RecoveryId::new(false, false);
    let recid_1 = RecoveryId::new(true, false);

    let recid = if VerifyingKey::recover_from_prehash(hash.as_ref(), &signature, recid_0)
        .is_ok_and(|vk| vk == *verifying_key)
    {
        recid_0
    } else {
        recid_1
    };

    let mut sig_bytes = [0u8; 65];
    sig_bytes[..64].copy_from_slice(&signature.to_bytes());
    sig_bytes[64] = 27 + recid.to_byte();
    Ok(sig_bytes)
}
