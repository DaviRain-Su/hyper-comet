import { Slot } from "@radix-ui/react-slot";
import { cva, type VariantProps } from "class-variance-authority";
import type { ComponentProps } from "react";
import { cn } from "@/lib/cn";

const buttonVariants = cva(
  "inline-flex items-center justify-center gap-2 whitespace-nowrap rounded-lg text-[13.5px] font-medium transition-colors duration-150 disabled:pointer-events-none disabled:opacity-45 [&_svg]:pointer-events-none [&_svg]:size-4 [&_svg]:shrink-0",
  {
    variants: {
      variant: {
        default: "bg-purple text-white hover:bg-purple-hi",
        ghost:
          "bg-transparent text-dim border border-line hover:text-ink hover:border-faint",
        subtle: "bg-raise text-ink border border-line hover:border-faint",
        link: "text-dim hover:text-ink underline-offset-4 hover:underline px-0",
      },
      size: {
        default: "h-10 px-[18px] py-2.5",
        sm: "h-8 px-3.5 text-[11.5px]",
        lg: "h-11 px-5",
        icon: "size-9",
      },
    },
    defaultVariants: { variant: "default", size: "default" },
  },
);

export function Button({
  className,
  variant,
  size,
  asChild = false,
  ...props
}: ComponentProps<"button"> &
  VariantProps<typeof buttonVariants> & { asChild?: boolean }) {
  const Comp = asChild ? Slot : "button";
  return <Comp className={cn(buttonVariants({ variant, size, className }))} {...props} />;
}
