# ProofShip

> **AI drafts the contract. The gate decides if it ships.**
> 面向 web3 开发者的产品开发部署 app：任何合约产品，从自然语言到机器门禁到上链。

ProofShip 的本体是一个**本地优先的桌面 app**(gpui 原生 UI + Rust 引擎；基座架构
见仓库根 `ARCHITECTURE.md`，门禁/部署服务见 `docs/proofship-studio.md`)，
内嵌 **ProofForge 机器门禁**作为合约开发/部署内核：
在 **Sessions** 里用 ACP agent（skill + MCP）起草 ProgramV1 →
`check → build → inspect`（不过门禁就没有制品、没有部署）→ 部署到目标链
（首发 X Layer)。**没有**第二套 Studio 聊天房——形态对齐 Cursor / ChatGPT。
后续提供 **web app**（托管前端 + Cloudflare relay 旁观/驱动本机引擎）与纯云端
agent 形态。

本目录是 ProofShip 的**平台件**所在（门禁工具链、relay、桥接参考）；具体业务
vertical(如黑客松的 RWA 示例）不入库——它们由 agent 在用户项目里生成。

## 多 code agent

桌面 app 通过 ACP(Agent Client Protocol）驱动本机已登录的 agent CLI:

Claude Code · Codex · Grok Build · Hermes · Pi · Cursor · OpenCode

实现：`crates/harness`（统一 AcpHarness + per-agent spec)。**Sessions** 发消息时
自动注入 ProofForge skill 并挂载 MCP（`enrich_sessions_run_request`）。

## 布局

```text
proofship/
  scripts/
    install-toolchain.sh   ← 一键就绪门禁工具链（bin + olean 闭包 + 锁定链工具）
    gate.sh                ← 通用门禁:任意 .lean → check/build/inspect(开发/调试用)
    ci-gate-example.sh     ← CI 示例:gate + 断言 gate-report.json(certified)
  templates/               ← Studio 数据驱动模板(RWA / Time-Lock;X Layer first；仅为起步样例)
  web/                     ← Phase 3 静态壳(未来真内嵌 dapp UI)
  … (bridge/relay/toolchain/inbox) …
```

本地起草权威 skill：`.agents/skills/proofforge-program-v1/`（任意 ProgramV1 合约；
**Sessions** 对话自动注入，并挂载 `proofship/mcp/` stdio MCP）。
Web / 远程 agent 走 ProofForge HTTP MCP（见 `proofship/web/`）。
`proofship/prompts/program-v1-author.md` 仅作指针。
独立 Studio 聊天入口已移除。

Preview（引擎服务）可用系统浏览器打开 localhost HTML dapp
(no separate WebView binary / window on desktop).

## 快速复现（本机门禁）

```bash
proofship/scripts/install-toolchain.sh   # 就绪 toolchain/(默认从 proof_forge dist+checkout 同步)

# 跑通门禁（用 engine 测试夹具作候选源）:
proofship/scripts/gate.sh crates/engine/tests/fixtures/rwa_share_registry.lean RwaShareRegistry

# CI 形态(断言制品旁 gate-report.json ok+certified):
proofship/scripts/ci-gate-example.sh
```

README / PR 可用的 badge 思路(由 `ci-gate-example.sh` 打印):

```markdown
![gate](https://img.shields.io/badge/gate-passing-3fb950)
```

诚实边界:badge 表示**工程级**门禁通过,不是 full formal verification。
Rust 原生门禁（app 内嵌路径，与脚本同一 CLI/环境解析）:

```bash
cargo test -p comet-engine --lib studio_gate_real_toolchain_passes -- --ignored --nocapture
```

## 工具链解析（脚本与 Rust 门禁一致）

CLI 四级解析:`PF_CLI` → PATH → `PROOF_FORGE_ROOT/.lake/build/bin` →
`proofship/toolchain/bin`。运行时环境自动 pin:`ELAN_TOOLCHAIN`(lean-toolchain
文件内容）与 `PROOF_FORGE_TOOL_ROOT`(`toolchain/tool-root/<platform>`)。
注意 CLI 的 tool-lock 拒绝路径链上任何 group/world-writable 的目录组件
(install 脚本会 chmod 755 自建目录并检查祖先链）。

## 诚实边界（必守）

机器门禁是**工程级**部署前检查（语义检查 + 同文件 theorem certification),
不过门禁不产出制品、不部署。我们**不**声称 full formal verification、不声称
链上字节码已被证明、不声称证券级合规。部署密钥永不进入 app/脚本/仓库——
部署链路只读环境变量持钥。

## 路线

- ✅ 本地 GUI app 基座（多设备同步、多 agent harness、终端/diff 面板）
- ✅ Sessions Agent 对话与机器门禁（NL → agent 起草 → pf_check/pf_build 门禁；失败时 PF-* 诊断回灌修复；通过后展示制品与部署）
- ✅ Preview & Interact 界面（Phase 2.4 引擎 preview ABI 镜像 + 测试网合约交互台）
- ⏳ web app(Cloudflare 托管前端 + relay,`relay/` 为 seam)
- ⏳ 纯云端 agent 形态（pi/其他 lane 的托管执行）
