import type { ReactNode } from "react";
import { Label } from "@/components/ui/label";

interface FieldWrapperProps {
  label: string;
  hint?: string;
  error?: string;
  required?: boolean;
  children: ReactNode;
  className?: string;
}

export function FieldWrapper({
  label,
  hint,
  error,
  required,
  children,
  className,
}: FieldWrapperProps) {
  return (
    <div className={className ?? "mb-6"}>
      <div className="flex items-baseline justify-between mb-1.5">
        <Label className="text-xs font-semibold uppercase tracking-wider text-text-label">
          {label}
          {required && <span className="text-danger ml-0.5">*</span>}
        </Label>
        {error && (
          <span className="text-xs text-danger-text">{error}</span>
        )}
      </div>
      {children}
      {hint && !error && (
        <p className="mt-1 text-xs text-text-muted">{hint}</p>
      )}
    </div>
  );
}
