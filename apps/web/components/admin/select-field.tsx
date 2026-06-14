"use client";

import * as React from "react";
import { cn } from "@/lib/utils";

interface SelectFieldProps extends React.SelectHTMLAttributes<HTMLSelectElement> {
  label?: string;
  options: { value: string; label: string }[];
}

export const SelectField = React.forwardRef<HTMLSelectElement, SelectFieldProps>(
  ({ label, options, className, ...props }, ref) => {
    return (
      <div className="space-y-1.5">
        {label && <label className="text-sm font-medium text-text-secondary">{label}</label>}
        <select
          ref={ref}
          className={cn(
            "flex h-10 w-full rounded-radius-sm border border-border-subtle bg-bg-secondary px-3 py-2 text-sm text-text-primary focus-visible:outline-none focus-visible:border-text-tertiary disabled:cursor-not-allowed disabled:opacity-50",
            className
          )}
          {...props}
        >
          {options.map((o) => (
            <option key={o.value} value={o.value}>
              {o.label}
            </option>
          ))}
        </select>
      </div>
    );
  }
);
SelectField.displayName = "SelectField";
