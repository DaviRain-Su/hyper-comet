/**
 * ProofShip PlatformExecutor — gate-only relay client (`role=platform`).
 *
 * Connects to the Cloudflare relay like a UserExecutor, but never holds deploy
 * keys. Accepts `cmd.prompt` for ProgramV1 gate checks when `proof-forge-next`
 * is available; refuses `cmd.deploy` belt-and-suspenders with the relay.
 *
 * Env:
 * - PROOFSHIP_RELAY (required)
 * - PROOFSHIP_DEVICE_TOKEN | DEVICE_TOKEN | ENGINE_TOKEN
 * - PROOFSHIP_DEVICE_ID (default platform-1)
 * - PROOFSHIP_SESSION_ID | PROOFSHIP_LAUNCH_ID | LAUNCH_ID (default default)
 * - PROOF_FORGE_CLI | PF_CLI — optional absolute path to proof-forge-next
 */
import { spawn } from "node:child_process";
import { createHash } from "node:crypto";
import { mkdtemp, mkdir, writeFile, rm, readdir, readFile, stat } from "node:fs/promises";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { access, constants } from "node:fs/promises";
import WebSocket from "ws";

const MAX_TEXT = 4096;
const MAX_SOURCE = 64 * 1024;
const GATE_TIMEOUT_MS = 240_000;
const BUFFER_CAP = 200;

type CommandBase = { id?: string; chatId?: string };

type PromptCommand = CommandBase & {
  type: "cmd.prompt";
  nl: string;
  lane?: string;
  executor?: "user" | "platform";
};

type CancelCommand = CommandBase & { type: "cmd.cancel" };

type SteerCommand = CommandBase & { type: "cmd.steer"; nl: string };

type DeployCommand = CommandBase & {
  type: "cmd.deploy";
  networkId: string;
  module: string;
  digest?: string;
  executor?: "user" | "platform";
};

type RelayCommand = PromptCommand | CancelCommand | SteerCommand | DeployCommand;

interface ExtractedSource {
  module: string;
  source: string;
}

function env(name: string): string | undefined {
  const v = process.env[name];
  return v && v.trim() ? v.trim() : undefined;
}

function requiredRelayBase(): string {
  const base = env("PROOFSHIP_RELAY");
  if (!base) {
    console.error("PROOFSHIP_RELAY is required (Worker base URL, http(s) or ws(s))");
    process.exit(1);
  }
  return base.replace(/\/+$/u, "");
}

function deviceToken(): string {
  return (
    env("PROOFSHIP_DEVICE_TOKEN") ??
    env("DEVICE_TOKEN") ??
    env("ENGINE_TOKEN") ??
    ""
  );
}

function deviceId(): string {
  return env("PROOFSHIP_DEVICE_ID") ?? "platform-1";
}

function sessionId(): string {
  return (
    env("PROOFSHIP_SESSION_ID") ??
    env("PROOFSHIP_LAUNCH_ID") ??
    env("LAUNCH_ID") ??
    "default"
  );
}

function encodeQuery(s: string): string {
  return encodeURIComponent(s);
}

function socketUrl(base: string, session: string, token: string, device: string): string {
  let u = base.replace(/^https:/iu, "wss:").replace(/^http:/iu, "ws:");
  if (!/^wss?:/iu.test(u)) {
    // bare host → assume wss in prod-ish, ws for localhost
    u = u.includes("localhost") || u.includes("127.0.0.1") ? `ws://${u}` : `wss://${u}`;
  }
  return `${u}/ws/engine/${encodeQuery(session)}?token=${encodeQuery(token)}&deviceId=${encodeQuery(device)}&role=platform`;
}

function truncate(text: string, max = MAX_TEXT): string {
  return text.length <= max ? text : text.slice(0, max);
}

function isRecord(value: unknown): value is Record<string, unknown> {
  return typeof value === "object" && value !== null && !Array.isArray(value);
}

function validModule(module: string): boolean {
  return /^[A-Za-z][A-Za-z0-9_]{0,63}$/u.test(module);
}

function looksLikeProgramV1(source: string): boolean {
  const trimmed = source.trim();
  return (
    trimmed.startsWith("import ProofForgeV2") ||
    /\bimport\s+ProofForgeV2\b/u.test(trimmed)
  );
}

