# ProofShip — 90s 视频脚本（拍摄版）

> 目标：评审在 90 秒内相信三件事——① AI 真的在写合约；② 门禁真的能拦住，
> 而且失败会被**自动修复**；③ 它真的部署在 X Layer 上。
> 载体是**桌面 app（gpui Studio）+ 一键部署脚本**；web app 是后续里程碑，不入镜。
> 纪律：说 *machine-checked gate*，**不**说 formal verification / 字节码已证。

## 分镜与口播

### 0:00–0:08 · Hook（痛点）

**画面**：黑底白字快速打出两行：
「AI 一分钟能写完一个合约。」「你敢让它直接上链吗？」
**口播**：AI can draft a contract in a minute. Would you let it ship straight to a chain?

### 0:08–0:16 · 产品一句话

**画面**：桌面 app，侧栏切到 **Studio**（空态），镜头带到 composer 的 lane
picker（Claude Code / Codex / Grok / Hermes / Pi / Cursor / OpenCode）。
**口播**：ProofShip — describe the contract in plain language, pick any coding
agent you already use, and a machine gate decides what ships to X Layer.

### 0:16–0:28 · 对话 → 草案

**画面**：Studio 输入 NL 需求（份额登记：总量、单笔上限、滚动窗口、白名单）→
**draft card** 展开：模块名、字段表、真实 ProgramV1 源码、"drafted by …" lane 注记。
**口播**：I describe a share registry: total supply, per-transfer cap, a rolling
window, allowlist only. The agent drafts real ProgramV1 source — not pseudocode.

### 0:28–0:40 · 门禁拦截（冲突开始）

**画面**：**gate card** 三阶段 check → build → inspect 跑起来；check 阶段变红，
展开 **PF-\*** 诊断文本。
**口播**：Every draft faces the ProofForge gate. This one fails — semantic checks
catch the bug and emit a PF diagnostic. Fail means zero artifacts. Nothing
bypasses the gate.

### 0:40–0:56 · 修复环（高潮，给足时间）

**画面**：无人操作：诊断自动回灌给同一个 agent，出现 **R2** 徽章的新 draft card
→ 新 gate card 三阶段全绿 → **GATE PASS**，制品清单 + `outputSetDigest` 滚动。
**口播**：Now watch — the diagnostics feed straight back to the agent. Round two:
a revised draft, the full gate again — check, build, inspect with exact content
digests — and this time it passes. Sealed artifacts, bounded auto-repair, no
human in the loop.

### 0:56–1:10 · 一键部署到 X Layer

**画面**：终端跑 `proofship/scripts/deploy-xlayer-testnet.sh …`：先重跑完整门禁，
再输出 `contract=0x…`、`chainId=1952`；切 OKX explorer 页面显示该合约。
**口播**：One command re-runs the full gate, then deploys — the key lives only in
my environment, never in the app or the repo. Live on X Layer testnet, chain 1952.

### 1:10–1:22 · 收口 + 生态

**画面**：回到 Studio 的制品清单；角落打
`Powered by ProofForge · any agent via ACP`。
**口播**：This run drafted an RWA share registry, but ProofShip is
vertical-agnostic: any contract, any of seven agent lanes over ACP, same gate.
The gate is the authority — no pass, no artifacts, no deploy.

### 1:22–1:30 · 结尾卡

**画面**：黑底：⊢ ProofShip · AI drafts the contract. The gate decides if it
ships. · @XLayerOfficial #BuildX
**口播**：ProofShip — the gate decides what ships.

## 拍摄注意

| 项 | 处理 |
|---|---|
| gate 真实时长 ~20s/轮 | 用跳剪；保留 digests 滚动的 2–3 秒真实感 |
| 触发修复环 | 用会被 check 拒的候选源起跑（如改坏回归样本 `crates/engine/tests/fixtures/rwa_share_registry.lean` 的一处约束），确保 PF-* 诊断与 R2 徽章真实出现 |
| 部署段 | 需要 funded testnet key：`PF_XLAYER_CONFIRM=yes PF_XLAYER_PRIVATE_KEY_ENV=… deploy-xlayer-testnet.sh`；录屏时 env 设置不入镜 |
| 孪生纪律 | 口播只说 "machine-checked gate"，不说 "contract is proven" |
| 密钥 | 测试钱包；地址可露，私钥永不出现在任何画面 |
