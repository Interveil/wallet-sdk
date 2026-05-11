use sha2::{Digest, Sha256};
use thiserror::Error;
use zeroize::Zeroizing;

const WALLET_PATH: &str = "wallet.dat";

#[derive(Debug, Error)]
pub enum SdkError {
    #[error("invalid mnemonic phrase")]
    InvalidMnemonic,
    #[error("invalid password")]
    InvalidPassword,
    #[error("wallet file corrupted or tampered")]
    WalletCorrupted,
    #[error("vault error: {0}")]
    VaultError(String),
    #[error("address derivation failed")]
    DerivationFailed,
    #[error("wallet session is locked")]
    WalletLocked,
    #[error("message cannot be empty")]
    InvalidMessage,
    #[error("signing failed: {0}")]
    SigningFailed(String),
}

/// Supported blockchain identifiers for signing.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Chain {
    Sol,
    Eth,
}

pub struct Wallet {
    seed: Zeroizing<Vec<u8>>,
    mnemonic: Option<String>,
    eth: String,
    sol: String,
    pvx: String,
}

impl Wallet {
    /// Creates a new random wallet.
    ///
    /// # Errors
    ///
    /// Returns `SdkError::DerivationFailed` if BIP-39 mnemonic generation
    /// or address derivation fails.
    pub fn create() -> Result<Self, SdkError> {
        let core = wallet_core::Wallet::create().map_err(|_| SdkError::DerivationFailed)?;
        Self::from_core(&core)
    }

    /// Imports a wallet from a BIP-39 mnemonic phrase.
    ///
    /// # Errors
    ///
    /// Returns `SdkError::InvalidMnemonic` if the phrase is invalid.
    /// Returns `SdkError::DerivationFailed` if address derivation fails.
    pub fn import(mnemonic: &str) -> Result<Self, SdkError> {
        let core = wallet_core::Wallet::import(mnemonic).map_err(|e| match e {
            wallet_core::WalletError::InvalidMnemonic
            | wallet_core::WalletError::InvalidChecksum => SdkError::InvalidMnemonic,
            wallet_core::WalletError::SeedGenerationFailed => SdkError::DerivationFailed,
        })?;
        Self::from_core(&core)
    }

    fn from_core(core: &wallet_core::Wallet) -> Result<Self, SdkError> {
        let seed = Zeroizing::new(core.seed().to_vec());
        let mnemonic_str = core.mnemonic();
        let mnemonic = if mnemonic_str.is_empty() {
            None
        } else {
            Some(mnemonic_str.to_string())
        };
        let eth = eth::derive_eth_address(&seed).map_err(|_| SdkError::DerivationFailed)?;
        let sol = sol::derive_sol_address(&seed).map_err(|_| SdkError::DerivationFailed)?;
        let pvx = veil::derive_private_address(&seed).map_err(|_| SdkError::DerivationFailed)?;
        Ok(Wallet {
            seed,
            mnemonic,
            eth,
            sol,
            pvx,
        })
    }

    #[must_use]
    pub fn mnemonic(&self) -> Option<&str> {
        self.mnemonic.as_deref()
    }

    #[must_use]
    pub fn eth_address(&self) -> &str {
        &self.eth
    }

    #[must_use]
    pub fn sol_address(&self) -> &str {
        &self.sol
    }

    #[must_use]
    pub fn pvx_address(&self) -> &str {
        &self.pvx
    }

    /// Encrypts the seed and persists it to `wallet.dat`.
    ///
    /// # Errors
    ///
    /// Returns `SdkError::InvalidPassword` if the password-derived key
    /// fails. Returns `SdkError::VaultError` for I/O errors.
    pub fn save(&self, password: &str) -> Result<(), SdkError> {
        let core =
            wallet_core::Wallet::from_seed(self.seed.to_vec(), self.mnemonic.clone());
        storage::save_wallet(WALLET_PATH, &core, password).map_err(map_storage_err)
    }

    /// Loads a wallet from `wallet.dat` and decrypts it.
    ///
    /// # Errors
    ///
    /// Returns `SdkError::InvalidPassword` on wrong password.
    /// Returns `SdkError::WalletCorrupted` if the file is tampered.
    /// Returns `SdkError::VaultError` for I/O errors.
    pub fn load(password: &str) -> Result<Self, SdkError> {
        let core = storage::load_wallet(WALLET_PATH, password).map_err(map_storage_err)?;
        Self::from_core(&core)
    }

    /// Derives the secp256k1 private key for Ethereum (BIP-44 path
    /// `m/44'/60'/0'/0/0`).
    ///
    /// # Errors
    ///
    /// Returns `SdkError::DerivationFailed` if BIP-32 derivation fails.
    pub fn eth_private_key(&self) -> Result<[u8; 32], SdkError> {
        eth::derive_eth_private_key(&self.seed).map_err(|_| SdkError::DerivationFailed)
    }

