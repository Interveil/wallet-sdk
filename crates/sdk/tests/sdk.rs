use std::sync::Mutex;
use sdk::{Chain, Session, Wallet, WalletError};

static SERIAL: Mutex<()> = Mutex::new(());

fn with_isolated_dir<F>(test_name: &str, f: F)
where
    F: FnOnce(),
{
    let _guard = SERIAL.lock().unwrap();
    let dir = std::env::temp_dir().join(test_name);
    std::fs::create_dir_all(&dir).unwrap();
    let original = std::env::current_dir().unwrap();
    std::env::set_current_dir(&dir).unwrap();

    f();

    std::env::set_current_dir(&original).unwrap();
    std::fs::remove_dir_all(&dir).ok();
}

#[test]
fn test_create_valid_addresses() {
    let wallet = Wallet::create().unwrap();
    let eth = wallet.eth_address();
    let sol = wallet.sol_address();
    let pvx = wallet.pvx_address();

    assert!(eth.starts_with("0x"), "eth must start with 0x");
    assert_eq!(eth.len(), 42, "eth must be 42 chars");

    assert!(!sol.is_empty(), "sol must not be empty");

    assert!(pvx.starts_with("pvx1"), "pvx must start with pvx1");
}

#[test]
fn test_import_deterministic() {
    let phrase =
        "abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon about";
    let a = Wallet::import(phrase).unwrap();
    let b = Wallet::import(phrase).unwrap();

    assert_eq!(a.eth_address(), b.eth_address());
    assert_eq!(a.sol_address(), b.sol_address());
    assert_eq!(a.pvx_address(), b.pvx_address());
}

#[test]
fn test_different_seed_different_addresses() {
    let a = Wallet::import(
        "abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon about",
    )
    .unwrap();
    let b = Wallet::import(
        "legal winner thank year wave sausage worth useful legal winner thank yellow",
    )
    .unwrap();

    assert_ne!(a.eth_address(), b.eth_address());
    assert_ne!(a.sol_address(), b.sol_address());
    assert_ne!(a.pvx_address(), b.pvx_address());
}

#[test]
fn test_invalid_mnemonic_fails() {
    let result = Wallet::import("foo bar baz");
    assert!(matches!(result, Err(WalletError::InvalidMnemonic)));
}

#[test]
fn test_save_load_roundtrip() {
    with_isolated_dir("sdk_roundtrip", || {
        let wallet = Wallet::create().unwrap();
        let eth = wallet.eth_address().to_string();
        let sol = wallet.sol_address().to_string();
        let pvx = wallet.pvx_address().to_string();

        wallet.save("correct-horse-battery-staple").unwrap();

        let loaded = Wallet::load("correct-horse-battery-staple").unwrap();
        assert_eq!(eth, loaded.eth_address());
        assert_eq!(sol, loaded.sol_address());
        assert_eq!(pvx, loaded.pvx_address());
    });
}

#[test]
fn test_wrong_password_fails_load() {
    with_isolated_dir("sdk_wrong_pw", || {
        let wallet = Wallet::create().unwrap();
        wallet.save("correct-password").unwrap();

        let result = Wallet::load("wrong-password");
        assert!(matches!(result, Err(WalletError::InvalidPassword)));
    });
}

#[test]
fn test_save_to_load_from_custom_path() {
    with_isolated_dir("sdk_custom_path", || {
        let wallet = Wallet::create().unwrap();
        let eth = wallet.eth_address().to_string();
        let path = "my-custom-wallet.dat";

        wallet.save_to(path, "pw").unwrap();

        let loaded = Wallet::load_from(path, "pw").unwrap();
        assert_eq!(eth, loaded.eth_address());
        assert_eq!(wallet.mnemonic(), loaded.mnemonic());
    });
}

#[test]
fn test_create_save_roundtrip() {
    with_isolated_dir("sdk_create_save", || {
        let wallet = Wallet::create_save("pw").unwrap();
        let loaded = Wallet::load("pw").unwrap();
        assert_eq!(wallet.eth_address(), loaded.eth_address());
        assert_eq!(wallet.sol_address(), loaded.sol_address());
        assert_eq!(wallet.pvx_address(), loaded.pvx_address());
        assert_eq!(wallet.mnemonic(), loaded.mnemonic());
    });
}

