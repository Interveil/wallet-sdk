# wallet-sdk

Multi-chain non-custodial wallet SDK for Ethereum (ETH), Solana (SOL), and Veil (PVX).

Handles **only cryptographic operations** — key derivation, address generation, encrypted storage, and message signing. No blockchain RPC, no transaction building.

---

## Dependency

```toml
[dependencies]
sdk = { path = "crates/sdk" }
```

---

## Quick Start

```rust
use sdk::{Chain, Session, Wallet, WalletError};

fn main() -> Result<(), WalletError> {
    let wallet = Wallet::create()?;

    println!("ETH: {}", wallet.eth_address());
    println!("SOL: {}", wallet.sol_address());
    println!("PVX: {}", wallet.pvx_address());

    let session = Session::from_wallet(wallet)?;

    let sig = session.sign(b"hello world", Chain::Sol)?;
    println!("Signature ({} bytes): {:?}", sig.len(), sig);

    session.lock()?;
    Ok(())
}
```

Save as `src/main.rs` and run with `cargo run`.

---

## `Wallet` — the core storage type

Holds the BIP-39 seed and derived addresses. All secret material is zeroed on drop.

### Creating / importing

| Method | Description |
|---|---|---|
| `Wallet::create()` | Generate a new random 12-word BIP-39 wallet |
| `Wallet::import(mnemonic)` | Recover from an existing BIP-39 mnemonic phrase |
| `Wallet::create_save(password)` | Create and persist to `wallet.dat` in one step |
| `Wallet::import_save(mnemonic, password)` | Import and persist to `wallet.dat` in one step |

```rust
let wallet = Wallet::create()?;

let wallet = Wallet::import(
    "abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon about"
)?;

// One-step creation + persistence
let wallet = Wallet::create_save("my-password")?;
```

### Addresses

```rust
wallet.eth_address();   // "0x9858effd232b4033e47d90003d41ec34ecaeda94"
wallet.sol_address();   // "HAgk14JpMQLgt6rVgv7cBQFJWFto5Dqxi472uT3DKpqk"
wallet.pvx_address();   // "pvx1..."
```

### Mnemonic

Available **only** immediately after `create()` or `import()`. Returns `None` after loading from vault (the mnemonic is preserved in v0x02 vaults; see `save`/`load`).

```rust
if let Some(phrase) = wallet.mnemonic() {
    println!("Backup this phrase: {phrase}");
}
```

### Vault persistence

Saves/loads the encrypted wallet to/from `wallet.dat` in the current directory. Uses AES-256-GCM with Argon2id key derivation. The mnemonic is preserved inside the vault (format v0x02).

```rust
// Save / load with default path (wallet.dat)
wallet.save("my-secure-password")?;
let wallet = Wallet::load("my-secure-password")?;

// Save / load with custom path
wallet.save_to("backup.dat", "my-secure-password")?;
let wallet = Wallet::load_from("backup.dat", "my-secure-password")?;
```

### Exporting secrets

Returns secrets as `Zeroizing<String>` — zeroed on drop, can't be accidentally `Debug` printed.

```rust
// Seed phrase
if let Some(phrase) = wallet.export_seed() {
    println!("Backup phrase: {phrase}");
    // phrase is zeroed when it goes out of scope
}

// Chain private keys (human-readable format)
let eth_key = wallet.export_eth_private_key()?;    // "0x4f3e...a1b2"
let sol_key = wallet.export_sol_private_key()?;    // "2xs7...9f3k" (base58)
let pvx_key = wallet.export_pvx_spending_key()?;   // "pvxsk1qyp...u8q" (bech32)
```

### Raw private keys (advanced)

Derive chain-specific private keys from the seed as raw `[u8; 32]`. Prefer `export_*` or `Session::sign()` instead.

```rust
let eth_key: [u8; 32] = wallet.eth_private_key()?;   // secp256k1
let sol_key: [u8; 32] = wallet.sol_private_key()?;   // Ed25519
let pvx_key: [u8; 32] = wallet.pvx_spending_key()?;  // Ed25519
```

---

## `Session` — the signing gateway

An unlocked session holding cached private keys. This is the **only** way to sign messages. It enforces at the type level that no signing happens without an explicit unlock.

### Creating a session

```rust
let wallet = Wallet::create()?;
let session = Session::from_wallet(wallet)?;
```

### Signing

Hashes the message with **SHA-256** before signing. Returns raw signature bytes.

