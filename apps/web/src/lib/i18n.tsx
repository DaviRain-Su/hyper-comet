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
    navHow: "怎么用",
    navDiff: "有何不同",
    navForge: "内核",
    navPricing: "定价",
    navStudio: "工作台",
    navDownload: "下载",
    navShowcase: "长什么样",
    login: "登录",
    openStudio: "进入工作台",
    signOut: "退出",
    heroBadge: "自带 Agent · 多链 · 过门再上链",
    heroTitle: "AI 起草合约。过了门禁，才能上链。",
    heroSub:
      "用自然语言说清楚需求，本机的 coding agent 写成 Lean 程序。这张网页就是工作台：提示发到你电脑，门禁在桌面跑完检查、构建、核验，过了才有制品。",
    ctaLogin: "开始",
    ctaDesktop: "下载桌面版",
    ctaForge: "了解 ProofForge",
    ctaGithub: "GitHub",
    statsAgents: "自带 Agent，不绑模型",
    statsGate: "过不了门，不出制品",
    statsChain: "一份源码，多条链",
    productTitle: "为什么是 ProofShip",
    productLead:
      "很多 AI 合约工具是「一句话生成，立刻部署」。ProofShip 做另一件事：草稿和上链之间有一道机器门禁。Agent 用你自己的，链也不绑死一家。",
    feature1Title: "用你自己的 Agent",
    feature1Body:
      "接上你已经在用的 CLI agent：Claude Code、Codex、Grok、Cursor、OpenCode… 模型和订阅都是你的，平台不抽成。",
    feature2Title: "门禁说了算",
    feature2Body:
      "语义检查、同文件定理认证、构建、磁盘闭包核验。没过关，制品为零。不是 vibe coding 一把梭。",
    feature3Title: "一份源码，多条链",
    feature3Body:
      "ProofForge 把同一份程序编到 EVM、Solana、NEAR、Noir、Aleo 等。先发路径含 X Layer，按多链来设计，不是单生态工具。",
    feature4Title: "算力在你电脑上",
    feature4Body: "Agent 和门禁跑在本机。网页只做远程面板。部署密钥不会进浏览器。",
    showcaseTitle: "产品长这样",
    showcaseLead: "桌面跑 Agent 和门禁。网页是远程工作台——点「开始」登录即可。",
    showcaseCaption: "上：桌面 App 实机。下：网页工作台。点开始即可登录。",
    showcaseKicker: "产品演示",
    showcaseTabDesktop: "桌面 App",
    showcaseTabEmpty: "网页开箱",
    showcaseTabSession: "网页会话中",
    showcaseEnter: "开始",
    showcaseHeroHint: "网页工作台",
    showcaseShotAlt: "ProofShip 桌面工作台截图：侧栏会话、配对卡片与门禁栏",
    showcaseShotHint: "桌面 App 实机 · 点此看完整演示",
    showcaseShotCaption: "桌面版 ProofShip — Agent 和门禁跑在这台机器上。",
    downloadTitle: "下载桌面版",
    downloadLead:
      "Agent 和门禁必须跑在你电脑上。装好桌面版，再用这张网页远程驱动。安装包在 GitHub Release。",
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
    howTitle: "怎么用",
    howLead: "自然语言 → 本机 Agent 写出 Lean → 机器门禁 → 过了再部署。",
    step1: "描述需求",
    step1Body: "用自然语言说明合约意图和约束。",
    step2: "本机 Agent 起草",
    step2Body: "用你已经订阅的本机 agent / 模型，写出带结构的 ProgramV1。",
    step3: "机器门禁",
    step3Body: "检查（含定理认证）→ 构建 → 核验。没过关，制品为零。",
    step4: "部署上链",
    step4Body: "只有过门的合约才会部署。多链编译由 ProofForge 完成。",
    diffTitle: "不是「一句话生成全栈」",
    diffLead:
      "有的产品强调一个提示词生成前端、后端、合约，并快速上某一条链。ProofShip 走另一条路：",
    diff1Title: "多链，不是单生态",
    diff1Body: "一份可移植源码，受控编译到多条执行链。不是某条链的模板工厂。",
    diff2Title: "Agent 自带，不锁死",
    diff2Body: "没有平台专属 agent。你用自己的 code agent 和模型订阅，我们负责门禁和编译管线。",
    diff3Title: "先过门，再上链",
    diff3Body: "不过门就没有制品、没有部署。工程级 fail-closed，不是生成即上线。",
    diff4Title: "收费对应云成本，不是 token 抽成",
    diff4Body:
      "本地和开源路径不靠锁模型赚钱。若做成云服务，订阅覆盖服务器和运维，而不是强制买平台 AI 积分。",
    tableColUs: "ProofShip",
    tableColThem: "常见 AI 合约工具",
    tableRowScope: "范围",
    tableRowScopeA: "可移植程序源 + 机器门禁",
    tableRowScopeB: "一句话生成全栈应用",
    tableRowChain: "链",
    tableRowChainA: "一份源码，多条链",
    tableRowChainB: "常偏单生态（如 Solana）",
    tableRowAgent: "Agent / 模型",
    tableRowAgentA: "自带 — 你的订阅，不绑定",
    tableRowAgentB: "内置 AI + 积分计量",
    tableRowFormal: "形式化",
    tableRowFormalA: "Lean 源码 + 同文件定理认证",
    tableRowFormalB: "通常只生成可运行代码",
    tableRowShip: "上线规则",
    tableRowShipA: "先过门禁，再出制品",
    tableRowShipB: "快速生成，聊天里迭代",
    tableRowPay: "付费点",
    tableRowPayA: "可选的云基础设施",
    tableRowPayB: "积分 / AI 生成用量",
    forgeKicker: "内核",
    forgeTitle: "编译内核 ProofForge",
    forgeLead:
      "ProofShip 用 ProofForge 做编译和语义内核。一份 Lean 程序，编到 EVM、Solana、NEAR、Noir 等；保不住语义就拒绝，不会凑合出货。",
    forgeCtaSite: "打开 pf.grok.me",
    forgeCtaRepo: "ProofForge 仓库",
    forgeTargets: "编译目标",
    forgeFail: "保不住语义就拒绝",
    forgeFailBody: "不会降级，也不会「尽量编译」。",
    forgeLean: "基于 Lean 4",
    forgeLeanBody: "多目标编译器，程序源可以推理。",
    formalKicker: "形式化",
    formalTitle: "合约可以带着证明走",
    formalLead:
      "不是「吐一段 Solidity 就完事」。程序写在 Lean 4 里：业务逻辑、语义要求和同文件定理可以放在同一份源码。门禁会做语义检查和定理认证。出来的是过关后的制品，不是裸代码。",
    formal1Title: "Lean 程序源",
    formal1Body:
      "program … where 描述 state / init / entry / view。源码能做类型检查、能推理，不是自由文本补丁。",
    formal2Title: "同文件定理认证",
    formal2Body: "定理和证明可以跟合约源放在一起。门禁会做同文件定理认证，跟着草稿进入检查循环。",
    formal3Title: "语义对不上就拒绝",
    formal3Body:
      "编译器从源码推导需求，再精确匹配目标平台能力。保不住语义就拒绝，禁止凑合降级。",
    formal4Title: "证明跟着管线走",
    formal4Body: "语义层保住含义，物化只改编码。deploy / prove / verify 都是显式步骤，不会偷偷联网。",
    formalNote:
      "说清楚：当前是工程级机器验证（语义检查 + 同文件定理认证），形式化主线先推 EVM。我们不声称已完成全目标形式化证明、已证明字节码或证券合规。",
    pricingTitle: "怎么收费",
    pricingLead:
      "今天：本地优先，开源可用。以后如果提供云同步 / 托管门禁，订阅只覆盖真实的服务器和运维——不绑模型，不卖强制 AI 积分。",
    pricingLocalKicker: "本机",
    pricingLocalTitle: "本地 / 开源",
    pricingLocalPrice: "免费",
    pricingLocalBody: "本机跑 Agent 和门禁，网页只做远程面板。密钥和草稿留在你这边。",
    pricingLocalF1: "自带任意 code agent",
    pricingLocalF2: "机器门禁 + 定理认证",
    pricingLocalF3: "ProofForge 多链内核",
    pricingCloudKicker: "云端",
    pricingCloudTitle: "云托管（规划中）",
    pricingCloudPrice: "订阅制",
    pricingCloudBody:
      "协作、同步和托管算力会有服务器成本。届时按用量 / 席位订阅——为基础设施付费，不是为锁死的模型付费。",
    pricingCloudF1: "可选云端同步与协作",
    pricingCloudF2: "仍可接你自己的 agent / 模型",
    pricingCloudF3: "费用对应算力与存储，不是 token 抽成",
    pricingNote: "具体云套餐上线时公布。本地路径会一直保留。",
    honestyTitle: "说清楚边界",
    honestyBody:
      "门禁是工程级机器验证（语义检查 + 同文件定理认证）。我们不声称完整形式化验证、已证明字节码或证券合规。部署密钥不会接触网页、App 或仓库。我们也不是「一键生成整站」产品——核心是门禁、可移植程序源，以及合约自带的证明。",
    ctaTitle: "先从网页开始",
    ctaSub: "登录后进入工作台，连上你电脑上的 ProofShip。",
    footerTag: "自带 Agent。多链。过门再上链。",
    footerMit: "MIT License",
    loginTitle: "登录后进入工作台",
    loginSub: "只用来认出你。不会拿走部署密钥。",
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
    studioTitle: "工作台",
    studioWelcome: "欢迎回来",
    studioLead: "从自然语言开始，用桌面上的 agent 起草 ProgramV1，再过机器门禁。",
    studioPlaceholder: "描述你的合约意图…",
    studioRun: "发到本机 Agent",
    studioHint: "提示经中继发到你的桌面。草稿是 ProofForge ProgramV1（Lean）。密钥不在此页。",
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
    navHow: "How it works",
    navDiff: "Difference",
    navForge: "Kernel",
    navPricing: "Pricing",
    navStudio: "Workspace",
    navDownload: "Download",
    navShowcase: "See it",
    login: "Sign in",
    openStudio: "Open workspace",
    signOut: "Sign out",
    heroBadge: "BYO agent · many chains · gate before ship",
    heroTitle: "AI drafts the contract. The gate decides if it ships.",
    heroSub:
      "Describe the contract in plain language. Your coding agent on this machine writes Lean. This page is the workspace — prompts ride a thin relay to your desktop. The gate runs check → build → inspect there. Nothing ships unless it passes.",
    ctaLogin: "Get started",
    ctaDesktop: "Download desktop",
    ctaForge: "About ProofForge",
    ctaGithub: "GitHub",
    statsAgents: "BYO agent, no model lock-in",
    statsGate: "No pass, no artifact",
    statsChain: "One source, many chains",
    productTitle: "Why ProofShip",
    productLead:
      "Many AI contract tools ship “one prompt → generate → deploy.” ProofShip does something else: a machine gate between draft and chain. You bring the agent. The architecture is multi-chain, not one ecosystem.",
    feature1Title: "Bring your own agent",
    feature1Body:
      "ACP drives the CLI agent you already use: Claude Code, Codex, Grok, Cursor, OpenCode, and more. Your model, your subscription. No token tax.",
    feature2Title: "The gate decides",
    feature2Body:
      "Semantic checks, same-file theorem certification, build, exact-disk-closure inspect. Failing drafts emit zero artifacts.",
    feature3Title: "One source, many chains",
    feature3Body:
      "ProofForge materializes one program to EVM, Solana, NEAR, Noir, Aleo, and more. First deploy path includes X Layer. Built as a platform, not a single-ecosystem tool.",
    feature4Title: "Compute stays on your machine",
    feature4Body:
      "The agent and the gate run on your computer. This page is a remote panel. Deploy keys never enter the browser.",
    showcaseTitle: "What the product looks like",
    showcaseLead: "The agent and gate run on your computer. This page is the remote workspace — hit Get started to sign in.",
    showcaseCaption: "Top: the real desktop app. Below: the web workspace. Get started to sign in.",
    showcaseKicker: "Product demo",
    showcaseTabDesktop: "Desktop app",
    showcaseTabEmpty: "Web empty",
    showcaseTabSession: "Web in session",
    showcaseEnter: "Get started",
    showcaseHeroHint: "Web workspace",
    showcaseShotAlt: "ProofShip desktop workspace: session list, pairing card, and gate rail",
    showcaseShotHint: "Real desktop app · see the full demo",
    showcaseShotCaption: "Desktop ProofShip — the agent and gate run on this machine.",
    downloadTitle: "Download the desktop app",
    downloadLead:
      "The agent and gate must run on your machine. Install the desktop app, then drive it from this page. Installers ship with GitHub Releases.",
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
    howTitle: "How it works",
    howLead: "Natural language → your desktop agent writes Lean → machine gate → deploy only if it passes.",
    step1: "Describe",
    step1Body: "State intent and constraints in plain language.",
    step2: "Your agent drafts",
    step2Body: "Use the agent and model you already pay for. Emit structured ProgramV1.",
    step3: "Machine gate",
    step3Body: "check (incl. theorem cert) → build → inspect. Fail closed — zero artifacts.",
    step4: "Deploy",
    step4Body: "Only gate-passing contracts deploy. Materialization is multi-chain via ProofForge.",
    diffTitle: "Not a one-prompt full-stack generator",
    diffLead:
      "Some products generate frontend, backend, and contracts from one prompt and launch fast on a single chain. ProofShip takes a different path:",
    diff1Title: "Many chains, not one ecosystem",
    diff1Body:
      "One portable source, controlled compilation to many execution platforms. Not a template factory for a single chain.",
    diff2Title: "Bring your agent, no lock-in",
    diff2Body:
      "No forced platform agent. You keep your code agent and model subscriptions; we own the gate and the compiler pipeline.",
    diff3Title: "Gate first, then chain",
    diff3Body: "No gate pass means no artifacts and no deploy. Engineering-grade fail-closed.",
    diff4Title: "Pay for cloud, not a token tax",
    diff4Body:
      "Local and open paths do not monetize by locking models. If we host in the cloud, subscription covers real servers — not a mandatory AI credit meter.",
    tableColUs: "ProofShip",
    tableColThem: "Typical AI contract tools",
    tableRowScope: "Scope",
    tableRowScopeA: "Portable program + machine gate",
    tableRowScopeB: "Full-stack app from one prompt",
    tableRowChain: "Chains",
    tableRowChainA: "One source, many chains",
    tableRowChainB: "Often one ecosystem (e.g. Solana)",
    tableRowAgent: "Agent / model",
    tableRowAgentA: "BYO — your subscriptions, no lock-in",
    tableRowAgentB: "Built-in AI + credit meters",
    tableRowFormal: "Formal methods",
    tableRowFormalA: "Lean source + same-file theorem certification",
    tableRowFormalB: "Usually runnable code only",
    tableRowShip: "Ship rule",
    tableRowShipA: "Gate before artifacts",
    tableRowShipB: "Generate fast, iterate in chat",
    tableRowPay: "Paid for",
    tableRowPayA: "Optional cloud infra later",
    tableRowPayB: "Credits / AI generation usage",
    forgeKicker: "Kernel",
    forgeTitle: "Compiler kernel: ProofForge",
    forgeLead:
      "ProofShip uses ProofForge as the compiler and semantic kernel. One Lean program materializes to EVM, Solana, NEAR, Noir, and more. If semantics cannot be preserved, compilation is refused.",
    forgeCtaSite: "Open pf.grok.me",
    forgeCtaRepo: "ProofForge repo",
    forgeTargets: "Compile targets",
    forgeFail: "Refuse if semantics slip",
    forgeFailBody: "No downgrade. No “compile anyway.”",
    forgeLean: "Built on Lean 4",
    forgeLeanBody: "A multi-target compiler whose source can be reasoned about.",
    formalKicker: "Formal methods",
    formalTitle: "Contracts can carry their proofs",
    formalLead:
      "Not “emit Solidity and ship.” Programs live in Lean 4. Business logic, semantic requirements, and same-file theorems can sit in one source. The gate runs semantic checks and theorem certification. Artifacts exist only after that.",
    formal1Title: "Lean program source",
    formal1Body:
      "program … where describes state / init / entry / view. The source is type-checked and reason-able — not free-form paste.",
    formal2Title: "Same-file theorem certification",
    formal2Body:
      "Theorems and proofs can travel with the contract source. The gate checks them in the same repair loop.",
    formal3Title: "Refuse when meaning cannot hold",
    formal3Body:
      "The compiler derives requirements from source, then exact-matches the target’s claims. If semantics cannot be preserved, it refuses.",
    formal4Title: "Proofs stay on the pipeline",
    formal4Body:
      "The semantic layer keeps meaning fixed; materialization only changes encoding. deploy / prove / verify stay explicit.",
    formalNote:
      "Honesty: this is engineering-grade machine verification (semantic checks + same-file theorem certification). Formal lighthouse work is EVM-first. We do not claim completed formal proofs for every target, proven bytecode, or securities compliance.",
    pricingTitle: "Pricing",
    pricingLead:
      "Today: local-first, open toolchain. If we offer cloud sync / hosted gates later, subscription covers real servers and ops — not a locked model, not forced AI credits.",
    pricingLocalKicker: "Local",
    pricingLocalTitle: "Local / open",
    pricingLocalPrice: "Free",
    pricingLocalBody:
      "Run the agent and the gate on your machine. This page is only a remote panel. Keys and drafts stay with you.",
    pricingLocalF1: "Any code agent you choose",
    pricingLocalF2: "Machine gate + theorem certification",
    pricingLocalF3: "ProofForge multi-chain kernel",
    pricingCloudKicker: "Cloud",
    pricingCloudTitle: "Cloud hosted (planned)",
    pricingCloudPrice: "Subscription",
    pricingCloudBody:
      "Collaboration, sync, and hosted compute cost real servers. When we ship cloud, pay for infrastructure — still BYO agent and models.",
    pricingCloudF1: "Optional cloud sync & collaboration",
    pricingCloudF2: "Still BYO agent / model",
    pricingCloudF3: "Fees map to compute & storage, not a token tax",
    pricingNote: "Cloud plans will be published when ready. The local path stays.",
    honestyTitle: "Honesty boundary",
    honestyBody:
      "The gate is engineering-grade machine verification (semantic checks + same-file theorem certification). We do not claim full formal verification, proven bytecode, or securities compliance. Deploy keys never touch the page, app, or repository. We are not a one-click full-stack generator — the core is the gate, a portable program source, and proofs that travel with the contract.",
    ctaTitle: "Start on the web",
    ctaSub: "Sign in to the workspace, then attach desktop ProofShip.",
    footerTag: "BYO agent. Many chains. Gate before ship.",
    footerMit: "MIT License",
    loginTitle: "Sign in to the workspace",
    loginSub: "This identifies you. It never takes a deploy key.",
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
    studioTitle: "Workspace",
    studioWelcome: "Welcome back",
    studioLead:
      "Start from natural language, draft ProgramV1 via your desktop agent, then run the gate.",
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

function applyDocumentLocale(locale: Locale) {
  if (typeof document === "undefined") return;
  document.documentElement.lang = locale === "zh" ? "zh-CN" : "en";
  document.documentElement.dataset.locale = locale;
}

export function I18nProvider({ children }: { children: ReactNode }) {
  const [locale, setLocaleState] = useState<Locale>("zh");

  useEffect(() => {
    try {
      const saved = window.localStorage.getItem(STORAGE);
      if (saved === "en" || saved === "zh") {
        setLocaleState(saved);
        applyDocumentLocale(saved);
        return;
      }
    } catch {
      /* ignore */
    }
    applyDocumentLocale("zh");
  }, []);

  const setLocale = useCallback((l: Locale) => {
    setLocaleState(l);
    applyDocumentLocale(l);
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
