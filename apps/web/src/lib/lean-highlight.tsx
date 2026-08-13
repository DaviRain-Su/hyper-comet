import { cn } from "@/lib/cn";

const KEYWORDS = new Set([
  "program",
  "where",
  "state",
  "entry",
  "import",
  "namespace",
  "open",
  "init",
  "view",
  "event",
  "error",
  "do",
  "return",
  "assert",
  "end",
  "match",
  "with",
  "then",
  "if",
]);

export function LeanCode({
  source,
  className,
  maxLines,
}: {
  source: string;
  className?: string;
  maxLines?: number;
}) {
  const body = maxLines ? source.split("\n").slice(0, maxLines).join("\n") : source;
  const tokens = body.split(/(\b)/);
  return (
    <pre className={cn("overflow-auto font-mono text-[12px] leading-relaxed text-fg-muted", className)}>
      {tokens.map((token, i) =>
        KEYWORDS.has(token) ? (
          <span key={`${i}-${token}`} className="text-accent">
            {token}
          </span>
        ) : (
          <span key={`${i}-${token}`}>{token}</span>
        ),
      )}
    </pre>
  );
}
