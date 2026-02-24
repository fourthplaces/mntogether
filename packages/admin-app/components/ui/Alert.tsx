import { cn } from "@/lib/utils";
import type { HTMLAttributes } from "react";

const variantStyles = {
  success: "bg-success-bg border-[#BBF7D0] text-success-text",
  error: "bg-danger-bg border-[#FECACA] text-danger-text",
  warning: "bg-warning-bg border-[#FDE68A] text-warning-text",
  info: "bg-info-bg border-[#BFDBFE] text-info-text",
};

type AlertProps = HTMLAttributes<HTMLDivElement> & {
  variant: keyof typeof variantStyles;
  title?: string;
};

export function Alert({
  variant,
  title,
  className,
  children,
  ...rest
}: AlertProps) {
  return (
    <div
      role="alert"
      className={cn(
        "p-3 border rounded-md text-sm",
        variantStyles[variant],
        className,
      )}
      {...rest}
    >
      {title && <p className="font-semibold mb-1">{title}</p>}
      {children}
    </div>
  );
}
