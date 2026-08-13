import { useLocale } from "@/lib/i18n";
import { cn } from "@/lib/cn";

export function LocaleToggle({ className }: { className?: string }) {
  const { locale, setLocale } = useLocale();
  return (
    <div className={cn("inline-flex items-center gap-1 text-[13px]", className)}>
      <button
        type="button"
        onClick={() => setLocale("en")}
        className={cn(
          "px-1.5 py-1 transition-colors",
          locale === "en" ? "text-ink" : "text-faint hover:text-dim",
        )}
      >
        EN
      </button>
      <span className="text-faint/50">/</span>
      <button
        type="button"
        onClick={() => setLocale("zh")}
        className={cn(
          "px-1.5 py-1 transition-colors",
          locale === "zh" ? "text-ink" : "text-faint hover:text-dim",
        )}
      >
        中文
      </button>
    </div>
  );
}
