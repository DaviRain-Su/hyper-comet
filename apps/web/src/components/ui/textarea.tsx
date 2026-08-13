import * as React from "react";
import { cn } from "@/lib/cn";

export function Textarea({ className, ...props }: React.ComponentProps<"textarea">) {
  return (
    <textarea
      className={cn(
        "flex min-h-20 w-full rounded-lg border border-line bg-bg px-3 py-2.5 text-sm text-ink",
        "placeholder:text-faint outline-none transition-colors resize-none",
        "focus-visible:border-purple/50 focus-visible:ring-2 focus-visible:ring-purple/25",
        "disabled:cursor-not-allowed disabled:opacity-50",
        className,
      )}
      {...props}
    />
  );
}
