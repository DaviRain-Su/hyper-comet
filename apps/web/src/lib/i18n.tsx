import {
  createContext,
  useCallback,
  useContext,
  useEffect,
  useMemo,
  useState,
  type ReactNode,
} from "react";

export type Locale = "zh" | "en";

const STORAGE = "proofship.locale";

const copy = {
  zh: {
    navProduct: "产品",
    navHow: "工作流",
    navDiff: "差异",
    navForge: "ProofForge",
    navPricing: "定价",
    navStudio: "Sessions",
    navDownload: "下载",
    navShowcase: "产品演示",
    login: "登录",
    openStudio: "进入 Sessions",
    signOut: "退出",
    heroBadge: "任意链 · 自带 Agent · 形式化门禁",
    heroTitle: "AI 起草合约。门禁决定它能不能上链。",
    heroSub:
      "自然语言描述需求，你用自己电脑上的 coding agent 起草 ProofForge ProgramV1 源——我们不绑定任何 agent 或模型。Web 只是远程面板：提示经中继发到本机，合约写在 Lean 中，可携带同文件定理；机器门禁 check → build → inspect 在你的桌面决定产出与否。",
    ctaLogin: "打开 Sessions",
    ctaDesktop: "下载桌面版",
    ctaForge: "ProofForge 内核",
    ctaGithub: "GitHub",
    statsAgents: "零绑定 · 自带 Agent",
    statsGate: "形式化 + Fail-closed",
    statsChain: "任意链目标",
    productTitle: "为什么是 ProofShip",
    productLead:
      "市面上很多 AI dApp 构建器做的是「一句话生成全栈并部署」。ProofShip 做的是另一件事：在草稿与上链之间放一道机器门禁，用一份可携带定理的 portable 程序源覆盖多链——agent 与模型由你自己订阅，平台不锁死。",
    feature1Title: "自带 Agent，零绑定",
    feature1Body:
      "通过 ACP 接入你已在用的 CLI agent（Claude Code、Codex、Grok Build、Hermes、Pi、Cursor、OpenCode…）。用你自己的模型订阅，不强制平台内置模型，不抽成 token。",
    feature2Title: "门禁即权威",
    feature2Body:
      "语义检查（PF-* 诊断）→ 同文件定理认证 → 构建 → 精确磁盘闭包检验。失败草稿零制品输出——不是 vibe coding 一把梭。",
    feature3Title: "任意链平台",
    feature3Body:
      "内核 ProofForge：一份 program 源受控物化到 EVM、Solana、NEAR、Noir、Aleo 等。首发部署路径含 X Layer；架构目标是链无关，不是单生态工具。",
    feature4Title: "本地优先，云可选",
    feature4Body:
      "Agent 与门禁跑在你自己的电脑上。这张网页只做远程面板，经官方中继对等交互。密钥从不进入网页。",
    showcaseTitle: "产品长这样",
    showcaseLead: "桌面跑 agent 与门禁。网页是远程面板——登录后进入 Sessions，连上你电脑上的 ProofShip。",
    showcaseCaption: "上：桌面 App 实机。下：Web Sessions 远程面板。点进去即可登录进入。",
    showcaseKicker: "产品演示",
    showcaseTabDesktop: "桌面 App",
    showcaseTabEmpty: "Web 开箱",
    showcaseTabSession: "Web 会话中",
    showcaseEnter: "打开 Web Sessions",
    showcaseHeroHint: "Web Sessions 远程面板",
    showcaseShotAlt: "ProofShip 桌面工作台截图：侧栏会话、配对卡片与门禁栏",
    showcaseShotHint: "桌面 App 实机 · 点此看完整演示",
    showcaseShotCaption: "Desktop ProofShip — agent 与门禁跑在这台机器上。",
    downloadTitle: "下载桌面 App",
    downloadLead:
      "Agent 和 ProofForge 门禁必须跑在你自己的电脑上。安装桌面版，再用这张网页远程驱动它。安装包随 GitHub Release 发布。",
    downloadMac: "macOS",
    downloadWin: "Windows",
    downloadLinux: "Linux",
    downloadMacHint: ".dmg / Apple Silicon 与 Intel",
    downloadWinHint: ".exe 安装包",
    downloadLinuxHint: ".AppImage / .deb",
    downloadGet: "去 Release 下载",
    downloadSoon: "安装包随 Release 发布",
    downloadSource: "从源码运行",
    downloadSourceHint: "cargo run -p comet",
    howTitle: "Sessions 工作流",
    howLead: "自然语言 → 你桌面上的 Agent 草稿（Lean + 定理）→ 机器门禁 → 受控多链部署。",
    step1: "描述需求",
    step1Body: "用自然语言说明合约意图与约束。",
    step2: "你的 Agent 起草",
    step2Body: "选定你已付费订阅的本机 agent / 模型，输出带形式化结构的 ProgramV1。",
    step3: "机器门禁",
    step3Body: "check（含定理认证）→ build → inspect，失败即零制品。",
    step4: "部署上链",
    step4Body: "门禁通过后部署到目标链；物化由 ProofForge 多目标内核完成。",
    diffTitle: "和「一句话全栈 dApp 生成器」不一样",
    diffLead:
      "例如 Noah 一类产品偏重单 prompt 生成前端 + 后端 + 合约并快速上 Solana 等生态；收费多是积分与内置 AI 用量。ProofShip 路径不同：",
    diff1Title: "链：平台，不是单生态",
    diff1Body: "一份 portable 源 → 多执行平台受控物化。不是只服务某一条链的模板工厂。",
    diff2Title: "Agent：自带，不锁死",
    diff2Body: "没有平台专属 agent 强绑定。你用自己的 code agent 与模型订阅，平台只负责门禁与物化管线。",
    diff3Title: "产出：先过门禁，再上链",
    diff3Body: "不过门禁没有制品、没有部署。工程级 fail-closed，而不是生成即上线。",
    diff4Title: "收费：云成本，不是 token 抽成",
    diff4Body: "本地与开源路径不靠锁模型赚钱。若做成云服务，订阅费用对应服务器与托管成本，而不是强制买平台 AI 积分。",
    tableColUs: "ProofShip",
    tableColThem: "常见 AI dApp 构建器",
    tableRowScope: "范围",
    tableRowScopeA: "可移植程序源 + 机器门禁",
    tableRowScopeB: "一句话生成全栈应用",
    tableRowChain: "链",
    tableRowChainA: "多目标 / 任意链方向",
    tableRowChainB: "常偏单生态（如 Solana-first）",
    tableRowAgent: "Agent / 模型",
    tableRowAgentA: "自带 — 你的订阅，零绑定",
    tableRowAgentB: "内置 AI + 积分计量",
    tableRowFormal: "形式化",
    tableRowFormalA: "Lean 源 + 同文件定理认证随合约走",
    tableRowFormalB: "通常只生成可运行代码，无证明载体",
    tableRowShip: "上线规则",
    tableRowShipA: "先过 fail-closed 门禁再出制品",
    tableRowShipB: "快速生成，聊天里迭代",
    tableRowPay: "付费点",
    tableRowPayA: "可选云基础设施成本",
    tableRowPayB: "积分 / AI 生成用量",
    forgeTitle: "内核：ProofForge",
    forgeLead:
      "ProofShip 以 ProofForge 为编译与语义内核——Lean 4 多目标编译器：一份 portable program，跨 EVM、Solana、NEAR、Noir 等受控物化；无法保持语义时拒绝。",
    forgeCtaSite: "打开 pf.grok.me",
    forgeCtaRepo: "ProofForge 仓库",
    forgeTargets: "工程目标（任意链方向）",
    forgeFail: "Fail-closed 语义",
    forgeLean: "Lean 4 基础",
    formalTitle: "独特能力：合约自带形式化验证载体",
    formalLead:
      "和「生成一段 Solidity 就完事」不同，ProofForge 的程序源活在 Lean 4 里。业务逻辑、语义需求与同文件定理可以写在同一份源里——门禁会做语义检查与定理认证。制品不是裸代码，而是经过语义求解与证明载体约束后的受控物化。",
    formal1Title: "Lean 程序源",
    formal1Body:
      "program … where 描述 state / init / entry / view。源码是可类型检查、可推理的形式化宿主，不是自由文本补丁。",
    formal2Title: "同文件定理认证",
    formal2Body:
      "定理与证明可与合约源同行。门禁包含 same-file theorem certification——工程级机器验证，随草稿进入 check 循环。",
    formal3Title: "语义需求推导",
    formal3Body:
      "编译器从源推导 ProgramRequirements，再精确匹配目标平台的 SupportClaim；无法保语义则拒绝，禁止 best-effort 降级。",
    formal4Title: "证明随管线走",
    formal4Body:
      "Semantic 层保留语义结构；物化只改制品编码。deploy / prove / verify 为显式步骤，不隐式联网执行。",
    formalNote:
      "诚实边界：当前是工程级机器验证（语义检查 + 同文件定理认证），形式化 lighthouse 以 EVM 优先推进。我们不声称已完成全目标 formal Reference 证明、已证明字节码或证券合规。",
    pricingTitle: "定价原则",
    pricingLead:
      "今天：本地优先、开源工具链可用。明天若提供云端同步 / 托管门禁，订阅只覆盖真实的服务器与运维成本——不绑定模型，不卖强制 AI 积分。",
    pricingLocalTitle: "本地 / 开源",
    pricingLocalPrice: "免费",
    pricingLocalBody: "本机跑 agent 与门禁，网页只做远程面板。密钥与草稿留在你这边。",
    pricingLocalF1: "自带任意 code agent",
    pricingLocalF2: "机器门禁 + 定理认证",
    pricingLocalF3: "ProofForge 多目标内核",
    pricingCloudTitle: "云托管（规划中）",
    pricingCloudPrice: "订阅制",
    pricingCloudBody:
      "协作、同步与托管算力会有服务器成本。届时按用量/席位订阅——为基础设施付费，不是为锁死的模型付费。",
    pricingCloudF1: "可选云端同步与协作",
    pricingCloudF2: "仍可接你自己的 agent / 模型",
    pricingCloudF3: "费用对应算力与存储，非 token 抽成",
    pricingNote: "具体云套餐将在上线时公布。本地路径会持续保留。",
    honestyTitle: "诚实边界",
    honestyBody:
      "门禁是工程级机器验证（语义检查 + 同文件定理认证）。我们不声称完整形式化验证、已证明字节码或证券合规。部署密钥永不接触网页、app 或仓库。我们也不声称自己是「一键生成整站 dApp」产品——我们的核心是门禁、可移植程序源，以及合约自带的形式化载体。",
    ctaTitle: "准备好让门禁替你把关？",
    ctaSub: "登录后进入 Sessions，连上你电脑上的 ProofShip，跑通第一道带定理认证的门禁。",
    footerTag: "自带 Agent。任意链。形式化门禁。",
    footerMit: "MIT License",
    loginTitle: "登录 ProofShip",
    loginSub: "只用来识别你。不会交出部署密钥。登录后进入远程 Sessions。",
    loginContinue: "继续使用",
    loginDisabled: "登录已关闭。",
    loginBack: "返回主页",
    loginEmail: "或使用邮箱",
    loginName: "名字",
    loginPassword: "密码",
    loginCreate: "创建账户",
    loginSignIn: "邮箱登录",
    loginHaveAccount: "已有账户？去登录",
    loginNewHere: "新用户？创建账户",
    studioTitle: "Sessions",
    studioWelcome: "欢迎回来",
    studioLead: "从自然语言描述开始，用你桌面上的 agent 起草可携带定理的 ProgramV1，再过机器门禁。",
    studioPlaceholder: "描述你的合约意图…",
    studioRun: "发到本机 Agent",
    studioHint: "提示经中继发到你的桌面。草稿为 ProofForge ProgramV1（Lean）。密钥不在此页。",
    studioGate: "门禁状态",
    studioIdle: "等待草稿",
    studioSignedOut: "请先登录",
    langEn: "EN",
    langZh: "中文",
    openMenu: "打开菜单",
    closeMenu: "关闭菜单",
  },
  en: {
    navProduct: "Product",
    navHow: "Workflow",
    navDiff: "Difference",
    navForge: "ProofForge",
    navPricing: "Pricing",
    navStudio: "Sessions",
    navDownload: "Download",
    navShowcase: "See it",
    login: "Sign in",
    openStudio: "Open Sessions",
    signOut: "Sign out",
    heroBadge: "Any chain · BYO agent · Formal gate",
    heroTitle: "AI drafts the contract. The gate decides if it ships.",
    heroSub:
      "Describe a contract in natural language. Your own coding agent on your computer drafts a ProofForge ProgramV1 source. We never bind you to a platform agent or model. This page is a remote panel — prompts ride a thin relay to your desktop. Contracts live in Lean and can carry same-file theorems; a machine gate (check → build → inspect) on your machine decides whether anything ships.",
    ctaLogin: "Open Sessions",
    ctaDesktop: "Download desktop",
    ctaForge: "ProofForge kernel",
    ctaGithub: "GitHub",
    statsAgents: "Zero lock-in · BYO agent",
    statsGate: "Formal + fail-closed",
    statsChain: "Any-chain targets",
    productTitle: "Why ProofShip",
    productLead:
      "Many AI dApp builders ship “one prompt → full stack → deploy.” ProofShip does something else: a machine-checked gate between draft and chain, and one portable program source that can carry theorems across execution platforms — with agent and model subscriptions you already pay for, not ours.",
    feature1Title: "BYO agent, zero lock-in",
    feature1Body:
      "ACP drives whichever CLI agent you already use: Claude Code, Codex, Grok Build, Hermes, Pi, Cursor, OpenCode, and more. Bring your own model subscription. No forced in-app model. No token tax.",
    feature2Title: "The gate is the authority",
    feature2Body:
      "Semantic checks with PF-* diagnostics, same-file theorem certification, build, exact-disk-closure inspect. Failing drafts emit zero artifacts — not vibe-code and pray.",
    feature3Title: "Any-chain platform",
    feature3Body:
      "ProofForge kernel: one program source materializes to EVM, Solana, NEAR, Noir, Aleo, and more. First deploy path includes X Layer; the architecture is chain-agnostic, not single-ecosystem tooling.",
    feature4Title: "Local-first, cloud optional",
    feature4Body:
      "The agent and gate run on your computer. This page is a remote panel talking peer-to-peer over the official relay. Keys never enter the browser.",
    showcaseTitle: "What the product looks like",
    showcaseLead:
      "The agent and gate run on your computer. This page is the remote panel — sign in to Sessions and attach desktop ProofShip.",
    showcaseCaption: "Top: the real desktop app. Below: the Web Sessions remote panel. Open it to sign in.",
    showcaseKicker: "Product demo",
    showcaseTabDesktop: "Desktop app",
    showcaseTabEmpty: "Web empty",
    showcaseTabSession: "Web in session",
    showcaseEnter: "Open Web Sessions",
    showcaseHeroHint: "Web Sessions remote panel",
    showcaseShotAlt: "ProofShip desktop workspace: session list, pairing card, and gate rail",
    showcaseShotHint: "Real desktop app · see the full demo",
    showcaseShotCaption: "Desktop ProofShip — the agent and gate run on this machine.",
    downloadTitle: "Download the desktop app",
    downloadLead:
      "The agent and ProofForge gate must run on your machine. Install the desktop app, then drive it from this page. Installers ship with GitHub Releases.",
    downloadMac: "macOS",
    downloadWin: "Windows",
    downloadLinux: "Linux",
    downloadMacHint: ".dmg · Apple Silicon & Intel",
    downloadWinHint: ".exe installer",
    downloadLinuxHint: ".AppImage / .deb",
    downloadGet: "Get it from Releases",
    downloadSoon: "Installers ship with each Release",
    downloadSource: "Run from source",
    downloadSourceHint: "cargo run -p comet",
    howTitle: "Sessions workflow",
    howLead:
      "Natural language → your desktop agent draft (Lean + theorems) → machine gate → controlled multi-chain deploy.",
    step1: "Describe",
    step1Body: "State intent and constraints in plain language.",
    step2: "Your agent drafts",
    step2Body: "Use the agent and model you already subscribe to on your machine. Emit structured ProgramV1.",
    step3: "Machine gate",
    step3Body: "check (incl. theorem cert) → build → inspect. Fail closed — zero artifacts.",
    step4: "Deploy",
    step4Body: "Only gate-passing contracts deploy. Materialization is multi-target via ProofForge.",
    diffTitle: "Not another one-prompt full-stack dApp builder",
    diffLead:
      "Products like Noah focus on single-prompt frontend + backend + contracts with fast launch on ecosystems such as Solana, often monetized via credits and built-in AI usage. ProofShip takes a different path:",
    diff1Title: "Chains: a platform, not one ecosystem",
    diff1Body:
      "One portable source → controlled materialization across execution platforms. Not a template factory for a single chain.",
    diff2Title: "Agents: bring your own, no lock-in",
    diff2Body:
      "No forced platform agent. You keep your code agent and model subscriptions; we own the gate and materialization pipeline.",
    diff3Title: "Shipping: gate first, then chain",
    diff3Body: "No gate pass means no artifacts and no deploy. Engineering-grade fail-closed — not generate-and-go-live.",
    diff4Title: "Pricing: cloud cost, not token tax",
    diff4Body:
      "Local and open paths do not monetize by locking models. If we host in the cloud, subscription covers real server cost — not a mandatory AI credit meter.",
    tableColUs: "ProofShip",
    tableColThem: "Typical AI dApp builders",
    tableRowScope: "Scope",
    tableRowScopeA: "Portable program + machine gate",
    tableRowScopeB: "Full-stack app from one prompt",
    tableRowChain: "Chains",
    tableRowChainA: "Multi-target / any-chain direction",
    tableRowChainB: "Often single ecosystem (e.g. Solana-first)",
    tableRowAgent: "Agent / model",
    tableRowAgentA: "BYO — your subscriptions, zero lock-in",
    tableRowAgentB: "Built-in AI + credit meters",
    tableRowFormal: "Formal methods",
    tableRowFormalA: "Lean source + same-file theorem certification",
    tableRowFormalB: "Usually runnable code only, no proof carrier",
    tableRowShip: "Ship rule",
    tableRowShipA: "Fail-closed gate before artifacts",
    tableRowShipB: "Generate fast, iterate in chat",
    tableRowPay: "Paid for",
    tableRowPayA: "Optional cloud infra cost later",
    tableRowPayB: "Credits / AI generation usage",
    forgeTitle: "Kernel: ProofForge",
    forgeLead:
      "ProofShip is powered by ProofForge — a Lean 4 multi-target compiler. One portable program materializes to EVM, Solana, NEAR, Noir, and more. If semantics cannot be preserved, compilation is rejected.",
    forgeCtaSite: "Open pf.grok.me",
    forgeCtaRepo: "ProofForge repo",
    forgeTargets: "Engineering targets (any-chain direction)",
    forgeFail: "Fail-closed semantics",
    forgeLean: "Lean 4 foundation",
    formalTitle: "What is unique: contracts carry formal verification",
    formalLead:
      "Unlike “emit Solidity and ship,” ProofForge programs live in Lean 4. Business logic, semantic requirements, and same-file theorems can ride in one source. The gate runs semantic checks and theorem certification. Artifacts are controlled materializations after requirements resolve — not bare vibe-generated code.",
    formal1Title: "Lean program source",
    formal1Body:
      "program … where describes state / init / entry / view. The source is a type-checked, reason-able formal host — not free-form paste.",
    formal2Title: "Same-file theorem certification",
    formal2Body:
      "Theorems and proofs can travel with the contract source. The gate includes same-file theorem certification — engineering-grade machine checks in the repair loop.",
    formal3Title: "Semantic requirements inference",
    formal3Body:
      "The compiler derives ProgramRequirements from source, then exact-matches target SupportClaims. If semantics cannot be preserved, it refuses — no best-effort fallback.",
    formal4Title: "Proofs along the pipeline",
    formal4Body:
      "The Semantic layer keeps meaning fixed; materialization only changes encoding. deploy / prove / verify stay explicit — never implicit network side effects.",
    formalNote:
      "Honesty boundary: this is engineering-grade machine verification (semantic checks + same-file theorem certification). Formal lighthouse work is EVM-first. We do not claim completed formal Reference proofs for every target, proven bytecode, or securities compliance.",
    pricingTitle: "Pricing principles",
    pricingLead:
      "Today: local-first, open toolchain. If we offer cloud sync / hosted gates later, subscription covers real servers and ops — not a locked model, not forced AI credits.",
    pricingLocalTitle: "Local / open",
    pricingLocalPrice: "Free",
    pricingLocalBody:
      "Run the agent and the gate on your machine. This page is only a remote panel. Keys and drafts stay with you.",
    pricingLocalF1: "Any code agent you choose",
    pricingLocalF2: "Machine gate + theorem certification",
    pricingLocalF3: "ProofForge multi-target kernel",
    pricingCloudTitle: "Cloud hosted (planned)",
    pricingCloudPrice: "Subscription",
    pricingCloudBody:
      "Collaboration, sync, and hosted compute cost real servers. When we ship cloud, pay for infrastructure — still BYO agent and models.",
    pricingCloudF1: "Optional cloud sync & collaboration",
    pricingCloudF2: "Still BYO agent / model",
    pricingCloudF3: "Fees map to compute & storage, not token tax",
    pricingNote: "Cloud plans will be published when ready. The local path stays.",
    honestyTitle: "Honesty boundary",
    honestyBody:
      "The gate is engineering-grade machine verification (semantic checks + same-file theorem certification). We do not claim full formal verification, proven bytecode, or securities compliance. Deploy keys never touch the page, app, or repository. We also do not claim to be a one-click full-stack dApp generator — our core is the gate, a portable program source, and formal carriers that travel with the contract.",
    ctaTitle: "Ready for a gate that actually decides?",
    ctaSub: "Sign in to open Sessions, attach desktop ProofShip, and run the first theorem-aware gate.",
    footerTag: "BYO agent. Any chain. Formal gate.",
    footerMit: "MIT License",
    loginTitle: "Sign in to ProofShip",
    loginSub: "This identifies you. It never sends a deploy key. Continue to remote Sessions.",
    loginContinue: "Continue with",
    loginDisabled: "Sign-in is disabled.",
    loginBack: "Back to home",
    loginEmail: "or email",
    loginName: "Name",
    loginPassword: "Password",
    loginCreate: "Create account",
    loginSignIn: "Sign in with email",
    loginHaveAccount: "Already have an account? Sign in",
    loginNewHere: "New here? Create an account",
    studioTitle: "Sessions",
    studioWelcome: "Welcome back",
    studioLead:
      "Start from natural language, draft ProgramV1 (with formal structure) via your desktop agent, then run the gate.",
    studioPlaceholder: "Describe your contract intent…",
    studioRun: "Send to desktop agent",
    studioHint:
      "Prompts ride the relay to your desktop. Drafts become ProofForge ProgramV1 (Lean). Keys never live here.",
    studioGate: "Gate status",
    studioIdle: "Waiting for draft",
    studioSignedOut: "Please sign in",
    langEn: "EN",
    langZh: "中文",
    openMenu: "Open menu",
    closeMenu: "Close menu",
  },
} as const;

