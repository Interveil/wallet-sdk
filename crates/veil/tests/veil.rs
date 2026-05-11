use std::str::FromStr;
use bip39::{Language, Mnemonic};
use veil::{derive_nullifier_key, derive_private_address, derive_spending_key, derive_viewing_key};

fn seed_from_mnemonic(phrase: &str) -> Vec<u8> {
    Mnemonic::from_str(phrase).unwrap().to_seed("").to_vec()
}

#[test]
fn test_deterministic_address() {
    let seed = seed_from_mnemonic(
        "abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon about",
    );
    let addr1 = derive_private_address(&seed).unwrap();
    let addr2 = derive_private_address(&seed).unwrap();
    assert_eq!(addr1, addr2);
}

#[test]
fn test_different_seed_different_address() {
    let seed_a = seed_from_mnemonic(
        "abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon about",
    );
    let seed_b = seed_from_mnemonic(
        "legal winner thank year wave sausage worth useful legal winner thank yellow",
    );
    let addr_a = derive_private_address(&seed_a).unwrap();
    let addr_b = derive_private_address(&seed_b).unwrap();
    assert_ne!(addr_a, addr_b);
}

#[test]
fn test_address_starts_with_pvx1() {
    let seed = seed_from_mnemonic(
        "abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon about",
    );
    let address = derive_private_address(&seed).unwrap();
    assert!(address.starts_with("pvx1"));
}

#[test]
fn test_viewing_key_not_spending() {
    let seed = Mnemonic::generate_in(Language::English, 12).unwrap().to_seed("");
    let spending = derive_spending_key(&seed).unwrap();
    let viewing = derive_viewing_key(&seed).unwrap();
    assert_ne!(viewing, spending);
}

#[test]
fn test_nullifier_key_not_spending() {
    let seed = Mnemonic::generate_in(Language::English, 12).unwrap().to_seed("");
    let spending = derive_spending_key(&seed).unwrap();
    let nullifier = derive_nullifier_key(&seed).unwrap();
    assert_ne!(nullifier, spending);
}
