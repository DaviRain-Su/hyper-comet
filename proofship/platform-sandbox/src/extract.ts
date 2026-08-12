/**
 * Pure ProgramV1 extraction + module-name validation (no I/O, no relay).
 */

export const MAX_SOURCE = 64 * 1024;

export interface ExtractedSource {
  module: string;
  source: string;
}

export function validModule(module: string): boolean {
  return /^[A-Za-z][A-Za-z0-9_]{0,63}$/u.test(module);
}

export function looksLikeProgramV1(source: string): boolean {
  const trimmed = source.trim();
  return (
    trimmed.startsWith("import ProofForgeV2") ||
    /\bimport\s+ProofForgeV2\b/u.test(trimmed)
  );
}

/** Pull ProgramV1 from a lean fence or bare source block; infer module. */
export function extractSource(nl: string): ExtractedSource | null {
  const fence =
    /```(?:lean|lean4)?\s*\n([\s\S]*?)```/iu.exec(nl) ??
    /```\s*\n(import\s+ProofForgeV2[\s\S]*?)```/iu.exec(nl);

  let source: string | null = fence?.[1]?.trim() ?? null;
  if (!source && looksLikeProgramV1(nl)) {
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
