import { useLocale } from "@/lib/i18n";
import { cn } from "@/lib/cn";

export function LocaleToggle({ className }: { className?: string }) {
  const { locale, setLocale } = useLocale();
  return (
    <div
      className={cn(
        "inline-flex items-center gap-0.5 rounded-[var(--radius-md)] border border-border bg-bg p-0.5",
        className,
      )}
      role="group"
      aria-label="Language"
    >
      <button
        type="button"
        onClick={() => setLocale("en")}
        className={cn(
          "rounded-md px-2 py-1 text-[12px] font-semibold tracking-wide transition-colors",
          locale === "en" ? "bg-white/10 text-fg" : "text-fg-subtle hover:text-fg",
        )}
      >
        EN
      </button>
      <button
        type="button"
        onClick={() => setLocale("zh")}
        className={cn(
          "rounded-md px-2 py-1 text-[12px] font-semibold tracking-wide transition-colors",
          locale === "zh" ? "bg-white/10 text-fg" : "text-fg-subtle hover:text-fg",
        )}
      >
        中文
      </button>
    </div>
  );
}
