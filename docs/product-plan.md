# ProofShip 产品规划（roadmap)

> 定位:**ProofShip = 面向 web3 开发者的通用合约开发部署 app**
> (桌面 + web;ProofForge 机器门禁作内核;Cloudflare 做托管)。
> 核心叙事:AI drafts the contract. The gate decides if it ships.

## 0. 现状盘点(2026-08-12)

已有(本地优先闭环,全部验证绿):

- **Launch Studio**(gpui):NL → 7 个 agent lane(Claude Code/Codex/Grok/Hermes/Pi/Cursor/OpenCode,统一 ACP)起草 ProgramV1 → 机器门禁(check→build→inspect,digest 与脚本逐位一致)→ 失败自动带 PF-\* 诊断修复(≤4 轮)→ 制品清单。
- **引擎**:StudioGate / DraftRunner / StudioLaunchRunner / launch 存储 / RPC;多设备同步基座(Loro CRDT + DO)休眠中。
- **工具链**:vendored proof-forge-next + olean 闭包 + 锁定链工具(solc=EVM / sbpf=Solana / leo=Aleo / nargo=Noir / wat2wasm=wasm 系目标(near/ton)—— 工具已在手)。
- **脚本**:`gate.sh`(任意 .lean→门禁)、`deploy-xlayer-testnet.sh`(gate→`cast send --create`→X Layer testnet,env 持钥)。
- **web 预备**:`proofship/relay/`(Worker+DO,engine→web 旁观/命令)、`proofship/bridge/server.mjs`(参考)。
- **Cloudflare 使用面**(继承自基座,未删):Durable Objects(session/device rooms)、R2(附件)、Workers(auth/relay 路由)。

## 1. 差距分析

用户识别(确认):

1. **合约交互前端** —— ✅ 本地:Studio Contract 面板 + Preview HTML;`Open in browser`。Web 交互台后置(Phase 3.3)。
2. **网络/钱包配置** —— ✅ Networks + Wallets + WC 会话签名;多 EVM 预设(X Layer 优先)。
3. **平台多用户账户体系**(未来)—— 很多开发者各自注册/登录使用 ProofShip:自托管 edge + 登录(WorkOS 管线已内建;**SIWE 钱包登录**对 web3 用户更自然,二选一或并存)、D1 用户/组织表、每用户空间隔离、分享权限策略。注意分层:同步/组织模型从第一天就是多用户设计(workspace doc 按 org 授权、devices 注册表、WorkOS org 门禁,继承自基座);缺的是**托管平台侧**的账户层(relay README 的 R1+ 备注本来就列着:per-device tokens、accounts、sharing policy、D1、OAuth/SIWE)。
4. **右侧前端预览**(类 Codex / 其它 code-agent app)—— ✅ Studio 右侧 Preview + 本机 HTTP;WebView 内嵌后置。

补充(产品化必需):

5. **部署管理** —— ✅ `deployments.json` + Studio Deploy 条;按 project/launch 归集。
6. **项目模型** —— ✅ launch `project_*` + 侧栏分组 + 项目概览(源/门禁计数/部署)。
7. **ABI 驱动的交互台** —— ✅ `comet-abi` schema + Studio call/send;链上事件日志仍靠 explorer。
8. **模板/vertical 体系** —— ✅ RWA + Time-Lock Payout;模板市场后置。
9. **分享** —— `gate-report.json` ✅;只读 launch 链接仍依赖 relay/web(后置)。
10. **多链 deploy lanes** —— 工具已 vendor;ProofForge 目标(evm 已通;solana/aleo/near/ton/cosmwasm 在 proof_forge 侧)按需接。

## 2. 分阶段计划

### Phase 1 — 参赛收尾(本周,截止 8-21 23:59 UTC)

| 项 | 内容 | 负责 |
|---|---|---|
| 1.1 | X Layer testnet 实际部署(funded key + `deploy-xlayer-testnet.sh`) | 用户 |
| 1.2 | 90s 演示视频(修复环为高潮)+ X 账号首帖 @XLayerOfficial + Google 表单 | 用户(材料 docs/competition/) |
| 1.3 | 全链路彩排:app 内 Studio 起草→门禁→脚本部署→浏览器查合约 | 一起 |

### Phase 2 — 本地产品完整化(赛后第一波)

