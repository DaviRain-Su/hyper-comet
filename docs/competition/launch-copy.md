# ProofShip — 发布文案（X 主帖 + 提交表单）

> 占位纪律：测试网合约地址 / tx / X 帖链接在**实际部署与发帖后回填**，
> 提交前不得声称 "live"。产品本体是桌面 app；web app 未上线，不给 pages.dev 链接。

## X 主帖（提交时发，@XLayerOfficial 必带）

**EN（主推）**

> AI can draft a smart contract in a minute. We built the gate that decides if it's allowed to ship.
>
> ProofShip: describe the contract in plain language → any coding agent you already use
> (7 lanes over ACP) drafts the program → the ProofForge machine gate checks it — and on
> failure the diagnostics feed back for bounded auto-repair (fail-closed, zero artifacts
> on reject) → one command to @XLayerOfficial X Layer testnet.
>
> Demo case: an RWA share registry with transfer policy — but the gate ships any contract.
> Contract on X Layer testnet: （部署后回填地址）
>
> #BuildX #XLayer #AI #RWA

**中文（可连发第二条）**

> AI 一分钟能写完合约，但谁来决定它配不配上线？
>
> ProofShip：自然语言描述需求 → 你常用的 code agent（7 条 lane，统一 ACP）起草程序
> → ProofForge 机器门禁核验，失败诊断自动回灌修复（不过则零制品）
> → 一条命令部署到 @XLayerOfficial X Layer 测试网。
>
> 演示用例：RWA 份额登记 + 受限转让；门禁本身通用于任何合约。
> 测试网合约：（部署后回填地址）
>
> #BuildX #XLayer

**预热帖（今天就可发）**

> Building ProofShip for #BuildX on @XLayerOfficial — AI drafts the contract, a machine gate
> decides what ships. Local-first desktop app, 7 agent lanes over ACP. Testnet soon. ⊢

## Google 表单预填

| 字段 | 内容 |
|---|---|
| Project Name | **ProofShip** |
| Description | ProofShip is a local-first desktop app for building and deploying Web3 products. Describe a contract in natural language in Sessions; any coding agent you already use (Claude Code / Codex / Grok / Hermes / Pi / Cursor / OpenCode, unified over ACP) drafts the ProgramV1 source; the ProofForge machine gate runs semantic checks and same-file theorem certification — fail-closed, zero artifacts on reject, with a bounded auto-repair loop feeding PF-* diagnostics back to the agent; passing programs deploy to X Layer (chain 1952); keys stay in the user's environment. Demo case: an RWA onchain share registry with allowlist + per-tx cap + rolling window cap. Engineering-grade gate; no bytecode-proven or full-formal-verification claims. |
| URL | （GitHub 仓库链接） |
| GitHub | （仓库链接，提交前确认可见性） |
| X handle | @（待定） |
| X 帖 URL | （发布后回填） |
| 补充信息（如有） | Contract (X Layer testnet 1952): （部署后回填地址 + deploy tx） · engine: ProofForge (Lean 4) · agent-agnostic via ACP (7 lanes) · keys stay user-side |

## 措辞红线（发帖/表单共用）

| 说 | 不说 |
|---|---|
| machine-checked gate / fail-closed / zero artifacts | formal verification / formally proven |
| engineering-grade certification | bytecode proven / audited |
| AI-RWA oriented share registry (demo case) | securities issuance / compliant RWA platform |
| agent-agnostic via ACP | works only with one agent / via MCP |
| keys stay user-side | we custody keys |
