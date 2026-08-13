# OKX Build X Series · AI Season — 参赛对照清单

比赛页:https://web3.okx.com/zh-hans/xlayer/build-x-series
赛季:**2026-08-07 → 2026-08-21 23:59 UTC**(线上);提交:Google 表单。
评审维度(条款 §4):AI 应用与创新性、产品完成度、用户价值、X Layer 集成度、增长潜力、生态贡献。

## 硬性参赛要求 → 我们的证据

| 要求 | 状态 | 证据/动作 |
|---|---|---|
| 产品包含 AI 元素 | ✅ | 7 个 code agent lanes(Claude Code/Codex/Grok/Hermes/Pi/Cursor/OpenCode)在 Sessions 中经 ACP 层驱动; ProofForge skill 草稿在 `.agents/skills/`(MCP 接线后续) |
| 部署于 X Layer(赛期测试网) | 🔄 待执行 | Settings → Networks / Wallets; 操作者在本机用自己的 key 部署;需要 funded 测试网 key(水龙头在比赛页有链接) |
| 后续主网上线 | ⏳ 承诺 | 同一脚本换 RPC/chainId 即可;README 路线声明 |
| 独立 X 账号 + 持续运营 | ⏳ 用户动作 | 建号;首发帖文案见 `launch-copy.md` |
| 官方 X 账号发帖 @XLayerOfficial | ⏳ 用户动作 | `launch-copy.md` 的 X 主帖(EN 主推 + 中文) |
| 8-21 23:59 UTC 前 Google 表单提交 | ⏳ 用户动作 | 备好:repo 链接、demo 视频、测试网合约地址、X 帖链接 |

## 演示主线(评审 90 秒理解)

`video-script-90s.md` 分镜:在 Sessions 中输入 NL 需求 → agent (skill) 起草 ProgramV1 → 机器门禁
check / build / inspect(真实 digest)→ **修复环**(故意触发 PF-* 诊断,agent 读取诊断自动修复)
→ 部署 X Layer testnet → 浏览器查合约。

核心叙事:**"AI drafts the contract. The gate decides if it ships."**
AI 写得快,门禁决定它能不能上链——fail closed,不过门禁没有制品、没有部署。

## 赛道注意

- **Liquidity Grant(5 万 USDT)面向 AI-RWA 赛道**:综合评估产品表现/创新性/
  用户价值/生态贡献。我们的演示用例就是 AI 起草 RWA 受限份额登记
  (演示用例:受限份额登记)——产品故事
  自然覆盖该赛道。
- Launch Grant(20 万)需 1000 万 USDT DEX 交易额——面向发币项目,不适用于
  开发工具,不作目标。

## 材料清单(docs/competition/)

- `launch-copy.md` — X 主帖(EN/CN)+ 提交表单文案(已按现产品重写:桌面 app + ACP lanes;合约地址/链接部署后回填)
- `video-script-90s.md` — 90s 视频分镜与口播(纪律:只说 machine-checked gate,不说 formal verification)

## 诚实边界(评审问答预案)

门禁是工程级机器核验(语义检查 + 同文件 theorem certification),不是 full
formal verification,不声称字节码已证。部署合约不含 invariant(EVM fail closed)。
密钥永不进入 app/仓库,部署仅 env 持钥。
