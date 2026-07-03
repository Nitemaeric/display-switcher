import * as React from "react";
import { Slot } from "@radix-ui/react-slot";
import { cva, type VariantProps } from "class-variance-authority";
import { cn } from "@/lib/utils";

const buttonVariants = cva(
  "inline-flex shrink-0 cursor-pointer items-center justify-center gap-3 rounded-lg border border-transparent box-border text-sm font-medium leading-none transition-colors focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-[var(--color-accent)] disabled:pointer-events-none disabled:opacity-50",
  {
    variants: {
      variant: {
        default:
          "border-transparent bg-[var(--color-accent)] text-white hover:bg-[var(--color-accent-hover)]",
        secondary:
          "border-[var(--color-card-border)] bg-transparent hover:bg-black/5 dark:hover:bg-white/5",
        ghost: "border-transparent hover:bg-black/5 dark:hover:bg-white/5",
        destructive:
          "border-transparent bg-rose-600 text-white hover:bg-rose-700",
      },
      size: {
        default: "h-10 min-h-10 px-3",
        sm: "h-8 min-h-8 rounded-md px-3 text-xs",
        icon: "h-10 min-h-10 w-10 min-w-10 p-0",
      },
    },
    defaultVariants: {
      variant: "default",
      size: "default",
    },
  },
);

export interface ButtonProps
  extends React.ButtonHTMLAttributes<HTMLButtonElement>,
    VariantProps<typeof buttonVariants> {
  asChild?: boolean;
}

export const Button = React.forwardRef<HTMLButtonElement, ButtonProps>(
  ({ className, variant, size, asChild = false, ...props }, ref) => {
    const Comp = asChild ? Slot : "button";
    return (
      <Comp
        className={cn(buttonVariants({ variant, size, className }))}
        ref={ref}
        {...props}
      />
    );
  },
);
Button.displayName = "Button";