# ProofShip 产品规划（roadmap)

> 定位:**ProofShip = 面向 web3 开发者的通用合约开发部署 app**
> (桌面 + web;ProofForge 机器门禁作内核;Cloudflare 做托管)。
> 核心叙事:AI drafts the contract. The gate decides if it ships.

## 0. 现状盘点(2026-08-13)

已有(本地优先闭环,全部验证绿):

- **Sessions 对话**(gpui,Cursor 形态):普通 ACP 聊天 + `proofforge-program-v1` skill + ProofForge stdio MCP(`pf_check`/`pf_build`/`pf_artifacts`)→ 同一条 transcript。**无独立 Studio 聊天入口**。
- **引擎服务层**(`studio::*`):门禁 / 部署 / Preview / 网络钱包 RPC(不是第二套对话 UI)。
- **Sessions 右栏**: **Changes | Preview** 页签。Preview(ABI 镜像 / localhost HTML)不经 `Route::Studio` 可达;Changes 仍依赖 git。
- **工具链**:vendored proof-forge-next + olean 闭包 + 锁定链工具(solc=EVM / sbpf=Solana / leo=Aleo / nargo=Noir / wat2wasm=wasm 系目标(near/ton)—— 工具已在手)。
- **脚本**:`gate.sh`(任意 .lean→门禁)、`deploy-xlayer-testnet.sh`(gate→`cast send --create`→X Layer testnet,env 持钥)。
- **web 预备**:`proofship/relay/`(Worker+DO,Sessions 旁观/命令)、`proofship/web/`(Sessions UI,含 `?share=1` 只读分享视图,打现有 `GET /api/share/:id`)、`proofship/platform-sandbox/`(Platform executor 脚手架)。
- **落地页**:`apps/landing` 文案已 ProofShip 品牌化。
- **参赛材料**:`docs/competition/` + 根 README 按 **Sessions + skill + MCP** 叙述(非 Studio chat);90s 片允许 Preview / interact。
- **Cloudflare 使用面**(继承自基座,未删):Durable Objects(session/device rooms)、R2(附件)、Workers(auth/relay 路由)。

## 1. 差距分析

用户识别(确认):

1. **合约交互前端** —— ✅ 本地:Sessions Preview / Contract 面板 + Preview HTML;`Open in browser`。Web 交互台 ✅ Phase 3 W3。
2. **网络/钱包配置** —— ✅ Networks + Wallets + WC 会话签名;多 EVM 预设(X Layer 优先)。
3. **平台多用户账户体系**(未来)—— 很多开发者各自注册/登录使用 ProofShip:自托管 edge + 登录(WorkOS 管线已内建;**SIWE 钱包登录**对 web3 用户更自然,二选一或并存)、D1 用户/组织表、每用户空间隔离、分享权限策略。注意分层:同步/组织模型从第一天就是多用户设计(workspace doc 按 org 授权、devices 注册表、WorkOS org 门禁,继承自基座);缺的是**托管平台侧**的账户层(relay README 的 R1+ 备注本来就列着:per-device tokens、accounts、sharing policy、D1、OAuth/SIWE)。
4. **右侧前端预览**(类 Codex / 其它 code-agent app)—— ✅ Sessions 右栏 **Changes | Preview**;Preview 不经 `Route::Studio`(本机 HTTP + 系统浏览器 + ABI 镜像)。Changes 仍需 git。真内嵌 dapp 留给 web app。

补充(产品化必需):

5. **部署管理** —— ✅ `deployments.json` + Studio Deploy 条;按 project/launch 归集。
6. **项目模型** —— ✅ launch `project_*` + 侧栏分组 + 项目概览(源/门禁计数/部署)。
7. **ABI 驱动的交互台** —— ✅ `comet-abi` schema + Studio call/send;链上事件日志仍靠 explorer。
8. **模板/vertical 体系** —— ✅ RWA + Time-Lock Payout;模板市场后置。
9. **分享** —— `gate-report.json` ✅;web `?share=1` 只读视图已接 `GET /api/share/:id`。完整 SIWE / 分权仍 Phase 4。
10. **多链 deploy lanes** —— 工具已 vendor;ProofForge 目标(evm 已通;solana/aleo/near/ton/cosmwasm 在 proof_forge 侧)按需接。

## 2. 分阶段计划

### Phase 1 — 参赛收尾(本周,截止 8-21 23:59 UTC)

| 项 | 内容 | 负责 |
|---|---|---|
| 1.1 | X Layer testnet 实际部署(funded key + `deploy-xlayer-testnet.sh`) | 用户 ⏳ |
| 1.2 | 90s 演示视频(修复环为高潮;Preview/interact 可入镜 3–5s)+ X 账号首帖 @XLayerOfficial + Google 表单 | 用户 ⏳(材料 docs/competition/ 已对齐 Sessions + MCP) |
| 1.3 | 全链路彩排:Sessions 对话起草(+MCP)→门禁/部署脚本→浏览器查合约 → Preview/interact | 一起 |

### Phase 2 — 本地产品完整化(赛后第一波)

