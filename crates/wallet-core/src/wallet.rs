use bip39::{Language, Mnemonic};
use thiserror::Error;
use zeroize::Zeroizing;

#[derive(Debug, Error)]
pub enum WalletError {
    #[error("invalid mnemonic phrase")]
    InvalidMnemonic,
    #[error("invalid mnemonic checksum")]
    InvalidChecksum,
    #[error("seed generation failed")]
    SeedGenerationFailed,
}

pub struct Wallet {
    mnemonic: String,
    seed: Zeroizing<Vec<u8>>,
}

impl Wallet {
    /// Creates a new random wallet with a 12-word BIP-39 mnemonic.
    ///
    /// # Errors
    ///
    /// Returns `WalletError::SeedGenerationFailed` if the OS random source
    /// fails or BIP-39 mnemonic generation fails.
    pub fn create() -> Result<Self, WalletError> {
        let mnemonic = Mnemonic::generate_in(Language::English, 12)
            .map_err(|_| WalletError::SeedGenerationFailed)?;

        let phrase = mnemonic.to_string();
        let seed = Zeroizing::new(mnemonic.to_seed("").to_vec());

        Ok(Wallet {
            mnemonic: phrase,
            seed,
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

        Ok(Wallet {
            mnemonic: phrase,
            seed,
        })
    }

    #[must_use]
    pub fn mnemonic(&self) -> &str {
        &self.mnemonic
    }

    #[must_use]
    pub fn seed(&self) -> &[u8] {
        &self.seed
    }

    /// Reconstructs a `Wallet` from a raw seed, optionally with a mnemonic.
    ///
    /// This is used when loading a wallet from encrypted storage. When a
    /// mnemonic is provided it is preserved; when `None` the mnemonic
    /// is left empty (e.g. legacy vaults that only stored the seed).
    #[must_use]
    pub fn from_seed(seed: Vec<u8>, mnemonic: Option<String>) -> Self {
        Wallet {
            mnemonic: mnemonic.unwrap_or_default(),
            seed: Zeroizing::new(seed),
        }
    }
}
