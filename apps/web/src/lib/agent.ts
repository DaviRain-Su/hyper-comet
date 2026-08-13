import { extractLean } from "./gate";
import { TEMPLATES } from "./templates";

const SKILL = `You are the ProofShip drafting agent. Output ProofForge ProgramV1 (Lean DSL), not Solidity.

Output contract:
1. Exactly one .lean program. First line must be exactly: import ProofForgeV2
2. Fixed skeleton:
import ProofForgeV2
namespace Proofship
open ProofForgeV2.Language

program <Module> where
  -- ProgramV1 DSL only

end Proofship
3. Name <Module> from the domain (valid Lean identifier).
4. If required numeric parameters are missing, ask — do not invent. Prefer init/entry params.

Language surface ONLY:
- Types: UInt64, Principal, Bool (expression/return only), Map Principal UInt64, Option
- Statements: let / assignment / m[k] := v / return / assert <Bool> / revert ErrorName() / emit EventName(args) / if c then … else … / match e with | Option.some(x) => do … | _ => do …
- Expressions: + - * / %, compares, && || !, Map.empty(), context.caller, context.blockHeight
- Declarations: event E(amount : UInt64), error E() (parens required), init / entry / view

Forbidden: Bool as init/entry param; Map value not UInt64; error X without (); invariant/proof in deploy file; Solidity (mapping, function, pragma); String/Bytes state.

Reply with a short product-language explanation, then a single \`\`\`lean fence containing the full file.`;

export type DraftResult =
  | { ok: true; source: string; note: string; via: "grok" | "template" | "fallback" }
  | { ok: false; error: string; ask?: string };

function matchTemplate(prompt: string) {
  const p = prompt.toLowerCase();
  if (/(rwa|share registry|allowlist|份额|白名单)/.test(p)) return TEMPLATES[0];
  if (/(time.?lock|payout|vest|时间锁|领取)/.test(p)) return TEMPLATES[1];
  if (/(state.?cell|counter|increment|计数)/.test(p)) return TEMPLATES[2];
  return null;
}

function fallbackDraft(prompt: string): DraftResult {
  const tpl = matchTemplate(prompt);
  if (tpl) {
    return {
      ok: true,
      source: tpl.source,
      note: `Matched starter ${tpl.module}. Gate still has to pass before anything ships.`,
      via: "template",
    };
  }
  if (prompt.trim().length < 24) {
    return {
      ok: false,
      error: "Need a concrete contract spec.",
      ask: "Describe state, who can call what, and any caps or time windows. Or start from a template.",
    };
  }
  return {
    ok: true,
    source: `import ProofForgeV2

namespace Proofship

open ProofForgeV2.Language

program DraftedVault where
  state owner : Principal
  state held : UInt64

  event Deposited(amount : UInt64)
  event Withdrawn(amount : UInt64)

  error NotOwner()

  init() do
    owner := context.caller
    held := 0

  entry deposit(amount : UInt64) : UInt64 do
    held := held + amount
    emit Deposited(amount)
    return held

  entry withdraw(amount : UInt64) : UInt64 do
    assert context.caller == owner
    assert amount <= held
    held := held - amount
    emit Withdrawn(amount)
    return held

  view getHeld() : UInt64 do
    return held

end Proofship
`,
    note: "Drafted a minimal owner vault from your prompt. Review the rules, then run the gate.",
    via: "fallback",
  };
}

export async function draftProgram(prompt: string, prior?: string): Promise<DraftResult> {
  const apiKey = process.env.XAI_API_KEY;
  if (!apiKey) return fallbackDraft(prompt);

  const messages: { role: "system" | "user"; content: string }[] = [
    { role: "system", content: SKILL },
  ];
  if (prior) {
    messages.push({
      role: "user",
      content: `Previous draft (repair this if the user is iterating):\n\`\`\`lean\n${prior}\n\`\`\``,
    });
  }
  messages.push({ role: "user", content: prompt });

  try {
    const res = await fetch("https://api.x.ai/v1/chat/completions", {
      method: "POST",
      headers: {
        "Content-Type": "application/json",
        Authorization: `Bearer ${apiKey}`,
      },
      body: JSON.stringify({
        model: "grok-4.5",
        messages,
        max_tokens: 1800,
        temperature: 0.2,
      }),
    });
    if (!res.ok) return fallbackDraft(prompt);
    const body = (await res.json()) as {
      choices?: { message?: { content?: string } }[];
    };
    const text = body.choices?.[0]?.message?.content ?? "";
    const source = extractLean(text);
    if (!source) {
      if (/ask|need|missing|specify|请/.test(text.toLowerCase()) && text.length < 800) {
        return { ok: false, error: "Agent needs more spec.", ask: text.trim() };
      }
      return fallbackDraft(prompt);
    }
    const note = text.split("```")[0]?.trim() || "Drafted ProgramV1. Sending it through the gate.";
    return { ok: true, source, note, via: "grok" };
  } catch {
    return fallbackDraft(prompt);
  }
}
