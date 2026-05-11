use sdk::{Wallet, WalletError};

#[test]
fn test_create_wallet() {
    let wallet = Wallet::create().unwrap();
    let words: Vec<&str> = wallet.mnemonic().unwrap().split_whitespace().collect();
    assert_eq!(words.len(), 12);
    assert_eq!(wallet.seed().len(), 64);
}

#[test]
fn test_same_mnemonic_same_seed() {
    let wallet1 = Wallet::create().unwrap();
    let phrase = wallet1.mnemonic().unwrap().to_string();
    let wallet2 = Wallet::import(&phrase).unwrap();
    assert_eq!(wallet1.seed(), wallet2.seed());
}

#[test]
fn test_different_mnemonic_different_seed() {
    let phrase_a = "abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon about";
    let phrase_b = "legal winner thank year wave sausage worth useful legal winner thank yellow";

    let wallet_a = Wallet::import(phrase_a).unwrap();
    let wallet_b = Wallet::import(phrase_b).unwrap();

    assert_ne!(wallet_a.seed(), wallet_b.seed());
}

#[test]
fn test_invalid_mnemonic() {
    let result = Wallet::import("foo bar baz");
    assert!(result.is_err());

    match result {
        Err(WalletError::InvalidMnemonic) => {}
        _ => panic!("expected InvalidMnemonic error"),
    }
}

#[test]
fn test_invalid_checksum() {
    let result = Wallet::import(
        "abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon",
    );
    assert!(result.is_err());

    match result {
        Err(WalletError::InvalidChecksum) => {}
        _ => panic!("expected InvalidChecksum error"),
    }
}
