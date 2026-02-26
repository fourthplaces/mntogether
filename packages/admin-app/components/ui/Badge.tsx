import { cn } from "@/lib/utils";
import type { HTMLAttributes } from "react";

const variantStyles = {
  default: "bg-surface-muted text-text-secondary",
  success: "bg-success-bg text-success-text",
  warning: "bg-warning-bg text-warning-text",
  danger: "bg-danger-bg text-danger-text",
  info: "bg-info-bg text-info-text",
  spotlight: "bg-[#F3E8FF] text-[#6B21A8]",
};

const sizeStyles = {
  sm: "px-2 py-0.5 text-xs",
  md: "px-3 py-1 text-xs",
};

type Variant = keyof typeof variantStyles;
type Size = keyof typeof sizeStyles;

type BadgeProps = HTMLAttributes<HTMLSpanElement> & {
  variant?: Variant;
  size?: Size;
  pill?: boolean;
  /** Pass a hex color for dynamic tag styling (e.g. from API). Overrides variant colors. */
  color?: string;
};

export function Badge({
  variant = "default",
  size = "sm",
  pill = true,
  color,
  className,
  children,
  ...rest
}: BadgeProps) {
  const classes = cn(
    "inline-flex items-center font-medium tracking-wide",
    pill ? "rounded-full" : "rounded-sm",
    !color && variantStyles[variant],
    !color && sizeStyles[size],
    color && sizeStyles[size],
    className,
  );

  const dynamicStyle = color
    ? { backgroundColor: color + "20", color: color }
    : undefined;

  return (
    <span className={classes} style={dynamicStyle} {...rest}>
      {children}
    </span>
  );
}
