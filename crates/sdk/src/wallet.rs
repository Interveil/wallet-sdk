use bip39::{Language, Mnemonic};
use std::path::Path;
use thiserror::Error;
use zeroize::Zeroizing;

const WALLET_PATH: &str = "wallet.dat";

#[derive(Debug, Error)]
pub enum WalletError {
    #[error("invalid mnemonic phrase")]
    InvalidMnemonic,
    #[error("invalid mnemonic checksum")]
    InvalidChecksum,
    #[error("seed generation failed")]
    SeedGenerationFailed,
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

pub struct Wallet {
    mnemonic: String,
    seed: Zeroizing<Vec<u8>>,
    eth_address: String,
    sol_address: String,
    pvx_address: String,
}

impl Wallet {
    fn derive_addresses(seed: &[u8]) -> Result<(String, String, String), WalletError> {
        let eth = eth::derive_eth_address(seed).map_err(|_| WalletError::DerivationFailed)?;
        let sol = sol::derive_sol_address(seed).map_err(|_| WalletError::DerivationFailed)?;
        let pvx =
            veil::derive_private_address(seed).map_err(|_| WalletError::DerivationFailed)?;
        Ok((eth, sol, pvx))
    }

    /// Creates a new random wallet with a 12-word BIP-39 mnemonic.
    ///
    /// # Errors
    ///
    /// Returns `WalletError::DerivationFailed` if BIP-39 mnemonic generation
    /// or address derivation fails.
    pub fn create() -> Result<Self, WalletError> {
        let mnemonic =
            Mnemonic::generate_in(Language::English, 12).map_err(|_| WalletError::DerivationFailed)?;
        let phrase = mnemonic.to_string();
        let seed = Zeroizing::new(mnemonic.to_seed("").to_vec());
        let (eth_address, sol_address, pvx_address) = Self::derive_addresses(&seed)?;
        Ok(Wallet {
            mnemonic: phrase,
            seed,
            eth_address,
            sol_address,
            pvx_address,
        })
    }

    /// Imports a wallet from an existing BIP-39 mnemonic phrase.
    ///
    /// # Errors
    ///
    /// Returns `WalletError::InvalidMnemonic` if the phrase contains
    /// invalid words. Returns `WalletError::InvalidChecksum` if the
    /// mnemonic fails the checksum validation.
    pub fn import(mnemonic: &str) -> Result<Self, WalletError> {
        let mnemonic = Mnemonic::parse_in(Language::English, mnemonic).map_err(|e| match e {
            bip39::Error::InvalidChecksum => WalletError::InvalidChecksum,
            _ => WalletError::InvalidMnemonic,
        })?;
        let phrase = mnemonic.to_string();
        let seed = Zeroizing::new(mnemonic.to_seed("").to_vec());
        let (eth_address, sol_address, pvx_address) = Self::derive_addresses(&seed)?;
        Ok(Wallet {
            mnemonic: phrase,
            seed,
            eth_address,
            sol_address,
            pvx_address,
        })
    }

    /// Reconstructs a `Wallet` from a raw seed, optionally with a mnemonic.
    ///
    /// This is used when loading a wallet from encrypted storage.
    ///
    /// # Errors
    ///
    /// Returns `WalletError::DerivationFailed` if address derivation fails.
    pub fn from_seed(seed: Vec<u8>, mnemonic: Option<String>) -> Result<Self, WalletError> {
        let mnemonic_str = mnemonic.unwrap_or_default();
        let (eth_address, sol_address, pvx_address) = Self::derive_addresses(&seed)?;
        Ok(Wallet {
            mnemonic: mnemonic_str,
            seed: Zeroizing::new(seed),
            eth_address,
            sol_address,
            pvx_address,
        })
    }

    #[must_use]
    pub fn mnemonic(&self) -> Option<&str> {
        if self.mnemonic.is_empty() {
            None
        } else {
            Some(self.mnemonic.as_str())
        }
    }

