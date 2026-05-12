use blake3::Hasher;

/// Derives a deterministic 32-byte key from a BIP-39 seed.
///
/// Implements: `sk_i = H(seed || index)`
///
/// Uses Blake3 as the PRF. The zeroize feature is enabled on blake3
/// so that hash outputs are zeroed on drop.
///
/// # Indexes
///
/// | Index | Key |
/// |-------|-----|
/// | 0     | spending key |
/// | 1     | viewing key  |
/// | 2     | nullifier key |
pub(crate) fn derive_key(seed: &[u8], index: u8) -> [u8; 32] {
    let mut hasher = Hasher::new();
    hasher.update(seed);
    hasher.update(&[index]);
    *hasher.finalize().as_bytes()
}
