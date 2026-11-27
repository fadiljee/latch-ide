import type { ButtonHTMLAttributes } from "react";
import { cn } from "../../lib/cn";

interface IconButtonProps extends ButtonHTMLAttributes<HTMLButtonElement> {
  label: string;
  active?: boolean;
}

export function IconButton({
  label,
  active = false,
  className,
  type = "button",
  children,
  ...rest
}: IconButtonProps) {
  return (
    <button
      type={type}
      aria-label={label}
      title={label}
      className={cn(
        "inline-flex h-7 w-7 items-center justify-center rounded-sm border transition-colors",
        active
          ? "border-accent/50 bg-accent-soft text-accent"
          : "border-transparent text-fg-2 hover:bg-hover hover:text-fg",
        className,
      )}
      {...rest}
    >
      {children}
    </button>
  );
}