| 项 | 内容 | 依赖/验证 |
|---|---|---|
| 2.1 网络设置 | settings 新增 Networks 页:X Layer testnet(1952)/mainnet(196)预设优先 + Sepolia/Base Sepolia 多 EVM 预备 + 自定义 EVM;存 `networks.json`(本地,非同步) | ✅ |
| 2.2 钱包连接 | settings Wallets 页 + 部署时选择签名者。**多账户地址簿**(label + address + 来源),与 agent-accounts 的 slot 模式同构:多条记录、部署时指定其一;来源三类:**WalletConnect(Reown)**会话(桌面 QR/deeplink,主路径)/ 观察地址(只读)/ dev env-key 引用(文档明示仅测试网)。**私钥永不落盘、永不进 app 存储**;WC 会话仅存内存 | ✅ Connect + 会话签名 |
| 2.3 部署 lane 入 app | `StudioDeploy` RPC:包装 gate→(evm 链)签名发送→回执;**部署记录表** `deployments.json`;Sessions Preview **Deploy** 条(候选来自 `StudioCandidates`:inbox `.lean` + launch 草稿)+ 网络/钱包选择 | ✅ |
| 2.4 合约交互台 | ABI→表单 schema(crate 级,纯 Rust,可测);gpui 面板:view 直接 eth_call 只读,entry 走 2.2 钱包;事件日志 `StudioLogs`(cast logs,近 10k 块) | ✅ |
| 2.5 项目模型 v1 | launch 归集到 project(path + 名称);Studio 侧栏按项目分组;项目页=源+门禁历史+部署列表 | ✅ 侧栏分组 + 项目概览条 + `project_id` 部署归集 |
| 2.6 Studio Preview | Sessions 右栏 Preview 页签:ABI→localhost HTML + 应用内 ABI 镜像;Start **默认用主机浏览器打开**;不经 `Route::Studio`。真 WebView 内嵌留给 web app | ✅ |

**本地进度(2026-08-13):** Phase 2 能力已齐且产品面可达;聚焦 **OKX X Layer**。**主入口 = Sessions**(无 Studio 聊天页)。Sessions 右栏 **Changes | Preview**;Preview 内可 Deploy(再跑门禁)与 ABI 镜像。脚本 `deploy-xlayer-testnet.sh` 仍可用。
**起草权威:** Sessions 自动注入 `.agents/skills/proofforge-program-v1` + stdio MCP（`proofship/mcp/`；有 `PROOF_FORGE_ROOT` 时用完整 PF MCP）；web 侧表面 HTTP MCP URL（`proofship/web/`）。任意 ProgramV1，非竖切模板限制。

### Phase 3 — web app(Cloudflare 托管) · W1–W5

一个对话面:web 与桌面都是 **Sessions**(旁观 transcript / 下 prompt),不再以
`StudioLaunchRun` 为产品主路径。云协调、机执行:relay/DO 管房间与命令队列;
代码与门禁跑在 executor 上。

#### Executor

| 类型 | 形态 | 职责 |
|---|---|---|
| **UserExecutor** | 本机桌面引擎或用户自挂 VPS(`PROOFSHIP_RELAY` 注册) | Sessions `QueueCommand` + skill/MCP;gate;deploy(DevEnvKey / WC) |
| **PlatformExecutor** | Cloudflare **Sandbox**(`proofship/platform-sandbox/`) | gate / 后续 agent_draft;**拒绝** keyed deploy |
| `@cloudflare/computer` | 仅 spike | 编排/文件;gate 仍进 container — 见 `COMPUTER_SPIKE.md`;**非**生产默认 |

#### 切片

| 项 | 内容 | 状态 |
|---|---|---|
| W1 relay | session 房间;per-device token;Sessions 事件;`cmd.prompt`/`cancel`/`steer`/`deploy`;executor 路由 | ✅ `proofship/relay/` |
| W2 web Sessions | 连接态 + executor 选择器 + transcript + composer;`cmd.prompt` → 引擎 Sessions(enrich skill/MCP) | ✅ `proofship/web/` + engine relay boot |
| W3 interact | viem eth_call + `window.ethereum` 写;从 snapshot ABI/地址填充 | ✅ web interact |
| W4 deploy | UI → `cmd.deploy` → **仅 UserExecutor**;Platform 拒绝 | ✅ relay + web + engine |
| W5 Platform | Sandbox 镜像脚手架 + Computer spike 文档 | ✅ `proofship/platform-sandbox/` |

**不变量:** 不过门禁无制品;私钥/部署签名不过 relay;平台云可跑 check/build/inspect,**不能**代持用户 deploy key。

**验收主路径:** 手机打开 web → 选「我的桌面」在线 → 发 NL → 桌面 Sessions 跑 agent+MCP → 事件回 web;另选 Platform → 仅门禁 job 在 Sandbox;deploy 仍回本机/钱包。

**进度(2026-08-13):** W1–W5 合同与壳已落地。SIWE 账户 + 只读 share mint 已进 relay/web。Platform Sandbox 入口为脚手架(非生产镜像)。WorkOS 组织表 / 评论分权 / 托管 agent 仍 follow-on。

### Phase 4 — 平台账户与云(多用户)

| 项 | 内容 |
|---|---|
| 4.1 自托管 edge | 部署 `edge/`(Workers+DO+R2);`COMET_EDGE_URL` 指回自有域 |
| 4.2 平台登录(多用户) | **SIWE 钱包登录** ✅ relay `/api/auth/siwe/*` + web Account 面板(钱包地址即账户;签名只作登录,不授部署钥)。WorkOS 邮箱/OAuth 仍在 `edge/`(同步基座)。D1 表已写 `schema.sql`,未绑定时走内存 store;生产需 `wrangler d1 create` 后绑定 |
| 4.3 隔离与权限 | 每用户/每 org 空间隔离(org 门禁已内建于 workspace room 授权);relay 已支持 per-device token(W1);分享链接的权限策略(只读/可评论/可下命令)仍后置 |
| 4.4 分享链接 | 只读视图 ✅ web `?share=1`;已登录用户可 `POST /api/sessions/:id/share` 铸造只读 token。评论/下命令分权仍后置 |
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
