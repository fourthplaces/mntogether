import { cn } from "@/lib/utils";
import { forwardRef, type InputHTMLAttributes, type TextareaHTMLAttributes } from "react";

type InputProps = InputHTMLAttributes<HTMLInputElement> & {
  error?: boolean | string;
};

export const Input = forwardRef<HTMLInputElement, InputProps>(
  ({ error, className, ...rest }, ref) => {
    return (
      <div className="w-full">
        <input
          ref={ref}
          className={cn(
            "w-full px-4 py-2.5 text-sm bg-surface-subtle border rounded-md",
            "placeholder:text-text-faint",
            "focus:outline-none focus:ring-2 focus:border-transparent transition-all duration-150",
            error
              ? "border-danger-hover focus:ring-danger-hover"
              : "border-border focus:ring-focus-ring",
            rest.disabled && "opacity-50 cursor-not-allowed",
            className,
          )}
          {...rest}
        />
        {typeof error === "string" && error && (
          <p className="mt-1 text-xs text-danger-text">{error}</p>
        )}
      </div>
    );
  },
);

Input.displayName = "Input";

type TextareaProps = TextareaHTMLAttributes<HTMLTextAreaElement> & {
  error?: boolean | string;
};

export const Textarea = forwardRef<HTMLTextAreaElement, TextareaProps>(
  ({ error, className, ...rest }, ref) => {
    return (
      <div className="w-full">
        <textarea
          ref={ref}
          className={cn(
            "w-full px-4 py-3 text-sm bg-surface-subtle border rounded-md resize-none",
            "placeholder:text-text-faint",
            "focus:outline-none focus:ring-2 focus:border-transparent transition-all duration-150",
            error
              ? "border-danger-hover focus:ring-danger-hover"
              : "border-border focus:ring-focus-ring",
            rest.disabled && "opacity-50 cursor-not-allowed",
            className,
          )}
          {...rest}
        />
        {typeof error === "string" && error && (
          <p className="mt-1 text-xs text-danger-text">{error}</p>
        )}
      </div>
    );
  },
);

Textarea.displayName = "Textarea";
