import { cn } from "@/lib/utils";
import type { HTMLAttributes } from "react";

const variantStyles = {
  default: "bg-surface-raised rounded-lg border border-border",
  elevated: "bg-surface-raised rounded-lg shadow-card",
  interactive:
    "bg-surface-raised rounded-lg border border-border hover:shadow-card-hover hover:-translate-y-[1px] transition-all duration-200 cursor-pointer",
};

const paddingStyles = {
  none: "",
  sm: "p-4",
  md: "p-6",
  lg: "p-8",
};

type CardProps = HTMLAttributes<HTMLDivElement> & {
  variant?: keyof typeof variantStyles;
  padding?: keyof typeof paddingStyles;
};

export function Card({
  variant = "default",
  padding = "md",
  className,
  children,
  ...rest
}: CardProps) {
  return (
    <div
      className={cn(variantStyles[variant], paddingStyles[padding], className)}
      {...rest}
    >
      {children}
    </div>
  );
}
