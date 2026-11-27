import { cn } from "../lib/cn";
import type { UiStatus } from "../transport/protocol";

const DOT_CLASS: Record<UiStatus, string> = {
  needs_you: "bg-needs",
  working: "bg-working",
  error: "bg-danger",
  done: "bg-done",
  idle: "bg-idle",
};

interface StatusDotProps {
  status: UiStatus;
  label?: string;
  className?: string;
}

export function StatusDot({ status, label, className }: StatusDotProps) {
  return (
    <span
      className={cn("inline-block h-2 w-2 shrink-0 rounded-full", DOT_CLASS[status], className)}
      title={label}
      aria-hidden={label ? undefined : true}
      aria-label={label}
    />
  );
}
