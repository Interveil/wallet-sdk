use ed25519_dalek::Signer;
use zeroize::Zeroizing;

use crate::wallet::{Wallet, WalletError};

pub struct VeilWallet {
    wallet: Wallet,
    sol_private_key: Zeroizing<[u8; 32]>,
}

impl VeilWallet {
    pub fn create() -> Result<Self, WalletError> {
        let wallet = Wallet::create()?;
        let sol_private_key = Zeroizing::new(wallet.sol_private_key()?);
        Ok(Self {
            wallet,
            sol_private_key,
        })
    }

    pub fn import(mnemonic: &str) -> Result<Self, WalletError> {
        let wallet = Wallet::import(mnemonic)?;
        let sol_private_key = Zeroizing::new(wallet.sol_private_key()?);
        Ok(Self {
            wallet,
            sol_private_key,
        })
    }

    #[must_use]
    pub fn solana_address(&self) -> &str {
        self.wallet.sol_address()
    }

    #[must_use]
    pub fn pvx_identity(&self) -> &str {
        self.wallet.pvx_address()
    }

    pub fn sign_intent(
        &self,
        intent: &interveil_sdk::Intent,
    ) -> Result<interveil_sdk::SignedIntent, interveil_sdk::VeilError> {
        intent.sign(self)
    }
}

impl interveil_sdk::IntentSigner for VeilWallet {
    fn public_key(&self) -> String {
        self.wallet.sol_address().to_string()
    }

    fn sign_message(&self, message: &[u8]) -> Result<Vec<u8>, interveil_sdk::VeilError> {
        let signing_key = ed25519_dalek::SigningKey::from_bytes(&self.sol_private_key);
        let signature = signing_key.sign(message);
        Ok(signature.to_bytes().to_vec())
    }
}