/** Pull ProgramV1 from a lean fence or bare source block; infer module. */
function extractSource(nl: string): ExtractedSource | null {
  const fence =
    /```(?:lean|lean4)?\s*\n([\s\S]*?)```/iu.exec(nl) ??
    /```\s*\n(import\s+ProofForgeV2[\s\S]*?)```/iu.exec(nl);

  let source: string | null = fence?.[1]?.trim() ?? null;
  if (!source && looksLikeProgramV1(nl)) {
    // Whole prompt is source (or source-dominant).
    const start = nl.search(/import\s+ProofForgeV2/u);
    if (start >= 0) source = nl.slice(start).trim();
  }
  if (!source || !looksLikeProgramV1(source)) return null;
  if (source.length > MAX_SOURCE) return null;

  const programMatch = /\bprogram\s+([A-Za-z][A-Za-z0-9_]*)\s+where\b/u.exec(source);
  const hintMatch =
    /(?:--module|--\s*module|module\s*[:=])\s*([A-Za-z][A-Za-z0-9_]*)/iu.exec(nl) ??
    /(?:^|\n)\s*Module\s*[:=]\s*([A-Za-z][A-Za-z0-9_]*)/iu.exec(nl);

  const module = programMatch?.[1] ?? hintMatch?.[1] ?? null;
  if (!module || !validModule(module)) return null;
  return { module, source };
}

async function pathExists(path: string): Promise<boolean> {
  try {
    await access(path, constants.F_OK);
    return true;
  } catch {
    return false;
  }
}

async function resolveCli(): Promise<string | null> {
  const explicit = env("PROOF_FORGE_CLI") ?? env("PF_CLI");
  if (explicit && (await pathExists(explicit))) return explicit;

  const pathEnv = process.env.PATH ?? "";
  for (const dir of pathEnv.split(":")) {
    if (!dir) continue;
    const candidate = join(dir, "proof-forge-next");
    if (await pathExists(candidate)) return candidate;
  }

  const forgeRoot = env("PROOF_FORGE_ROOT");
  if (forgeRoot) {
    const lake = join(forgeRoot, ".lake/build/bin/proof-forge-next");
    if (await pathExists(lake)) return lake;
  }

  // Walk up from cwd for vendored toolchain.
  let dir = process.cwd();
  for (let i = 0; i < 6; i++) {
    const vendored = join(dir, "proofship/toolchain/bin/proof-forge-next");
    if (await pathExists(vendored)) return vendored;
    const parent = join(dir, "..");
    if (parent === dir) break;
    dir = parent;
  }
  return null;
}

function runProcess(
  cli: string,
  args: string[],
  cwd: string,
  timeoutMs: number,
): Promise<{ code: number; stdout: string; stderr: string }> {
  return new Promise((resolve) => {
    const child = spawn(cli, args, {
      cwd,
      env: process.env,
      stdio: ["ignore", "pipe", "pipe"],
    });
    let stdout = "";
    let stderr = "";
    const timer = setTimeout(() => {
      child.kill("SIGKILL");
    }, timeoutMs);
    child.stdout?.on("data", (chunk: Buffer) => {
      stdout += chunk.toString("utf8");
      if (stdout.length > 256_000) stdout = stdout.slice(-256_000);
    });
    child.stderr?.on("data", (chunk: Buffer) => {
      stderr += chunk.toString("utf8");
      if (stderr.length > 256_000) stderr = stderr.slice(-256_000);
    });
    child.on("close", (code) => {
      clearTimeout(timer);
      resolve({ code: code ?? 1, stdout, stderr });
    });
    child.on("error", (err) => {
      clearTimeout(timer);
      resolve({ code: 1, stdout, stderr: String(err) });
    });
  });
}

async function listOutFiles(outDir: string): Promise<{ name: string; size: number }[]> {
  try {
    const names = await readdir(outDir);
    const files: { name: string; size: number }[] = [];
    for (const name of names) {
      const s = await stat(join(outDir, name));
      if (s.isFile()) files.push({ name, size: s.size });
    }
    files.sort((a, b) => a.name.localeCompare(b.name));
    return files;
  } catch {
    return [];
  }
}