#[test]
fn test_import_save_roundtrip() {
    with_isolated_dir("sdk_import_save", || {
        let phrase =
            "abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon about";
        let wallet = Wallet::import_save(phrase, "pw").unwrap();
        let loaded = Wallet::load("pw").unwrap();
        assert_eq!(wallet.eth_address(), loaded.eth_address());
        assert_eq!(wallet.mnemonic(), loaded.mnemonic());
    });
}

// ---------------------------------------------------------------------------
// Export tests
// ---------------------------------------------------------------------------

const GOLDEN_MNEMONIC: &str =
    "abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon about";

#[test]
fn test_export_seed_matches() {
    let wallet = Wallet::import(GOLDEN_MNEMONIC).unwrap();
    let exported = wallet.export_seed().unwrap();
    assert_eq!(*exported, GOLDEN_MNEMONIC);
}

#[test]
fn test_export_seed_from_loaded_vault() {
    with_isolated_dir("export_seed_vault", || {
        let wallet = Wallet::import_save(GOLDEN_MNEMONIC, "pw").unwrap();
        let loaded = Wallet::load("pw").unwrap();
        let exported = loaded.export_seed().unwrap();
        assert_eq!(*exported, GOLDEN_MNEMONIC, "mnemonic preserved through save/load");
    });
}

#[test]
fn test_export_eth_private_key_format() {
    let wallet = Wallet::import(GOLDEN_MNEMONIC).unwrap();
    let key = wallet.export_eth_private_key().unwrap();
    assert!(key.starts_with("0x"), "ETH key must start with 0x");
    assert_eq!(key.len(), 66, "0x + 64 hex chars = 66");
    // All characters after 0x must be valid hex
    hex::decode(&key[2..]).expect("ETH key must be valid hex");
}

#[test]
fn test_export_sol_private_key_format() {
    let wallet = Wallet::import(GOLDEN_MNEMONIC).unwrap();
    let key = wallet.export_sol_private_key().unwrap();
    let decoded = bs58::decode(&*key).into_vec().expect("SOL key must be valid base58");
    assert_eq!(decoded.len(), 32, "SOL key must decode to 32 bytes");
}

#[test]
fn test_export_pvx_spending_key_format() {
    let wallet = Wallet::import(GOLDEN_MNEMONIC).unwrap();
    let key = wallet.export_pvx_spending_key().unwrap();
    assert!(key.starts_with("pvxsk1"), "PVX key must start with pvxsk1");
}

// ---------------------------------------------------------------------------
// Session / signing tests
// ---------------------------------------------------------------------------

const TEST_MNEMONIC: &str =
    "abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon about";

fn test_session() -> Session {
    let wallet = Wallet::import(TEST_MNEMONIC).unwrap();
    Session::from_wallet(wallet).unwrap()
}

// --- Solana ---

#[test]
fn test_sign_sol_deterministic() {
    let session = test_session();
    let msg = b"hello solana";

    let sig_a = session.sign(msg, Chain::Sol).unwrap();
    let sig_b = session.sign(msg, Chain::Sol).unwrap();

    assert_eq!(sig_a.len(), 64, "Ed25519 signature must be 64 bytes");
    assert_eq!(sig_a, sig_b, "signatures must be deterministic");
}

#[test]
fn test_sign_sol_different_messages() {
    let session = test_session();
    let sig_a = session.sign(b"message one", Chain::Sol).unwrap();
    let sig_b = session.sign(b"message two", Chain::Sol).unwrap();

    assert_ne!(sig_a, sig_b, "different messages must produce different signatures");
}