| 项 | 内容 | 依赖/验证 |
|---|---|---|
| 2.1 网络设置 | settings 新增 Networks 页:X Layer testnet(1952)/mainnet(196)预设优先 + Sepolia/Base Sepolia 多 EVM 预备 + 自定义 EVM;存 `networks.json`(本地,非同步) | ✅ |
| 2.2 钱包连接 | settings Wallets 页 + 部署时选择签名者。**多账户地址簿**(label + address + 来源),与 agent-accounts 的 slot 模式同构:多条记录、部署时指定其一;来源三类:**WalletConnect(Reown)**会话(桌面 QR/deeplink,主路径)/ 观察地址(只读)/ dev env-key 引用(文档明示仅测试网)。**私钥永不落盘、永不进 app 存储**;WC 会话仅存内存 | ✅ Connect + 会话签名 |
| 2.3 部署 lane 入 app | `StudioDeploy` RPC:包装 gate→(evm 链)签名发送→回执;**部署记录表** `deployments.json`(network/address/ctor/digest/tx/ts);Studio gate 通过卡出现 "Deploy" 按钮 | ✅ |
| 2.4 合约交互台 | ABI→表单 schema(crate 级,纯 Rust,可测);gpui 面板:view 直接 eth_call 只读,entry 走 2.2 钱包;事件日志 `StudioLogs`(cast logs,近 10k 块) | ✅ |
| 2.6 Studio Preview | 右侧 Preview:ABI→HTML + **应用内 ABI 镜像**(views/events);`Open in browser`;原生 WebView 待 gpui | ✅ 镜像+浏览器;WebView 后置 |

**本地进度(2026-08-12):** Phase 2 主路径已齐;产品聚焦 **OKX X Layer**(Studio 网络选择仅 X Layer;Settings 仍保留其它 EVM 预设)。Preview=应用内 ABI 镜像 + 浏览器 HTML;`StudioLogs` 拉最近事件。多链 deploy lane / 真 WebView 后置。Web/账户(Phase 3–4)后置。

### Phase 3 — web app(Cloudflare 托管)

| 项 | 内容 |
|---|---|
| 3.1 托管壳 | Cloudflare Pages 静态前端(`proofship/web/`);无引擎时展示只读旁观/空快照(诚实降级) |
| 3.2 relay 接通 | `proofship/relay/`:web 旁观本机 Studio(快照+事件尾);web 下命令(prompt/cancel)给本机引擎;engine 侧 WS 客户端在引擎内(Rust),替代 bridge |
| 3.3 web 交互台 | viem + 2.4 的 ABI schema;钱包=浏览器注入/WalletConnect |
| 3.4 部署(web) | 引擎仍是唯一部署执行者(key 不过 relay;安全纪律不变) |

**进度(2026-08-12):** 3.1 静态壳已落在 `proofship/web/`(relay 旁观 + ABI eth_call stub + Send prompt)。3.2 engine→relay Rust WS 客户端已接(`PROOFSHIP_RELAY`);web `cmd.prompt` 会触发本机 `StudioLaunchRun` 并回推事件。3.4 待做。

### Phase 4 — 平台账户与云(多用户)

| 项 | 内容 |
|---|---|
| 4.1 自托管 edge | 部署 `edge/`(Workers+DO+R2);`COMET_EDGE_URL` 指回自有域 |
| 4.2 平台登录(多用户) | 很多开发者各自注册/登录:WorkOS(邮箱/OAuth,管线已内建)+ **SIWE 钱包登录**(web3 用户习惯;钱包地址即账户,与 2.2 的钱包连接打通);D1 用户/组织表;会话与 token 刷新 |
| 4.3 隔离与权限 | 每用户/每 org 空间隔离(org 门禁已内建于 workspace room 授权);relay 升级为 per-device/per-user token(R0 共享 token 仅 spike);分享链接的权限策略(只读/可评论/可下命令) |
| 4.4 分享链接 | 只读 launch/门禁报告链接(relay 签发) |
| 4.5 托管 agent lane | 纯云端执行(pi 等 lane 的托管形态);按需 |

### Phase 5 — 生态与差异化

| 项 | 内容 |
|---|---|
| 5.1 模板体系 | 数据驱动 vertical 模板(RWA 份额登记 + Time-Lock Payout) ✅ `proofship/templates/` + `StudioTemplates` RPC;设计令牌借 Open Design 的 DESIGN.md 思路(非 Vue 运行时);模板市场后置 |
| 5.2 proof badge | 把门禁**已包含**的同文件 theorem certification 暴露为可见"certified"徽章 ✅(gate card + `digest.certified` + `gate-report.json`;诚实边界不变) |
| 5.3 多链 lanes | solana(sbpf 已 vendor)/aleo(leo 已 vendor)/noir(nargo 已 vendor)/near/ton(wat2wasm 已 vendor)/cosmwasm,按 ProofForge 目标逐个接 |
| 5.4 CI | `gate.sh` + `ci-gate-example.sh` 断言 `gate-report.json` ✅;README badge 示例已补 |

## 3. 不变量(任何阶段不破坏)

- 门禁是唯一权威:不过门禁 → 无制品、无部署(fail closed)。
- 密钥永不落盘/不入仓/不过 relay;部署仅 env 持钥或用户钱包签名。
- 诚实边界:工程级机器核验,不声称 full formal verification / 字节码已证 / 证券合规。
- 本地优先:离线可用;云是增强,不是前提。
