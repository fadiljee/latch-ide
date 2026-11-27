import type { ButtonHTMLAttributes } from "react";
import { cn } from "../../lib/cn";

export type ButtonVariant = "primary" | "secondary" | "ghost" | "danger";
export type ButtonSize = "sm" | "md";

interface ButtonProps extends ButtonHTMLAttributes<HTMLButtonElement> {
  variant?: ButtonVariant;
  size?: ButtonSize;
}

const VARIANT_CLASSES: Record<ButtonVariant, string> = {
  primary: "bg-accent text-on-accent hover:opacity-90 border border-transparent",
  secondary: "bg-raised text-fg border border-line hover:bg-hover",
  ghost: "bg-transparent text-fg-2 border border-transparent hover:bg-hover hover:text-fg",
  danger: "bg-danger/15 text-danger border border-danger/50 hover:bg-danger/25",
};

const SIZE_CLASSES: Record<ButtonSize, string> = {
  sm: "px-2.5 py-1 text-xs rounded-sm",
  md: "px-3.5 py-1.5 text-xs rounded-md",
};

export function Button({
  variant = "secondary",
  size = "md",
  className,
  type = "button",
  ...rest
}: ButtonProps) {
  return (
    <button
      type={type}
      className={cn(
        "inline-flex items-center justify-center gap-1.5 font-medium transition-colors disabled:opacity-50",
        VARIANT_CLASSES[variant],
        SIZE_CLASSES[size],
        className,
      )}
      {...rest}
    />
  );
}
