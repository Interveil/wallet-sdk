use veil::{derive_nullifier_key, derive_private_address, derive_spending_key, derive_viewing_key};
use wallet_core::Wallet;

#[test]
fn test_deterministic_address() {
    let wallet = Wallet::create().unwrap();
    let seed = wallet.seed();

    let addr1 = derive_private_address(seed).unwrap();
    let addr2 = derive_private_address(seed).unwrap();

    assert_eq!(addr1, addr2);
}

#[test]
fn test_different_seed_different_address() {
    let phrase_a = "abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon about";
    let phrase_b = "legal winner thank year wave sausage worth useful legal winner thank yellow";

    let wallet_a = Wallet::import(phrase_a).unwrap();
    let wallet_b = Wallet::import(phrase_b).unwrap();

    let addr_a = derive_private_address(wallet_a.seed()).unwrap();
    let addr_b = derive_private_address(wallet_b.seed()).unwrap();

    assert_ne!(addr_a, addr_b);
}

#[test]
fn test_address_starts_with_pvx1() {
    let wallet = Wallet::create().unwrap();
    let address = derive_private_address(wallet.seed()).unwrap();

    assert!(address.starts_with("pvx1"));
}

#[test]
fn test_viewing_key_not_spending() {
    let wallet = Wallet::create().unwrap();
    let seed = wallet.seed();

    let spending = derive_spending_key(seed).unwrap();
    let viewing = derive_viewing_key(seed).unwrap();

    assert_ne!(viewing, spending);
}

#[test]
fn test_nullifier_key_not_spending() {
    let wallet = Wallet::create().unwrap();
    let seed = wallet.seed();

    let spending = derive_spending_key(seed).unwrap();
    let nullifier = derive_nullifier_key(seed).unwrap();

    assert_ne!(nullifier, spending);
}
