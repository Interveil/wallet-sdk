use sol::derive_sol_address;
use wallet_core::Wallet;

/// BIP-39 standard test vector mnemonic (all-zeros entropy).
const GOLDEN_MNEMONIC: &str =
    "abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon about";

/// Golden test vector — changing this indicates a breaking derivation change.
///
/// Independently verified against:
/// - solders (Python) Keypair::from_seed() with SLIP-0010 Ed25519 path m/44'/501'/0'/0'
/// - seed derived from Wallet::import(GOLDEN_MNEMONIC).seed()
const GOLDEN_ADDRESS: &str = "HAgk14JpMQLgt6rVgv7cBQFJWFto5Dqxi472uT3DKpqk";

#[test]
fn test_golden_vector() {
    let wallet = Wallet::import(GOLDEN_MNEMONIC).unwrap();
    let address = derive_sol_address(wallet.seed()).unwrap();
    assert_eq!(address, GOLDEN_ADDRESS);
}

#[test]
fn test_deterministic() {
    let wallet = Wallet::create().unwrap();
    let seed = wallet.seed();

    let address1 = derive_sol_address(seed).unwrap();
    let address2 = derive_sol_address(seed).unwrap();

    assert_eq!(address1, address2);
}

#[test]
fn test_different_seed_different_address() {
    let phrase_a =
        "abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon about";
    let phrase_b =
        "legal winner thank year wave sausage worth useful legal winner thank yellow";

    let wallet_a = Wallet::import(phrase_a).unwrap();
    let wallet_b = Wallet::import(phrase_b).unwrap();

    let address_a = derive_sol_address(wallet_a.seed()).unwrap();
    let address_b = derive_sol_address(wallet_b.seed()).unwrap();

    assert_ne!(address_a, address_b);
}

#[test]
fn test_valid_base58() {
    let wallet = Wallet::create().unwrap();
    let address = derive_sol_address(wallet.seed()).unwrap();

    let decoded = bs58::decode(&address).into_vec();
    assert!(decoded.is_ok(), "address must be valid base58");
    assert_eq!(decoded.unwrap().len(), 32);
}

#[test]
fn test_address_length() {
    let wallet = Wallet::create().unwrap();
    let address = derive_sol_address(wallet.seed()).unwrap();

    // Base58 encoding of 32 bytes: typically 43-44 characters
    assert!(
        address.len() >= 32 && address.len() <= 44,
        "Solana address length {} out of expected range 32-44",
        address.len()
    );
}
