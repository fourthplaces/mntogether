import Link from "next/link";
import { cn } from "@/lib/utils";
import type { ButtonHTMLAttributes, AnchorHTMLAttributes } from "react";

const variantStyles = {
  primary:
    "bg-action text-text-on-action hover:bg-action-hover",
  secondary:
    "bg-transparent text-text-secondary border border-border-strong hover:border-action hover:text-text-primary",
  danger:
    "bg-danger text-white hover:bg-danger-hover",
  success:
    "bg-success text-white hover:bg-success-hover",
  admin:
    "bg-admin-accent text-white hover:bg-admin-accent-hover",
  ghost:
    "bg-transparent text-text-secondary hover:text-text-primary hover:bg-surface-muted",
};

const sizeStyles = {
  sm: "px-3 py-1.5 text-xs",
  md: "px-5 py-2.5 text-sm",
  lg: "px-7 py-3 text-base",
};

type Variant = keyof typeof variantStyles;
type Size = keyof typeof sizeStyles;

type BaseProps = {
  variant?: Variant;
  size?: Size;
  pill?: boolean;
  loading?: boolean;
  className?: string;
};

type ButtonAsButton = BaseProps &
  Omit<ButtonHTMLAttributes<HTMLButtonElement>, keyof BaseProps> & {
    href?: never;
  };

type ButtonAsLink = BaseProps &
  Omit<AnchorHTMLAttributes<HTMLAnchorElement>, keyof BaseProps> & {
    href: string;
  };

type ButtonProps = ButtonAsButton | ButtonAsLink;

export function Button(props: ButtonProps) {
  const {
    variant = "primary",
    size = "md",
    pill = false,
    loading = false,
    className,
    ...rest
  } = props;

  const classes = cn(
    "inline-flex items-center justify-center font-semibold transition-colors focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-focus-ring focus-visible:ring-offset-2 focus-visible:ring-offset-surface",
    pill ? "rounded-full" : "rounded-md",
    variantStyles[variant],
    sizeStyles[size],
    (rest as ButtonAsButton).disabled && "opacity-50 cursor-not-allowed",
    className,
  );

  if ("href" in rest && rest.href) {
    const { href, ...linkRest } = rest as ButtonAsLink;
    return (
      <Link href={href} className={classes} {...linkRest} />
    );
  }

  const { disabled, ...buttonRest } = rest as ButtonAsButton;
  return (
    <button
      className={classes}
      disabled={disabled || loading}
      {...buttonRest}
    >
      {loading && (
        <svg
          className="animate-spin -ml-1 mr-2 h-4 w-4"
          xmlns="http://www.w3.org/2000/svg"
          fill="none"
          viewBox="0 0 24 24"
        >
          <circle
            className="opacity-25"
            cx="12"
            cy="12"
            r="10"
            stroke="currentColor"
            strokeWidth="4"
          />
          <path
            className="opacity-75"
            fill="currentColor"
            d="M4 12a8 8 0 018-8V0C5.373 0 0 5.373 0 12h4z"
          />
        </svg>
      )}
      {(rest as ButtonAsButton).children}
    </button>
  );
}