async function readAbiJson(
  outDir: string,
  module: string,
): Promise<unknown | undefined> {
  const candidates = [
    `${module}.abi.json`,
    `${module.toLowerCase()}.abi.json`,
  ];
  for (const name of candidates) {
    try {
      const raw = await readFile(join(outDir, name), "utf8");
      return JSON.parse(raw) as unknown;
    } catch {
      /* try next */
    }
  }
  try {
    const names = await readdir(outDir);
    const hit = names.find((n) => n.endsWith(".abi.json"));
    if (!hit) return undefined;
    return JSON.parse(await readFile(join(outDir, hit), "utf8")) as unknown;
  } catch {
    return undefined;
  }
}

function parseOutputSetDigest(inspectOut: string): string | undefined {
  const m =
    /outputSetDigest\s+([0-9a-fA-F]{16,})/u.exec(inspectOut) ??
    /"outputSetDigest"\s*:\s*"([0-9a-fA-F]+)"/u.exec(inspectOut);
  return m?.[1];
}

async function runGatePipeline(
  cli: string,
  extracted: ExtractedSource,
): Promise<{
  ok: boolean;
  files: { name: string; size: number }[];
  abi?: unknown;
  digests?: { sourceSha256: string; outputSetDigest?: string };
  output?: string;
  error?: string;
  stage?: string;
}> {
  const work = await mkdtemp(join(tmpdir(), "proofship-platform-"));
  try {
    const inbox = join(work, "studio-inbox");
    await mkdir(inbox, { recursive: true });
    const fileName = `${extracted.module}.lean`;
    const absFile = join(inbox, fileName);
    await writeFile(absFile, extracted.source, "utf8");
    const rel = `studio-inbox/${fileName}`;
    const outRel = `studio-inbox/out-${extracted.module.toLowerCase()}`;
    const outDir = join(work, outRel);
    const sourceSha256 = createHash("sha256").update(extracted.source, "utf8").digest("hex");

    const check = await runProcess(
      cli,
      ["check", rel, "--module", extracted.module, "--root", work],
      work,
      GATE_TIMEOUT_MS,
    );
    if (check.code !== 0) {
      return {
        ok: false,
        stage: "check",
        files: [{ name: fileName, size: extracted.source.length }],
        digests: { sourceSha256 },
        output: truncate(`${check.stdout}\n${check.stderr}`.trim(), 8000),
        error: `proof-forge-next check exited ${check.code}`,
      };
    }

    const build = await runProcess(
      cli,
      [
        "build",
        rel,
        "--module",
        extracted.module,
        "--target",
        "evm",
        "-o",
        outRel,
        "--root",
        work,
      ],
      work,
      GATE_TIMEOUT_MS,
    );
    if (build.code !== 0) {
      return {
        ok: false,
        stage: "build",
        files: [{ name: fileName, size: extracted.source.length }],
        digests: { sourceSha256 },
        output: truncate(`${build.stdout}\n${build.stderr}`.trim(), 8000),
        error: `proof-forge-next build exited ${build.code}`,
      };
    }

    const inspect = await runProcess(
      cli,
      ["inspect", "--output-dir", outRel, "--root", work],
      work,
      GATE_TIMEOUT_MS,
    );
    const inspectText = `${inspect.stdout}\n${inspect.stderr}`.trim();
    if (inspect.code !== 0) {
      return {
        ok: false,
        stage: "inspect",
        files: await listOutFiles(outDir),
        digests: { sourceSha256 },
        output: truncate(inspectText, 8000),
        error: `proof-forge-next inspect exited ${inspect.code}`,
      };
    }

    const outputSetDigest = parseOutputSetDigest(inspectText);
    const files = await listOutFiles(outDir);
    const abi = await readAbiJson(outDir, extracted.module);
    return {
      ok: true,
      stage: "inspect",
      files,
      abi,
      digests: { sourceSha256, outputSetDigest },
      output: truncate(inspectText, 8000),
    };
  } finally {
    await rm(work, { recursive: true, force: true }).catch(() => undefined);
  }
}

class PlatformExecutor {
  private readonly base: string;
  private readonly token: string;
  private readonly device: string;
  private readonly session: string;
  private ws: WebSocket | null = null;
  private outbound: string[] = [];
  private backoffMs = 1000;
  private closed = false;
  private handling = Promise.resolve();