#[test]
fn test_sign_sol_verify() {
    use ed25519_dalek::{Signature, Verifier, VerifyingKey};
    use sha2::{Digest, Sha256};

    let session = test_session();
    let msg = b"verify me";

    let hash = Sha256::digest(msg);
    let sig_bytes = session.sign(msg, Chain::Sol).unwrap();
    let signature = Signature::from_slice(&sig_bytes).unwrap();

    let wallet = Wallet::import(TEST_MNEMONIC).unwrap();
    let pubkey_bytes = {
        let sk = ed25519_dalek::SigningKey::from_bytes(&wallet.sol_private_key().unwrap());
        sk.verifying_key().to_bytes()
    };
    let verifying_key = VerifyingKey::from_bytes(&pubkey_bytes).unwrap();

    assert!(
        verifying_key.verify(&hash, &signature).is_ok(),
        "signature must verify against the public key"
    );
}

// --- Ethereum ---

#[test]
fn test_sign_eth_deterministic() {
    let session = test_session();
    let msg = b"hello ethereum";

    let sig_a = session.sign(msg, Chain::Eth).unwrap();
    let sig_b = session.sign(msg, Chain::Eth).unwrap();

    assert_eq!(sig_a.len(), 64, "ECDSA signature must be 64 bytes");
    assert_eq!(sig_a, sig_b, "signatures must be deterministic (RFC 6979)");
}

#[test]
fn test_sign_eth_different_messages() {
    let session = test_session();
    let sig_a = session.sign(b"message one", Chain::Eth).unwrap();
    let sig_b = session.sign(b"message two", Chain::Eth).unwrap();

    assert_ne!(sig_a, sig_b, "different messages must produce different signatures");
}

#[test]
fn test_sign_eth_verify() {
    use k256::ecdsa::signature::hazmat::PrehashVerifier;
    use k256::ecdsa::{Signature, SigningKey, VerifyingKey};
    use sha2::{Digest, Sha256};

    let session = test_session();
    let msg = b"verify me eth";

    let hash = Sha256::digest(msg);
    let sig_bytes = session.sign(msg, Chain::Eth).unwrap();
    let signature = Signature::from_slice(&sig_bytes).unwrap();

    let eth_key = Wallet::import(TEST_MNEMONIC).unwrap().eth_private_key().unwrap();
    let signing_key = SigningKey::from_slice(&eth_key).unwrap();
    let verifying_key = VerifyingKey::from(&signing_key);

    assert!(
        verifying_key.verify_prehash(&hash, &signature).is_ok(),
        "ECDSA signature must verify against the public key"
    );
}

// --- Nonce ---

#[test]
fn test_sign_with_nonce_differs() {
    let session = test_session();
    let msg = b"hello";

    let sig_no_nonce = session.sign_with_nonce(msg, Chain::Sol, &[]).unwrap();
    let sig_standard = session.sign(msg, Chain::Sol).unwrap();
    let sig_nonce_a = session.sign_with_nonce(msg, Chain::Sol, b"nonce-1").unwrap();
    let sig_nonce_b = session.sign_with_nonce(msg, Chain::Sol, b"nonce-2").unwrap();
    let sig_nonce_eth = session.sign_with_nonce(msg, Chain::Eth, b"nonce-1").unwrap();

    assert_eq!(
        sig_no_nonce, sig_standard,
        "empty nonce must produce same result as sign()"
    );
    assert_ne!(
        sig_no_nonce, sig_nonce_a,
        "nonce must change the Solana signature"
    );
    assert_ne!(
        sig_nonce_a, sig_nonce_b,
        "different nonces must produce different Solana signatures"
    );
    assert_eq!(sig_nonce_eth.len(), 64, "ECDSA nonced signature must be 64 bytes");
}

// --- General ---

#[test]
fn test_sign_empty_message_fails() {
    let session = test_session();
    let result = session.sign(b"", Chain::Sol);
    assert!(matches!(result, Err(WalletError::InvalidMessage)));

    let result = session.sign_with_nonce(b"", Chain::Eth, b"nonce");
    assert!(matches!(result, Err(WalletError::InvalidMessage)));

    let result = session.sign(b"", Chain::Eth);
    assert!(matches!(result, Err(WalletError::InvalidMessage)));
}

#[test]
fn test_session_lock_consumes() {
    let session = test_session();
    assert!(session.sign(b"test", Chain::Sol).is_ok());
    assert!(session.sign(b"test", Chain::Eth).is_ok());
    session.lock().unwrap();
}
