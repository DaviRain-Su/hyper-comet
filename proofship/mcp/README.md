# ProofShip · ProofForge MCP (local Studio)

Desktop Studio attaches this stdio MCP to ACP draft sessions so coding agents
can call `pf_check` / `pf_build` / `pf_artifacts` against the resolved
`proof-forge-next` (vendored toolchain or `PROOF_FORGE_CLI`).

```sh
# Self-check
PROOF_FORGE_CLI=proofship/toolchain/bin/proof-forge-next \
  python3 -I proofship/mcp/proofship_pf_mcp.py --self-check
```

## Resolution (engine)

`comet_engine::studio::mcp::resolve_studio_mcp_servers`:

1. `PROOFSHIP_DISABLE_PF_MCP=1` → none
2. `PROOFSHIP_PF_MCP` → custom stdio script path
3. `$PROOF_FORGE_ROOT/tools/mcp/proof_forge_mcp_server.py` when present (full PF MCP)
4. Else this `proofship_pf_mcp.py` (Studio slim lane)

Optional HTTP for agents that advertise it: `PROOFSHIP_PF_MCP_URL`
(default remote documented in `proofship/web/` — catalog / remote tools; local
compile still prefers stdio + engine gate).

## Web

Browsers are not ACP hosts. The web shell surfaces the remote ProofForge MCP
URL for external agents; gate + deploy remain on the local engine via relay.
