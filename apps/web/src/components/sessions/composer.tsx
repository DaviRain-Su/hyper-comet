import { useState } from "react";
import { ArrowUp } from "lucide-react";
import { Textarea } from "@/components/ui/textarea";
import { pick, useLocale } from "@/lib/i18n";

export function Composer({
  disabled,
  onSend,
}: {
  disabled?: boolean;
  onSend: (text: string) => void;
}) {
  const { locale } = useLocale();
  const [value, setValue] = useState("");

  const submit = () => {
    const t = value.trim();
    if (!t || disabled) return;
    onSend(t);
    setValue("");
  };

  return (
    <div className="border-t border-line bg-bg/80 px-4 py-3 backdrop-blur-sm sm:px-6">
      <div className="mx-auto flex max-w-[720px] items-end gap-2 rounded-2xl border border-line bg-raise p-2 pl-3">
        <Textarea
          value={value}
          disabled={disabled}
          rows={2}
          placeholder={pick(
            locale,
            "Describe the contract in plain language…",
            "用自然语言描述合约…",
          )}
          className="min-h-[52px] flex-1 border-0 bg-transparent px-0 py-2 focus-visible:ring-0"
          onChange={(e) => setValue(e.target.value)}
          onKeyDown={(e) => {
            if (e.key === "Enter" && !e.shiftKey) {
              e.preventDefault();
              submit();
            }
          }}
        />
        <button
          type="button"
          disabled={disabled || !value.trim()}
          onClick={submit}
          className="grid size-10 shrink-0 place-items-center rounded-xl bg-purple text-white hover:bg-purple-hi disabled:opacity-40"
          aria-label={pick(locale, "Send", "发送")}
        >
          <ArrowUp className="size-4" />
        </button>
      </div>
      <p className="mx-auto mt-2 max-w-[720px] px-1 text-[11px] text-faint">
        {pick(
          locale,
          "Web companion drafts and gates. Deploy keys stay on your desktop / wallet.",
          "Web 端负责起草与门禁。部署密钥留在桌面端 / 钱包。",
        )}
      </p>
    </div>
  );
}
