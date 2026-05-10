use eth::derive_eth_address;
use wallet_core::Wallet;

#[test]
fn test_deterministic_address() {
    let wallet = Wallet::create().unwrap();
    let seed = wallet.seed();

    let address1 = derive_eth_address(seed).unwrap();
    let address2 = derive_eth_address(seed).unwrap();

    assert_eq!(address1, address2);
}

#[test]
fn test_different_seed_different_address() {
    let phrase_a = "abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon about";
    let phrase_b = "legal winner thank year wave sausage worth useful legal winner thank yellow";

    let wallet_a = Wallet::import(phrase_a).unwrap();
    let wallet_b = Wallet::import(phrase_b).unwrap();

    let address_a = derive_eth_address(wallet_a.seed()).unwrap();
    let address_b = derive_eth_address(wallet_b.seed()).unwrap();

    assert_ne!(address_a, address_b);
}

#[test]
fn test_address_starts_with_0x() {
    let wallet = Wallet::create().unwrap();

    let address = derive_eth_address(wallet.seed()).unwrap();

    assert!(address.starts_with("0x"));
}

#[test]
fn test_address_length() {
    let wallet = Wallet::create().unwrap();

    let address = derive_eth_address(wallet.seed()).unwrap();

    assert_eq!(address.len(), 42);
}
