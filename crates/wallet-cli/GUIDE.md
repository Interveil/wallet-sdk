# wallet-cli

Multichain wallet CLI for Ethereum (ETH), Solana (SOL), and Veil (PVX). Built on top of the `wallet-sdk` crate.

## Installation

```bash
cargo build --release -p wallet-cli
```

The binary is named `wallet`.

## Usage

```
wallet <COMMAND>
```

### Global behaviour

- Sensitive commands (`export-seed`, `export-private-key`, `verify`) load the vault directly — provide the vault password via `--password` / `-p` or enter it interactively.
- The vault file (`wallet.dat`) is stored in the current working directory.

---

## Command Reference

### `create`

Generate a new random wallet. Prints the mnemonic phrase and all three addresses **once**. The wallet is NOT persisted.

```
wallet create
```

> **Warning:** The mnemonic is shown only at creation time. Save it securely. It will not be displayed again by any subsequent command.

---

### `import [--save] [-p <password>] <mnemonic>`

Recover a wallet from a BIP-39 mnemonic phrase. Prints the derived addresses.

By default the wallet is NOT persisted. Use `--save` (`-s`) to immediately save to the encrypted vault:

```
wallet import "abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon about"
wallet import --save -p "my-password" "abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon about"
```

---

### `save [-p <password>]`

Create a new random wallet **and immediately save it** to an encrypted vault file on disk. Prints the mnemonic (shown once) and addresses.

```
wallet save
wallet save -p "my-secure-password"
```

The vault is encrypted with the provided password.

---

### `load [-p <password>]`

Load a wallet from the encrypted vault file. Prints the addresses.

```
wallet load
wallet load -p "my-secure-password"
```

---

### `address [--eth] [--sol] [--pvx]`

Load the wallet from the vault and display addresses. If no chain flag is given, all three addresses are shown. Use flags to filter:

```
wallet address
wallet address --eth
wallet address --eth --sol
wallet address --pvx
```

This command always prompts for the vault password interactively (the `--password` flag is not available here).

---

### `export-seed [-p <password>]`

Print the mnemonic seed phrase. Loads the vault directly — no prior `unlock` needed. Prompts for confirmation before displaying the seed.

```
wallet export-seed
wallet export-seed -p "my-password"
```

> **Warning:** Your mnemonic gives full control over all wallets. Only use this in a secure environment.

---

### `export-private-key --chain <CHAIN> [-p <password>]`

Print a chain-specific private key. Loads the vault directly — no prior `unlock` needed. Prompts for confirmation before displaying the key.

```
wallet export-private-key --chain eth
wallet export-private-key --chain sol -p "my-password"
wallet export-private-key --chain pvx
```

Key formats by chain:

| Chain | Format |
|---|---|
| `eth` | Hex with `0x` prefix |
| `sol` | Base58 encoded |
| `pvx` | Bech32 with `pvxsk` human-readable part |

---

### `verify [-p <password>]`

Verify that you have backed up your mnemonic correctly. Loads the vault directly — no prior `unlock` needed. You will be prompted for **3 random words** from your 24-word phrase.

```
wallet verify
wallet verify -p "my-password"
```

Output:
- `Backup verified.` — all 3 words match.
- `Backup verification failed.` — one or more words were incorrect.

---

## Vault file

The encrypted vault is stored as `wallet.dat` in the current working directory. It uses AES-256-GCM with Argon2id key derivation. The mnemonic phrase is preserved inside the vault so that `export-seed` and `verify` can work without a separate session.

## Security Notes

- The mnemonic phrase is **only shown once** (at `create` or `save`). After that it can only be retrieved via `export-seed` with the vault password.
- `export-seed`, `export-private-key`, and `verify` require explicit confirmation before displaying secrets.
- Each sensitive command loads the wallet from the vault, performs the operation, and then drops all secrets — no long-lived in-memory session.
- Interactive password prompts do **not** echo input to the terminal.