    #[must_use]
    pub fn seed(&self) -> &[u8] {
        &self.seed
    }

    #[must_use]
    pub fn eth_address(&self) -> &str {
        &self.eth_address
    }

    #[must_use]
    pub fn sol_address(&self) -> &str {
        &self.sol_address
    }

    #[must_use]
    pub fn pvx_address(&self) -> &str {
        &self.pvx_address
    }

    /// Encrypts the seed and persists it to `wallet.dat`.
    ///
    /// # Errors
    ///
    /// Returns `WalletError::InvalidPassword` if key derivation fails.
    /// Returns `WalletError::VaultError` for I/O errors.
    pub fn save(&self, password: &str) -> Result<(), WalletError> {
        self.save_to(WALLET_PATH, password)
    }

    /// Encrypts the seed and persists it to the given path.
    ///
    /// # Errors
    ///
    /// Returns `WalletError::InvalidPassword` if key derivation fails.
    /// Returns `WalletError::VaultError` for I/O errors.
    pub fn save_to(&self, path: impl AsRef<Path>, password: &str) -> Result<(), WalletError> {
        let mnemonic = if self.mnemonic.is_empty() {
            None
        } else {
            Some(self.mnemonic.as_str())
        };
        storage::save_wallet(path, &self.seed, mnemonic, password)
            .map_err(map_storage_err)
    }

    /// Loads a wallet from `wallet.dat` and decrypts it.
    ///
    /// # Errors
    ///
    /// Returns `WalletError::InvalidPassword` on wrong password.
    /// Returns `WalletError::WalletCorrupted` if the file is tampered.
    /// Returns `WalletError::VaultError` for I/O errors.
    pub fn load(password: &str) -> Result<Self, WalletError> {
        Self::load_from(WALLET_PATH, password)
    }

    /// Loads a wallet from the given path and decrypts it.
    ///
    /// # Errors
    ///
    /// Returns `WalletError::InvalidPassword` on wrong password.
    /// Returns `WalletError::WalletCorrupted` if the file is tampered.
    /// Returns `WalletError::VaultError` for I/O errors.
    pub fn load_from(path: impl AsRef<Path>, password: &str) -> Result<Self, WalletError> {
        let (seed, mnemonic) =
            storage::load_wallet(path, password).map_err(map_storage_err)?;
        Self::from_seed(seed, mnemonic)
    }

    /// Creates a new random wallet and immediately persists it to `wallet.dat`.
    ///
    /// Convenience wrapper around [`Wallet::create`] followed by [`Wallet::save`].
    ///
    /// # Errors
    ///
    /// Returns [`WalletError::DerivationFailed`] if generation fails.
    /// Returns [`WalletError::InvalidPassword`] if key derivation fails.
    /// Returns [`WalletError::VaultError`] for I/O errors.
    pub fn create_save(password: &str) -> Result<Self, WalletError> {
        let wallet = Self::create()?;
        wallet.save(password)?;
        Ok(wallet)
    }

    /// Imports a wallet from a mnemonic phrase and immediately persists it
    /// to `wallet.dat`.
    ///
    /// Convenience wrapper around [`Wallet::import`] followed by [`Wallet::save`].
    ///
    /// # Errors
    ///
    /// Returns [`WalletError::InvalidMnemonic`] if the phrase is invalid.
    /// Returns [`WalletError::InvalidChecksum`] if the checksum is wrong.
    /// Returns [`WalletError::DerivationFailed`] if address derivation fails.
    /// Returns [`WalletError::InvalidPassword`] if key derivation fails.
    /// Returns [`WalletError::VaultError`] for I/O errors.
    pub fn import_save(mnemonic: &str, password: &str) -> Result<Self, WalletError> {
        let wallet = Self::import(mnemonic)?;
        wallet.save(password)?;
        Ok(wallet)
    }