```rust
use sdk::Chain;

// Sign (no nonce — SHA-256(message))
let sig: Vec<u8> = session.sign(b"hello", Chain::Sol)?;
let sig: Vec<u8> = session.sign(b"hello", Chain::Eth)?;

// Sign with nonce for replay protection — SHA-256(nonce || message)
let sig: Vec<u8> = session.sign_with_nonce(b"hello", Chain::Sol, b"request-42")?;
```

Supported chains:

| Chain | Algorithm | Signature length |
|---|---|---|
| `Chain::Sol` | Ed25519 | 64 bytes |
| `Chain::Eth` | ECDSA (secp256k1, RFC 6979) | 64 bytes |

### Locking

Consumes the session. All cached keys are zeroed on drop via `Zeroizing`.

```rust
session.lock()?;
// session can no longer be used (compile-time guarantee)
```

---

## Nonce for replay protection

`sign_with_nonce` mixes a nonce into the hash before signing:

```
signature = Chain_sign(SHA-256(nonce || message))
```

Every unique nonce produces a unique signature, even for the same message. This prevents replay attacks — a signature captured from one context cannot be reused in another.

```rust
let msg = b"transfer 10 SOL";
let tx_1 = session.sign_with_nonce(msg, Chain::Sol, b"nonce-1")?;
let tx_2 = session.sign_with_nonce(msg, Chain::Sol, b"nonce-2")?;

assert_ne!(tx_1, tx_2); // different nonces → different signatures
```

Nonces work identically for `Chain::Eth`.

---

## Error handling

```rust
use sdk::WalletError;
```

| Variant | Meaning |
|---|---|---|
| `InvalidMnemonic` | Mnemonic phrase has invalid words or checksum |
| `InvalidPassword` | Wrong vault password or decryption failure |
| `WalletCorrupted` | Vault file is corrupt or tampered with |
| `VaultError(String)` | I/O error during save/load |
| `DerivationFailed` | BIP-32/SLIP-0010 key derivation failure |
| `WalletLocked` | Reserved for future locked-state enforcement |
| `InvalidMessage` | Empty message passed to `sign()` / `sign_with_nonce()` |
| `SigningFailed(String)` | Ed25519/ECDSA signing primitive failed |

```rust
match result {
    Err(WalletError::InvalidMessage) => eprintln!("Message cannot be empty"),
    Err(e) => eprintln!("Error: {e}"),
    Ok(sig) => println!("Signature: {sig:02x?}"),
}
```

---

## Full workflow example

```rust
use sdk::{Chain, Wallet, Session, WalletError};
use std::fs;

fn main() -> Result<(), WalletError> {
    // 1. Create and persist in one step
    let wallet = Wallet::create_save("strong-password")?;
    println!("SOL address: {}", wallet.sol_address());

    // 2. Later: load from vault
    let loaded = Wallet::load("strong-password")?;
    assert_eq!(loaded.sol_address(), wallet.sol_address());

    // 3. Export seed phrase (Zeroizing<String> — zeroed on drop)
    if let Some(phrase) = loaded.export_seed() {
        println!("Seed phrase: {phrase}");
    }

    // 4. Export formatted private keys
    let eth_key = loaded.export_eth_private_key()?;
    let sol_key = loaded.export_sol_private_key()?;
    println!("ETH key: {eth_key}");
    println!("SOL key: {sol_key}");

    // 5. Unlock into signing session
    let session = Session::from_wallet(loaded)?;

    // 6. Sign with nonce
    let sig = session.sign_with_nonce(
        b"transfer 10 SOL to alice",
        Chain::Sol,
        b"tx-nonce-001",
    )?;
    println!("Signature ({} bytes)", sig.len());

    // 7. Lock — keys are zeroed
    session.lock()?;

    let _ = fs::remove_file("wallet.dat");
    Ok(())
}
```

---

## Security notes

- **Never log private keys or seeds.** The `Session` API keeps keys internal — `sign()` never exposes them. Export methods return `Zeroizing<String>` which can't be `Debug` printed without explicit `.expose()`.
- **Secrets are zeroed on drop.** `Wallet`, `Session`, and export return types use `Zeroizing` wrappers that clear memory when the value is dropped.
- **Session lifecycle.** Call `from_wallet()` to unlock, `lock()` to consume and zero. Keeping sessions alive unnecessarily increases the window for memory-scraping attacks.
- **Export methods don't require a password.** The `Wallet` instance is the authorization boundary — you needed the vault password to load it. Adding a password parameter to export methods would create a false sense of security (the seed is already in memory).
- **No blockchain I/O.** This SDK only does crypto. Transaction building, RPC calls, and network interactions belong in a separate layer.
