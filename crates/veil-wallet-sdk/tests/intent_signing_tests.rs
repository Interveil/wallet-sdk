use ed25519_dalek::{Signature, SigningKey, Verifier};
use interveil_sdk::{Chain, Intent, IntentSigner};
use veil_wallet_sdk::VeilWallet;

const TEST_MNEMONIC: &str =
    "abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon about";

fn test_wallet() -> VeilWallet {
    VeilWallet::import(TEST_MNEMONIC).unwrap()
}

/// A fixed deterministic intent — no timestamps or random nonces.
fn test_intent() -> Intent {
    Intent {
        version: 1,
        chain: Chain::Solana,
        action: "transfer_sol".to_string(),
        payload: serde_json::json!({"to": "11111111111111111111111111111111", "lamports": 100}),
        nonce: 42,
        expires_at: 999999999999,
    }
}

#[test]
fn wallet_implements_intent_signer_trait() {
    let wallet = test_wallet();
    // Use as trait object — proves VeilWallet: IntentSigner
    let signer: &dyn IntentSigner = &wallet;

    let pubkey = signer.public_key();
    assert!(!pubkey.is_empty());

    let sig = signer.sign_message(b"trait object test").unwrap();
    assert_eq!(sig.len(), 64);
}

#[test]
fn wallet_signs_sdk_intent() {
    let wallet = test_wallet();
    let intent = test_intent();

    let signed = wallet.sign_intent(&intent).unwrap();

    // The signed intent should contain the same intent fields
    assert_eq!(signed.intent.version, intent.version);
    assert_eq!(signed.intent.chain, intent.chain);
    assert_eq!(signed.intent.action, intent.action);
    assert_eq!(signed.intent.nonce, intent.nonce);
}

#[test]
fn signed_intent_contains_solana_signer_and_signature() {
    let wallet = test_wallet();
    let intent = test_intent();
    let signed = wallet.sign_intent(&intent).unwrap();

    // Signer must match the wallet's Solana public key
    assert_eq!(signed.signer, wallet.solana_address());
    assert!(!signed.signer.is_empty());

    // Signature must be 64 bytes (Ed25519)
    assert_eq!(signed.signature.len(), 64);

    // Verify the signature locally
    let sig = Signature::from_slice(&signed.signature).unwrap();

    let wallet_for_key = veil_wallet_sdk::Wallet::import(TEST_MNEMONIC).unwrap();
    let sk_bytes = wallet_for_key.sol_private_key().unwrap();
    let signing_key = SigningKey::from_bytes(&sk_bytes);
    let verifying_key = signing_key.verifying_key();

    // Veil SDK signs blake3(intent_bytes). We must replicate that here.
    let intent_bytes = intent.to_bytes().unwrap();
    let hash = blake3::hash(&intent_bytes);

    assert!(
        verifying_key.verify(hash.as_bytes(), &sig).is_ok(),
        "signature must verify locally against blake3 hash of intent bytes"
    );
}

#[test]
fn changing_intent_hash_invalidates_signature() {
    let wallet = test_wallet();

    let intent_a = test_intent();
    let mut intent_b = test_intent();
    intent_b.nonce = 99;

    let signed_a = wallet.sign_intent(&intent_a).unwrap();
    let signed_b = wallet.sign_intent(&intent_b).unwrap();

    // Signatures must differ because the intent bytes differ
    assert_ne!(
        signed_a.signature, signed_b.signature,
        "different intents must produce different signatures"
    );

    // Both signatures must verify against their respective intents
    let wallet_for_key = veil_wallet_sdk::Wallet::import(TEST_MNEMONIC).unwrap();
    let sk_bytes = wallet_for_key.sol_private_key().unwrap();
    let signing_key = SigningKey::from_bytes(&sk_bytes);
    let verifying_key = signing_key.verifying_key();

    let sig_a = Signature::from_slice(&signed_a.signature).unwrap();
    let sig_b = Signature::from_slice(&signed_b.signature).unwrap();

    let hash_a = blake3::hash(&intent_a.to_bytes().unwrap());
    let hash_b = blake3::hash(&intent_b.to_bytes().unwrap());

    assert!(verifying_key.verify(hash_a.as_bytes(), &sig_a).is_ok());
    assert!(verifying_key.verify(hash_b.as_bytes(), &sig_b).is_ok());

    // Cross-verifying must fail: sig_a must NOT verify against intent_b's hash
    assert!(verifying_key.verify(hash_b.as_bytes(), &sig_a).is_err());
}
