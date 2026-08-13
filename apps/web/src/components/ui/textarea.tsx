import * as React from "react";
import { cn } from "@/lib/cn";

export function Textarea({ className, ...props }: React.ComponentProps<"textarea">) {
  return (
    <textarea
      className={cn(
        "flex min-h-20 w-full resize-none rounded-lg border border-border bg-bg px-3 py-2.5 text-sm text-fg",
        "placeholder:text-fg-subtle outline-none transition-colors",
        "focus-visible:border-accent/50 focus-visible:ring-2 focus-visible:ring-accent/25",
        "disabled:cursor-not-allowed disabled:opacity-50",
        className,
      )}
      {...props}
    />
  );
}
