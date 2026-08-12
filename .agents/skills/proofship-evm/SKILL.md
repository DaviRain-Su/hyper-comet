---
name: proofship-evm
description: >-
  ProofShip multi-EVM deploy guidance with X Layer first. Use when resolving
  chain metadata, RPC/explorer URLs, WalletConnect chain ids, Studio templates,
  or casting deploy/interact against ProofShip networks. Prefer xlayer-testnet
  (1952) unless the user names another chain.
---

# ProofShip EVM (X Layer first)

Borrowed patterns from `paulrberg/agent-skills@evm-chains` (chain metadata
catalog + resolve-before-RPC) and adapted to ProofShip's local-first Studio.

**Product focus right now: OKX X Layer only** (testnet 1952 default, mainnet 196
for funded ops). Sepolia/Base Sepolia remain as Settings builtins for power users
but Studio deploy/interact pickers only offer X Layer.

## Scope

Authoritative builtins live in `comet_proto::builtin_networks()`:

| id | name | chainId | currency | Role |
| --- | --- | --- | --- | --- |
| `xlayer-testnet` | X Layer Testnet | **1952** | OKB | **Default** — templates, demos, DevEnvKey |
| `xlayer-mainnet` | X Layer | 196 | OKB | Funded product ops (no DevEnvKey) |
| `ethereum-sepolia` | Ethereum Sepolia | 11155111 | ETH | Multi-EVM optional |
| `base-sepolia` | Base Sepolia | 84532 | ETH | Multi-EVM optional |

Custom networks may be added in Settings → Networks. Known mainnet chain ids
are blocked for **DevEnvKey** signing; WalletConnect may still target them.

## Resolve before acting

1. Infer network from: explicit name, chain id, template `preferredNetworkId`,
   or Studio selection.
2. If ambiguous, ask. Do **not** default to Ethereum mainnet.
3. Prefer `xlayer-testnet` for drafts, gates, Preview, and competition demos.
4. Explorer links: use the network's `explorerUrl` + `/address/{addr}`.

## Templates

- Catalog: `proofship/templates/*/template.json`
- First vertical: `rwa-share-registry` → module `RwaShareRegistry`, preferred
  network `xlayer-testnet`
- Preview design tokens: `proofship/templates/_design/DESIGN.md` (Open Design–
  style portable tokens; **not** a Vue OpenDesign runtime)

## WalletConnect

- Project id: `PROOFSHIP_WC_PROJECT_ID` / `REOWN_PROJECT_ID`
- Keep the local bridge browser tab open for signing
- Session material never hits disk; address book stores label + address only

## Deploy discipline

- Gate must PASS before deploy (fail closed)
- Keys: env var name for DevEnvKey, or WC session — never paste private keys
- Artifact: `{module}.bin` under studio inbox `out-{module}`

## Anti-patterns

- Do not invent RPC URLs when a builtin already exists
- Do not claim full formal verification / bytecode-proven
- Do not pull `@opensig/opendesign` Vue packages into gpui
