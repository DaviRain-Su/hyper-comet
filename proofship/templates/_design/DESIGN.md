# ProofShip Dapp Preview — DESIGN.md

Inspired by Open Design's portable `DESIGN.md` systems (tokens + anti-patterns),
not a Vue component dependency. Preview HTML and web interact UIs should read
these tokens so Studio Preview and `proofship/web` stay visually aligned.

## Brand

- Product: ProofShip
- Promise: AI drafts the contract. The gate decides if it ships.
- Primary chain story: X Layer (testnet 1952 first; mainnet 196 for funded ops)

## Color

| Token | Value | Use |
| --- | --- | --- |
| `--bg` | `#0f1115` | Page ground |
| `--panel` | `#171a21` | Cards / chrome |
| `--line` | `#2a3040` | Hairlines |
| `--text` | `#e8ecf4` | Body |
| `--muted` | `#8b93a7` | Meta |
| `--accent` | `#6ee7b7` | Primary action / success |
| `--warn` | `#fbbf24` | Soft warnings |
| `--danger` | `#f87171` | Errors |

Avoid purple-on-white gradients and glow stacks (product UI rule).

## Typography

- Display / brand: Iowan Old Style / Palatino / Georgia (serif)
- UI: Avenir Next / Segoe UI / system sans
- Mono: IBM Plex Mono / SF Mono / Menlo

## Spacing

- Page pad: 24–28px
- Card radius: 12–14px
- Control radius: 8px
- Gap rhythm: 8 / 12 / 18

## Layout

- Preview: single column, max-width ~920px
- Header: brand + contract meta + wallet CTA
- Sections: Views then Writes (one job each)

## Components

- Card: panel bg + line border + 12px radius
- Primary button: green plate (`#134e3a` / `#166534`)
- Ghost button: transparent + line
- Input: dark field, mono text

## Motion

- Prefer one entrance fade; no continuous glow
- Status text updates instantly; no spinner spam on every poll

## Voice

- Short, engineering-honest: no "verified on chain" / full formal claims
- Keys never leave the wallet / env

## Anti-patterns

- Do not embed OpenDesign Vue packages into gpui
- Do not invent a second theme for Preview vs web
- Do not default deploy UI to Ethereum mainnet; prefer X Layer testnet
