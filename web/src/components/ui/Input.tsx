import { forwardRef, type InputHTMLAttributes } from "react";
import { cn } from "../../lib/cn";

export const Input = forwardRef<HTMLInputElement, InputHTMLAttributes<HTMLInputElement>>(
  function Input({ className, ...rest }, ref) {
    return (
      <input
        ref={ref}
        className={cn(
          "w-full rounded-md border border-line bg-app px-2.5 py-1.5 text-xs text-fg placeholder:text-fg-3 focus:border-accent focus:outline-none",
          className,
        )}
        {...rest}
      />
    );
  },
);
