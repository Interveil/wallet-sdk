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

- Sensitive commands (`export-seed`, `export-private-key`, `verify`) require an **unlocked session** — run `wallet unlock` first.
- Passwords can be provided via `--password` / `-p` or entered interactively.
- The vault file is stored in the current working directory as an encrypted JSON file.

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

The vault is encrypted with the provided password. A vault file (e.g. `wallet.json`) is created in the current directory.

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

### `unlock`

Decrypt the vault and load the wallet into an **in-memory session**. This is a prerequisite for `export-seed`, `export-private-key`, and `verify`.

```
wallet unlock
```

Prompts for the vault password. The session holds the mnemonic and all private keys in memory (zeroed on drop).

---

### `lock`

Wipe all secrets from the in-memory session. After this, `export-seed`, `export-private-key`, and `verify` will refuse until `unlock` is run again.

```
wallet lock
```

---

### `export-seed`

Print the mnemonic seed phrase. Requires an **unlocked session** (`wallet unlock` first). Prompts for confirmation before displaying the seed.

```
wallet export-seed
```

> **Warning:** Your mnemonic gives full control over all wallets. Only use this in a secure environment.

---

### `export-private-key --chain <CHAIN>`

Print a chain-specific private key. Requires an **unlocked session**. Prompts for confirmation.

```
wallet export-private-key --chain eth
wallet export-private-key --chain sol
wallet export-private-key --chain pvx
```

Key formats by chain:

| Chain | Format |
|---|---|
| `eth` | Hex with `0x` prefix |
| `sol` | Base58 encoded |
| `pvx` | Bech32 with `pvxsk` human-readable part |

---

### `verify`

Verify that you have backed up your mnemonic correctly. Requires an **unlocked session**. You will be prompted for **3 random words** from your 24-word phrase.

```
wallet verify
```

Output:
- `Backup verified.` — all 3 words match.
- `Backup verification failed.` — one or more words were incorrect.

---

## Session Lifecycle

```
                    ┌──────────┐
                    │  create   │── (no persistence)
                    │  import   │── (no persistence)
                    │  save     │── creates vault + prints mnemonic
                    │  load     │── reads vault, prints addresses
                    └─────┬─────┘
                          │
                    ┌─────▼──────┐
                    │   unlock   │── decrypts vault into memory
                    └─────┬──────┘
                          │
         ┌────────────────┼────────────────┐
         ▼                ▼                ▼
   ┌──────────┐   ┌──────────────┐   ┌────────┐
   │export-seed│   │export-pvt-key│   │ verify │
   └──────────┘   └──────────────┘   └────────┘
         │                │                │
         └────────────────┼────────────────┘
                          ▼
                    ┌──────────┐
                    │   lock   │── wipe session
                    └──────────┘
```

- `unlock` must be called once before any of `export-seed`, `export-private-key`, or `verify`.
- `lock` ends the session and clears secrets from memory.
- The session does **not** persist across process restarts.

## Vault file

The encrypted vault is stored as `wallet.json` in the current working directory. It uses the same format as `wallet-sdk`. Keep this file and its password secure.

## Security Notes

- The mnemonic phrase is **only shown once** (at `create` or `save`). There is no way to retrieve it without the vault password.
- `export-seed` and `export-private-key` require explicit confirmation before displaying secrets.
- `lock` sets the in-memory session to `None`; the previous secrets are dropped and zeroed.
- Private keys and mnemonics use `Zeroizing` wrappers so that sensitive memory is zeroed on drop.
- Interactive password prompts do **not** echo input to the terminal.
