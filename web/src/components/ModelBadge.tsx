import { cn } from "../lib/cn";

interface ModelBadgeProps {
  profile: string;
  className?: string;
}

export function ModelBadge({ profile, className }: ModelBadgeProps) {
  return (
    <span
      className={cn(
        "inline-flex items-center rounded-sm border border-line bg-raised px-1.5 py-0.5 font-mono text-[11px] text-fg-2",
        className,
      )}
    >
      {profile}
    </span>
  );
}
