---
name: proofship-evm
description: >-
  ProofShip multi-EVM deploy guidance with X Layer first. Use when resolving
  chain metadata, RPC/explorer URLs, WalletConnect chain ids, or casting
  deploy/interact against ProofShip networks. Prefer xlayer-testnet (1952)
  unless the user names another chain.
---

# ProofShip EVM (X Layer first)

Borrowed patterns from `paulrberg/agent-skills@evm-chains` (chain metadata
catalog + resolve-before-RPC) and adapted to ProofShip's local-first desktop.

**Product focus right now: OKX X Layer only** (testnet 1952 default, mainnet 196
for funded ops). Sepolia/Base Sepolia remain as Settings builtins for power users.

## Scope

Authoritative builtins live in `comet_proto::builtin_networks()`:

| id | name | chainId | currency | Role |
| --- | --- | --- | --- | --- |
| `xlayer-testnet` | X Layer Testnet | **1952** | OKB | **Default** — drafts, demos |
| `xlayer-mainnet` | X Layer | 196 | OKB | Funded product ops (no DevEnvKey) |
| `ethereum-sepolia` | Ethereum Sepolia | 11155111 | ETH | Multi-EVM optional |
| `base-sepolia` | Base Sepolia | 84532 | ETH | Multi-EVM optional |

Custom networks may be added in Settings → Networks. Known mainnet chain ids
are blocked for **DevEnvKey** signing; WalletConnect may still target them.

## Resolve before acting

1. Infer network from: explicit name, chain id, or Settings → Networks pick.
2. If ambiguous, ask. Do **not** default to Ethereum mainnet.
3. Prefer `xlayer-testnet` for drafts, gates, and competition demos.
4. Explorer links: use the network's `explorerUrl` + `/address/{addr}`.

## WalletConnect

- Project id: `PROOFSHIP_WC_PROJECT_ID` / `REOWN_PROJECT_ID`
- Keep the local bridge browser tab open for signing
- Session material never hits disk; address book stores label + address only

## OKX OnchainOS MCP (when attached)

When the user configured an OnchainOS API key (Settings → Networks), every
session carries the hosted `okx-onchainos` MCP server. Prefer its tools for
DEX work instead of hand-rolling calldata:

- supported chains / liquidity sources: `dex-okx-dex-aggregator-supported-chains`, `dex-okx-dex-liquidity`
- best aggregated quote (output, price impact, route): `dex-okx-dex-quote`
- ERC-20 approve calldata: `dex-okx-dex-approve-transaction`
- full swap transaction (calldata + value + gas): `dex-okx-dex-swap`

These tools **construct** transactions only — signing and sending stays in
ProofShip (Settings → Wallets / Deploy flow). Never ask the user to paste a
private key to "complete" a swap.

## Deploy discipline

- Gate must PASS before deploy (fail closed)
- Keys: env var name for DevEnvKey, or WC session — never paste private keys
- Local Alloy keys live under `{data_dir}/studio/wallet-secrets/` (mode 0600)

## Anti-patterns

- Do not invent RPC URLs when a builtin already exists
- Do not claim full formal verification / bytecode-proven