    /// Returns the mnemonic seed phrase as a zeroable owned string.
    ///
    /// Returns `None` if the wallet has no mnemonic (e.g. imported without
    /// one from a legacy vault).
    pub fn export_seed(&self) -> Option<Zeroizing<String>> {
        self.mnemonic().map(|m| Zeroizing::new(m.to_string()))
    }

    /// Returns the Ethereum private key as a hex string with `0x` prefix.
    ///
    /// The returned [`Zeroizing`]`<String>` is zeroed on drop.
    ///
    /// # Errors
    ///
    /// Returns `WalletError::DerivationFailed` if BIP-32 derivation fails.
    pub fn export_eth_private_key(&self) -> Result<Zeroizing<String>, WalletError> {
        let key = self.eth_private_key()?;
        Ok(Zeroizing::new(format!("0x{}", hex::encode(key))))
    }

    /// Returns the Solana private key as a base58-encoded string.
    ///
    /// The returned [`Zeroizing`]`<String>` is zeroed on drop.
    ///
    /// # Errors
    ///
    /// Returns `WalletError::DerivationFailed` if SLIP-0010 derivation fails.
    pub fn export_sol_private_key(&self) -> Result<Zeroizing<String>, WalletError> {
        let key = self.sol_private_key()?;
        Ok(Zeroizing::new(bs58::encode(key).into_string()))
    }

    /// Returns the PVX spending key as a bech32 string with `pvxsk` prefix.
    ///
    /// The returned [`Zeroizing`]`<String>` is zeroed on drop.
    ///
    /// # Panics
    ///
    /// Panics if the hardcoded Bech32 HRP `"pvxsk"` is invalid (should never
    /// happen).
    ///
    /// # Errors
    ///
    /// Returns `WalletError::DerivationFailed` if key derivation fails.
    pub fn export_pvx_spending_key(&self) -> Result<Zeroizing<String>, WalletError> {
        let key = self.pvx_spending_key()?;
        let hrp = bech32::Hrp::parse("pvxsk").unwrap();
        let encoded = bech32::encode_lower::<bech32::Bech32>(hrp, key.as_ref()).unwrap();
        Ok(Zeroizing::new(encoded))
    }

    /// Derives the secp256k1 private key for Ethereum (BIP-44 path
    /// `m/44'/60'/0'/0/0`).
    ///
    /// # Errors
    ///
    /// Returns `WalletError::DerivationFailed` if BIP-32 derivation fails.
    pub fn eth_private_key(&self) -> Result<[u8; 32], WalletError> {
        eth::derive_eth_private_key(&self.seed).map_err(|_| WalletError::DerivationFailed)
    }

    /// Derives the Ed25519 private key for Solana (SLIP-0010 path
    /// `m/44'/501'/0'/0'`).
    ///
    /// # Errors
    ///
    /// Returns `WalletError::DerivationFailed` if SLIP-0010 derivation fails.
    pub fn sol_private_key(&self) -> Result<[u8; 32], WalletError> {
        sol::derive_sol_private_key(&self.seed).map_err(|_| WalletError::DerivationFailed)
    }

    /// Derives the Ed25519 spending key for Veil at index 0.
    ///
    /// # Errors
    ///
    /// Returns `WalletError::DerivationFailed` if key derivation fails.
    pub fn pvx_spending_key(&self) -> Result<[u8; 32], WalletError> {
        veil::derive_spending_key(&self.seed).map_err(|_| WalletError::DerivationFailed)
    }
}

fn map_storage_err(e: storage::StorageError) -> WalletError {
    match e {
        storage::StorageError::InvalidPassword | storage::StorageError::DecryptionFailed => {
            WalletError::InvalidPassword
        }
        storage::StorageError::CorruptedFile => WalletError::WalletCorrupted,
        storage::StorageError::IoError(io) => WalletError::VaultError(io.to_string()),
    }
}
