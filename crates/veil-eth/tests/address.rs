use std::str::FromStr;
use bip39::Mnemonic;
use veil_eth::derive_eth_address;

fn seed_from_mnemonic(phrase: &str) -> Vec<u8> {
    Mnemonic::from_str(phrase).unwrap().to_seed("").to_vec()
}

#[test]
fn test_deterministic_address() {
    let seed = seed_from_mnemonic(
        "abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon about",
    );

    let address1 = derive_eth_address(&seed).unwrap();
    let address2 = derive_eth_address(&seed).unwrap();

    assert_eq!(address1, address2);
}

#[test]
fn test_different_seed_different_address() {
    let seed_a = seed_from_mnemonic(
        "abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon about",
    );
    let seed_b = seed_from_mnemonic(
        "legal winner thank year wave sausage worth useful legal winner thank yellow",
    );

    let address_a = derive_eth_address(&seed_a).unwrap();
    let address_b = derive_eth_address(&seed_b).unwrap();

    assert_ne!(address_a, address_b);
}

#[test]
fn test_address_starts_with_0x() {
    let seed = seed_from_mnemonic(
        "abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon about",
    );
    let address = derive_eth_address(&seed).unwrap();

    assert!(address.starts_with("0x"));
}

#[test]
fn test_address_length() {
    let seed = seed_from_mnemonic(
        "abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon about",
    );
    let address = derive_eth_address(&seed).unwrap();

    assert_eq!(address.len(), 42);
}
