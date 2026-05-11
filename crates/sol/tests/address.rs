use std::str::FromStr;
use bip39::{Language, Mnemonic};
use sol::{derive_sol_address, sign_sol};

/// BIP-39 standard test vector mnemonic (all-zeros entropy).
const GOLDEN_MNEMONIC: &str =
    "abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon about";

/// Golden test vector — changing this indicates a breaking derivation change.
///
/// Independently verified against:
/// - solders (Python) Keypair::from_seed() with SLIP-0010 Ed25519 path m/44'/501'/0'/0'
const GOLDEN_ADDRESS: &str = "HAgk14JpMQLgt6rVgv7cBQFJWFto5Dqxi472uT3DKpqk";

fn seed_from_mnemonic(phrase: &str) -> Vec<u8> {
    Mnemonic::from_str(phrase).unwrap().to_seed("").to_vec()
}

#[test]
fn test_golden_vector() {
    let seed = seed_from_mnemonic(GOLDEN_MNEMONIC);
    let address = derive_sol_address(&seed).unwrap();
    assert_eq!(address, GOLDEN_ADDRESS);
}

#[test]
fn test_deterministic() {
    let seed = seed_from_mnemonic(GOLDEN_MNEMONIC);
    let address1 = derive_sol_address(&seed).unwrap();
    let address2 = derive_sol_address(&seed).unwrap();
    assert_eq!(address1, address2);
}

#[test]
fn test_different_seed_different_address() {
    let seed_a = seed_from_mnemonic(GOLDEN_MNEMONIC);
    let seed_b = seed_from_mnemonic(
        "legal winner thank year wave sausage worth useful legal winner thank yellow",
    );
    let address_a = derive_sol_address(&seed_a).unwrap();
    let address_b = derive_sol_address(&seed_b).unwrap();
    assert_ne!(address_a, address_b);
}

#[test]
fn test_valid_base58() {
    let seed = Mnemonic::generate_in(Language::English, 12).unwrap().to_seed("");
    let address = derive_sol_address(&seed).unwrap();
    let decoded = bs58::decode(&address).into_vec();
    assert!(decoded.is_ok(), "address must be valid base58");
    assert_eq!(decoded.unwrap().len(), 32);
}

#[test]
fn test_address_length() {
    let seed = Mnemonic::generate_in(Language::English, 12).unwrap().to_seed("");
    let address = derive_sol_address(&seed).unwrap();
    assert!(
        address.len() >= 32 && address.len() <= 44,
        "Solana address length {} out of expected range 32-44",
        address.len()
    );
}

#[test]
fn test_sign_sol_roundtrip() {
    let seed = seed_from_mnemonic(GOLDEN_MNEMONIC);
    let key = sol::derive_sol_private_key(&seed).unwrap();
    let hash = [0xabu8; 32];
    let sig = sign_sol(&key, &hash).unwrap();
    assert_eq!(sig.len(), 64, "Ed25519 signature must be 64 bytes");
}

#[test]
fn test_sign_sol_deterministic() {
    let seed = seed_from_mnemonic(GOLDEN_MNEMONIC);
    let key = sol::derive_sol_private_key(&seed).unwrap();
    let hash = [0xabu8; 32];
    let sig_a = sign_sol(&key, &hash).unwrap();
    let sig_b = sign_sol(&key, &hash).unwrap();
    assert_eq!(sig_a, sig_b, "sign_sol must be deterministic");
}

#[test]
fn test_sign_sol_verify() {
    use ed25519_dalek::{Signature, Verifier};
    let seed = seed_from_mnemonic(GOLDEN_MNEMONIC);
    let key = sol::derive_sol_private_key(&seed).unwrap();
    let hash = [0xabu8; 32];
    let sig_bytes = sign_sol(&key, &hash).unwrap();
    let signature = Signature::from_slice(&sig_bytes).unwrap();
    let signing_key = ed25519_dalek::SigningKey::from_bytes(&key);
    let verifying_key = signing_key.verifying_key();
    assert!(
        verifying_key.verify(&hash, &signature).is_ok(),
        "signature must verify"
    );
}