  constructor() {
    this.base = requiredRelayBase();
    this.token = deviceToken();
    this.device = deviceId();
    this.session = sessionId();
  }

  start(): void {
    console.info(
      `[platform] session=${this.session} device=${this.device} relay=${this.base}`,
    );
    void this.connectLoop();
  }

  stop(): void {
    this.closed = true;
    this.ws?.close();
  }

  private publish(kind: string, payload: unknown): void {
    this.send({ type: "event", kind, payload });
  }

  private note(text: string): void {
    this.publish("note", { text: truncate(text) });
  }

  private ack(id: string): void {
    this.send({ type: "cmd.ack", id });
  }

  private send(data: unknown): void {
    const msg = JSON.stringify(data);
    const ws = this.ws;
    if (ws && ws.readyState === WebSocket.OPEN) {
      ws.send(msg);
      return;
    }
    this.outbound.push(msg);
    while (this.outbound.length > BUFFER_CAP) this.outbound.shift();
  }

  private async connectLoop(): Promise<void> {
    while (!this.closed) {
      const url = socketUrl(this.base, this.session, this.token, this.device);
      try {
        await this.oneConnection(url);
      } catch (err) {
        console.warn("[platform] connection error:", err);
      }
      if (this.closed) break;
      console.warn(`[platform] reconnecting in ${this.backoffMs}ms`);
      await sleep(this.backoffMs);
      this.backoffMs = Math.min(this.backoffMs * 2, 30_000);
    }
  }

  private oneConnection(url: string): Promise<void> {
    return new Promise((resolve) => {
      const ws = new WebSocket(url);
      this.ws = ws;

      ws.on("open", () => {
        console.info("[platform] connected (role=platform)");
        this.backoffMs = 1000;
        for (const msg of this.outbound.splice(0)) {
          ws.send(msg);
        }
      });

      ws.on("message", (data) => {
        const text = typeof data === "string" ? data : data.toString("utf8");
        this.handling = this.handling
          .then(() => this.onMessage(text))
          .catch((err) => console.warn("[platform] handler error:", err));
      });

      ws.on("close", () => {
        if (this.ws === ws) this.ws = null;
        resolve();
      });

      ws.on("error", (err) => {
        console.warn("[platform] ws error:", err.message);
      });
    });
  }

  private async onMessage(text: string): Promise<void> {
    let raw: unknown;
    try {
      raw = JSON.parse(text) as unknown;
    } catch {
      return;
    }
    const cmd = parseCommand(raw);
    if (!cmd) return;

    if (cmd.id) this.ack(cmd.id);

    switch (cmd.type) {
      case "cmd.prompt":
        await this.handlePrompt(cmd);
        break;
      case "cmd.deploy":
        this.handleDeploy(cmd);
        break;
      case "cmd.cancel":
        this.note(
          cmd.chatId
            ? `cancel acknowledged (chatId=${cmd.chatId}); platform gate has nothing to interrupt`
            : "cancel acknowledged; platform gate has nothing to interrupt",
        );
        this.publish("session.done", {
          ok: true,
          status: "cancelled",
          hint: "PlatformExecutor is gate-only; cancel is a no-op when idle",
        });
        break;
      case "cmd.steer":
        this.note(`steer ignored on platform: ${truncate(cmd.nl, 200)}`);
        this.publish("session.agent", {
          text: "PlatformExecutor does not run a full agent loop; steer is ignored. Provide ProgramV1 source for gate, or use UserExecutor.",
        });
        this.publish("session.done", {
          ok: false,
          status: "steer_unsupported",
          hint: "use UserExecutor for interactive agent steering",
        });
        break;
    }
  }

  private handleDeploy(cmd: DeployCommand): void {
    this.publish("executor.refused", {
      reason: "platform_executor_cannot_hold_deploy_keys",
      hint: "Connect a user desktop/VPS executor and deploy there (wallet or DevEnvKey). Keys never live on platform.",
      networkId: cmd.networkId,
      module: cmd.module,
    });
  }

