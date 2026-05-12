use sha2::{Digest, Sha256};
use zeroize::Zeroizing;

use crate::wallet::{Wallet, WalletError};

/// Supported blockchain identifiers for signing.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Chain {
    Sol,
    Eth,
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
    /// Returns `WalletError::DerivationFailed` if any chain key derivation fails.
    pub fn from_wallet(wallet: Wallet) -> Result<Self, WalletError> {
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
    /// Returns `WalletError::InvalidMessage` if `message` is empty.
    /// Returns `WalletError::SigningFailed` if the signing primitive fails.
    pub fn sign(&self, message: &[u8], chain: Chain) -> Result<Vec<u8>, WalletError> {
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
    /// Returns `WalletError::InvalidMessage` if `message` is empty.
    /// Returns `WalletError::SigningFailed` if the signing primitive fails.
    pub fn sign_with_nonce(
        &self,
        message: &[u8],
        chain: Chain,
        nonce: &[u8],
    ) -> Result<Vec<u8>, WalletError> {
        if message.is_empty() {
            return Err(WalletError::InvalidMessage);
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

    fn sign_hash(&self, hash: &[u8; 32], chain: Chain) -> Result<Vec<u8>, WalletError> {
        match chain {
            Chain::Sol => {
                let sig = veil_sol::sign_sol(&self.sol_key, hash)
                    .map_err(|e| WalletError::SigningFailed(e.to_string()))?;
                Ok(sig.to_vec())
            }
            Chain::Eth => {
                let sig = veil_eth::sign_eth(&self.eth_key, hash)
                    .map_err(|e| WalletError::SigningFailed(e.to_string()))?;
                Ok(sig.to_vec())
            }
        }
    }

    /// Consumes the session and zeroes all cached private keys.
    ///
    /// After calling this, the session can no longer be used — the
    /// `Zeroizing` wrapper ensures memory is zeroed on drop.
    #[allow(clippy::unnecessary_wraps)]
    pub fn lock(self) -> Result<(), WalletError> {
        Ok(())
    }
}
