# Interveil Wallet SDK

A multichain non-custodial wallet SDK for **Ethereum**, **Solana**, and **PVX** (Veil privacy chain). Generates BIP-39 mnemonic seeds, derives deterministic chain addresses and private keys using BIP-44 (secp256k1) and SLIP-0010 (Ed25519) standards, and encrypts wallet data with AES-256-GCM keyed via Argon2id. The mnemonic is exposed at creation only and never stored after vault encryption — private key export requires explicit user confirmation.

Built as a Rust workspace with layered architecture: core mnemonic/seed generation, per-chain address modules, encrypted storage, a unified SDK, and a thin CLI binary. All secret material is zeroized on drop.

## License

BSD-3-Clause
