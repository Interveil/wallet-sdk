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
}

pub struct Wallet {
    seed: Zeroizing<Vec<u8>>,
    mnemonic: Option<String>,
    eth: String,
    sol: String,
    pvx: String,
}

impl Wallet {
    pub fn create() -> Result<Self, SdkError> {
        let core = wallet_core::Wallet::create().map_err(|_| SdkError::DerivationFailed)?;
        Self::from_core(core)
    }

    pub fn import(mnemonic: &str) -> Result<Self, SdkError> {
        let core = wallet_core::Wallet::import(mnemonic).map_err(|e| match e {
            wallet_core::WalletError::InvalidMnemonic
            | wallet_core::WalletError::InvalidChecksum => SdkError::InvalidMnemonic,
            wallet_core::WalletError::SeedGenerationFailed => SdkError::DerivationFailed,
        })?;
        Self::from_core(core)
    }

    fn from_core(core: wallet_core::Wallet) -> Result<Self, SdkError> {
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

    pub fn mnemonic(&self) -> Option<&str> {
        self.mnemonic.as_deref()
    }

    pub fn eth_address(&self) -> &str {
        &self.eth
    }

    pub fn sol_address(&self) -> &str {
        &self.sol
    }

    pub fn pvx_address(&self) -> &str {
        &self.pvx
    }

    pub fn save(&self, password: &str) -> Result<(), SdkError> {
        let core = wallet_core::Wallet::from_seed(self.seed.to_vec());
        storage::save_wallet(WALLET_PATH, &core, password).map_err(map_storage_err)
    }

    pub fn load(password: &str) -> Result<Self, SdkError> {
        let core = storage::load_wallet(WALLET_PATH, password).map_err(map_storage_err)?;
        Self::from_core(core)
    }
}

fn map_storage_err(e: storage::StorageError) -> SdkError {
    match e {
        storage::StorageError::InvalidPassword => SdkError::InvalidPassword,
        storage::StorageError::DecryptionFailed => SdkError::InvalidPassword,
        storage::StorageError::CorruptedFile => SdkError::WalletCorrupted,
        storage::StorageError::IoError(io) => SdkError::VaultError(io.to_string()),
    }
}
