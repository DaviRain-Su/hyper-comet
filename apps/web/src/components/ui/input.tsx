import * as React from "react";
import { cn } from "@/lib/cn";

export function Input({ className, type, ...props }: React.ComponentProps<"input">) {
  return (
    <input
      type={type}
      className={cn(
        "flex h-10 w-full rounded-lg border border-line bg-bg px-3 text-sm text-ink",
        "placeholder:text-faint outline-none transition-colors",
        "focus-visible:border-purple/50 focus-visible:ring-2 focus-visible:ring-purple/25",
        "disabled:cursor-not-allowed disabled:opacity-50",
        className,
      )}
      {...props}
    />
  );
}