    /// Derives the Ed25519 private key for Solana (SLIP-0010 path
    /// `m/44'/501'/0'/0'`).
    ///
    /// # Errors
    ///
    /// Returns `SdkError::DerivationFailed` if SLIP-0010 derivation fails.
    pub fn sol_private_key(&self) -> Result<[u8; 32], SdkError> {
        sol::derive_sol_private_key(&self.seed).map_err(|_| SdkError::DerivationFailed)
    }

    /// Derives the Ed25519 spending key for Veil at index 0.
    ///
    /// # Errors
    ///
    /// Returns `SdkError::DerivationFailed` if key derivation fails.
    pub fn pvx_spending_key(&self) -> Result<[u8; 32], SdkError> {
        veil::derive_spending_key(&self.seed).map_err(|_| SdkError::DerivationFailed)
    }
}

/// An unlocked signing session holding cached private keys.
///
/// Created from a [`Wallet`] via [`Session::from_wallet`]. All signing
/// operations go through this type, ensuring private keys are never
/// exposed to the caller.
///
/// Call [`Session::lock`] to consume the session and zero all keys.
pub struct Session {
    #[allow(dead_code)]
    wallet: Wallet,
    sol_key: Zeroizing<[u8; 32]>,
    eth_key: Zeroizing<[u8; 32]>,
}

impl Session {
    /// Creates a signing session from an already-loaded wallet.
    ///
    /// Derives and caches all chain private keys in memory.
    ///
    /// # Errors
    ///
    /// Returns `SdkError::DerivationFailed` if any chain key derivation fails.
    pub fn from_wallet(wallet: Wallet) -> Result<Self, SdkError> {
        let sol_key = Zeroizing::new(wallet.sol_private_key()?);
        let eth_key = Zeroizing::new(wallet.eth_private_key()?);
        Ok(Session { wallet, sol_key, eth_key })
    }

    /// Signs a message with the given chain's private key.
    ///
    /// The message is SHA-256 hashed before signing. No nonce is mixed in.
    ///
    /// # Errors
    ///
    /// Returns `SdkError::InvalidMessage` if `message` is empty.
    /// Returns `SdkError::SigningFailed` if the signing primitive fails.
    pub fn sign(&self, message: &[u8], chain: Chain) -> Result<Vec<u8>, SdkError> {
        self.sign_with_nonce(message, chain, &[])
    }

    /// Signs a message with an explicit nonce for replay protection.
    ///
    /// Hashing: `SHA-256(nonce || message)` before signing.
    /// Pass an empty `nonce` slice to sign without replay protection
    /// (equivalent to [`Session::sign`]).
    ///
    /// # Errors
    ///
    /// Returns `SdkError::InvalidMessage` if `message` is empty.
    /// Returns `SdkError::SigningFailed` if the signing primitive fails.
    pub fn sign_with_nonce(
        &self,
        message: &[u8],
        chain: Chain,
        nonce: &[u8],
    ) -> Result<Vec<u8>, SdkError> {
        if message.is_empty() {
            return Err(SdkError::InvalidMessage);
        }

        let hash = {
            let mut hasher = Sha256::new();
            if !nonce.is_empty() {
                hasher.update(nonce);
            }
            hasher.update(message);
            hasher.finalize()
        };

        let hash_bytes: [u8; 32] = hash.into();
        self.sign_hash(&hash_bytes, chain)
    }

    fn sign_hash(&self, hash: &[u8; 32], chain: Chain) -> Result<Vec<u8>, SdkError> {
        match chain {
            Chain::Sol => {
                let sig = sol::sign_sol(&self.sol_key, hash)
                    .map_err(|e| SdkError::SigningFailed(e.to_string()))?;
                Ok(sig.to_vec())
            }
            Chain::Eth => {
                let sig = eth::sign_eth(&self.eth_key, hash)
                    .map_err(|e| SdkError::SigningFailed(e.to_string()))?;
                Ok(sig.to_vec())
            }
        }
    }

    /// Consumes the session and zeroes all cached private keys.
    ///
    /// After calling this, the session can no longer be used — the
    /// `Zeroizing` wrapper ensures memory is zeroed on drop.
    #[allow(clippy::unnecessary_wraps)]
    pub fn lock(self) -> Result<(), SdkError> {
        // `self` is consumed; Zeroizing zeroes the keys on drop.
        Ok(())
    }
}

fn map_storage_err(e: storage::StorageError) -> SdkError {
    match e {
        storage::StorageError::InvalidPassword | storage::StorageError::DecryptionFailed => {
            SdkError::InvalidPassword
        }
        storage::StorageError::CorruptedFile => SdkError::WalletCorrupted,
        storage::StorageError::IoError(io) => SdkError::VaultError(io.to_string()),
    }
}