export type Copy = (typeof copy)[Locale];

type I18nContextValue = {
  locale: Locale;
  setLocale: (l: Locale) => void;
  t: Copy;
};

const I18nContext = createContext<I18nContextValue>({
  locale: "zh",
  setLocale: () => {},
  t: copy.zh,
});

export function I18nProvider({ children }: { children: ReactNode }) {
  const [locale, setLocaleState] = useState<Locale>("zh");

  useEffect(() => {
    try {
      const saved = window.localStorage.getItem(STORAGE);
      if (saved === "en" || saved === "zh") setLocaleState(saved);
    } catch {
      /* ignore */
    }
  }, []);

  const setLocale = useCallback((l: Locale) => {
    setLocaleState(l);
    try {
      window.localStorage.setItem(STORAGE, l);
    } catch {
      /* ignore */
    }
  }, []);

  const value = useMemo<I18nContextValue>(
    () => ({ locale, setLocale, t: copy[locale] }),
    [locale, setLocale],
  );

  return <I18nContext.Provider value={value}>{children}</I18nContext.Provider>;
}

export function useI18n() {
  return useContext(I18nContext);
}

export function useLocale() {
  const { locale, setLocale } = useI18n();
  return { locale, setLocale };
}

export function pick<T>(locale: Locale, en: T, zh: T): T {
  return locale === "zh" ? zh : en;
}
