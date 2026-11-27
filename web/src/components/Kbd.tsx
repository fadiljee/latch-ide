import { cn } from "../lib/cn";

interface KbdProps {
  keys: string;
  className?: string;
}

export function Kbd({ keys, className }: KbdProps) {
  return (
    <kbd
      className={cn(
        "inline-flex items-center rounded-sm border border-line bg-app px-1.5 py-0.5 font-mono text-[10px] text-fg-3",
        className,
      )}
    >
      {keys}
    </kbd>
  );
}
