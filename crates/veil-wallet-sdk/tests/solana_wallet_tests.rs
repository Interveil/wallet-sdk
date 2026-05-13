use ed25519_dalek::{Signature, SigningKey, Verifier};
use interveil_sdk::IntentSigner;
use veil_wallet_sdk::VeilWallet;

const TEST_MNEMONIC: &str =
    "abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon about";

fn test_wallet() -> VeilWallet {
    VeilWallet::import(TEST_MNEMONIC).unwrap()
}

#[test]
fn wallet_creates_solana_keypair() {
    let wallet = VeilWallet::create().unwrap();
    let addr = wallet.solana_address();
    assert!(!addr.is_empty(), "solana address must not be empty");
    assert!(addr.len() >= 32, "solana address must be reasonable length");
}

#[test]
fn wallet_exports_solana_public_key() {
    let wallet = test_wallet();
    let pubkey = wallet.public_key();

    assert!(!pubkey.is_empty(), "public key string must not be empty");
    // Should match the wallet's own solana_address
    assert_eq!(pubkey, wallet.solana_address());

    // Verify it's valid base58
    let decoded = bs58::decode(&pubkey).into_vec().expect("pubkey must be valid base58");
    assert_eq!(decoded.len(), 32, "Ed25519 pubkey must be 32 bytes");
}

#[test]
fn wallet_signs_message() {
    let wallet = test_wallet();
    let message = b"hello solana signer";

    let signature = wallet.sign_message(message).unwrap();
    assert_eq!(signature.len(), 64, "Ed25519 signature must be 64 bytes");
}

#[test]
fn solana_signature_verifies_locally() {
    let wallet = test_wallet();
    let message = b"verify this message please";

    // Sign the message
    let sig_bytes = wallet.sign_message(message).unwrap();
    let signature = Signature::from_slice(&sig_bytes).unwrap();

    // Reconstruct the public key from the private key
    let wallet_for_key = veil_wallet_sdk::Wallet::import(TEST_MNEMONIC).unwrap();
    let sk_bytes = wallet_for_key.sol_private_key().unwrap();
    let signing_key = SigningKey::from_bytes(&sk_bytes);
    let verifying_key = signing_key.verifying_key();

    // Verify the signature
    assert!(
        verifying_key.verify(message, &signature).is_ok(),
        "signature must verify against the correct public key"
    );
}

#[test]
fn solana_signature_fails_with_wrong_message() {
    let wallet = test_wallet();
    let sig_bytes = wallet.sign_message(b"original message").unwrap();
    let signature = Signature::from_slice(&sig_bytes).unwrap();

    let wallet_for_key = veil_wallet_sdk::Wallet::import(TEST_MNEMONIC).unwrap();
    let sk_bytes = wallet_for_key.sol_private_key().unwrap();
    let signing_key = SigningKey::from_bytes(&sk_bytes);
    let verifying_key = signing_key.verifying_key();

    assert!(
        verifying_key.verify(b"wrong message", &signature).is_err(),
        "signature must fail against a different message"
    );
}
