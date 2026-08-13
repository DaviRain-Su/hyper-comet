# ProofShip skills

Agent skills maintained in this repo, one directory per skill with a
`SKILL.md` entry (frontmatter: `name`, `description`). Layout follows the
multi-skill repo convention, so they are installable with the `skills` CLI:

```sh
npx skills add <this-repo> --skill proofforge-program-v1
npx skills add <this-repo> --skill proofship-evm
```

| Skill | Purpose |
|---|---|
| [proofforge-program-v1](proofforge-program-v1/SKILL.md) | Draft and gate ProofForge ProgramV1 contracts (NL → Lean DSL → machine gate). |
| [proofship-evm](proofship-evm/SKILL.md) | X Layer / EVM chain facts, RPC endpoints, wallet flows for the ship lane. |

## How ProofShip consumes them

`proofforge-program-v1/SKILL.md` is embedded into the engine binary at
compile time (`crates/engine/src/proofforge.rs`) and prepended to every
Session run when a ProofForge toolchain is detected. The matching MCP gate
server lives in `crates/pf-mcp` (`proofship-pf-mcp`, built on the official
`rmcp` SDK) and exposes `pf_doctor` / `pf_check` / `pf_build` /
`pf_artifacts` over stdio.

Editing a SKILL.md therefore changes what agents see on the next engine
build — keep the file self-contained (the whole body is injected).