  private async handlePrompt(cmd: PromptCommand): Promise<void> {
    const nl = truncate(cmd.nl, 4000);
    this.publish("session.user", {
      text: nl,
      chatId: cmd.chatId,
      lane: cmd.lane,
      executor: "platform",
    });

    const extracted = extractSource(nl);
    const cli = await resolveCli();

    if (extracted && cli) {
      this.publish("session.agent", {
        text: `Platform: running check → build → inspect for module ${extracted.module}.`,
      });
      this.publish("gate.start", { module: extracted.module, phase: "gate" });
      const result = await runGatePipeline(cli, extracted);
      this.publish("gate.done", {
        ok: result.ok,
        module: extracted.module,
        stage: result.stage,
        digests: result.digests?.outputSetDigest
          ? {
              outputSetDigest: result.digests.outputSetDigest,
              sourceSha256: result.digests.sourceSha256,
              certified: result.ok,
            }
          : result.digests,
        output: result.output,
        error: result.error,
      });
      if (result.ok) {
        this.publish("artifact.sealed", {
          module: extracted.module,
          outputSetDigest: result.digests?.outputSetDigest,
          abi: result.abi,
          files: result.files,
          digests: result.digests,
          honesty:
            "Platform gate seal (check/build/inspect). Deploy still requires UserExecutor; keys never on platform.",
        });
      }
      this.publish("session.done", {
        ok: result.ok,
        status: result.ok ? "gate_ok" : "gate_failed",
        module: extracted.module,
        stage: result.stage,
        error: result.error,
      });
      return;
    }

    if (extracted && !cli) {
      this.publish("session.agent", {
        text: "Platform scaffold accepted a ProgramV1-shaped job, but proof-forge-next is not on PATH (set PROOF_FORGE_CLI). Gate skipped.",
      });
      this.publish("session.done", {
        ok: false,
        status: "cli_missing",
        module: extracted.module,
        hint: "Install proof-forge-next or set PROOF_FORGE_CLI / bake CLI into the Sandbox image",
      });
      return;
    }

    // NL without extractable Lean source — honest scaffold response.
    this.publish("session.agent", {
      text: "Platform scaffold accepts the job and runs gate when proof-forge-next is present and the prompt includes ProgramV1 source (lean fence or import ProofForgeV2). Full NL→agent drafting lives on UserExecutor.",
    });
    this.publish("session.done", {
      ok: false,
      status: "needs_source",
      hint: "provide ProgramV1 source or use UserExecutor for full agent",
    });
  }
}

function parseCommand(raw: unknown): RelayCommand | null {
  if (!isRecord(raw) || typeof raw.type !== "string") return null;
  const id = typeof raw.id === "string" ? raw.id : undefined;
  const chatId = typeof raw.chatId === "string" ? raw.chatId : undefined;

  switch (raw.type) {
    case "cmd.prompt": {
      if (typeof raw.nl !== "string" || !raw.nl.trim()) return null;
      const cmd: PromptCommand = { type: "cmd.prompt", nl: raw.nl, id, chatId };
      if (typeof raw.lane === "string") cmd.lane = raw.lane;
      if (raw.executor === "user" || raw.executor === "platform") cmd.executor = raw.executor;
      return cmd;
    }
    case "cmd.cancel":
      return { type: "cmd.cancel", id, chatId };
    case "cmd.steer": {
      if (typeof raw.nl !== "string" || !raw.nl.trim()) return null;
      return { type: "cmd.steer", nl: raw.nl, id, chatId };
    }
    case "cmd.deploy": {
      if (typeof raw.networkId !== "string" || typeof raw.module !== "string") return null;
      const cmd: DeployCommand = {
        type: "cmd.deploy",
        networkId: raw.networkId,
        module: raw.module,
        id,
        chatId,
      };
      if (typeof raw.digest === "string") cmd.digest = raw.digest;
      if (raw.executor === "user" || raw.executor === "platform") cmd.executor = raw.executor;
      return cmd;
    }
    default:
      return null;
  }
}

function sleep(ms: number): Promise<void> {
  return new Promise((r) => setTimeout(r, ms));
}

const executor = new PlatformExecutor();
executor.start();

for (const sig of ["SIGINT", "SIGTERM"] as const) {
  process.on(sig, () => {
    console.info(`[platform] ${sig}, shutting down`);
    executor.stop();
    process.exit(0);
  });
}
