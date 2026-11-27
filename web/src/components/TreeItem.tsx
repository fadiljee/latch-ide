import type { ReactNode } from "react";
import { cn } from "../lib/cn";

interface TreeItemProps {
  label: string;
  meta?: string;
  depth?: number;
  selected?: boolean;
  leading?: ReactNode;
  trailing?: ReactNode;
  onClick?: () => void;
}

export function TreeItem({
  label,
  meta,
  depth = 0,
  selected = false,
  leading,
  trailing,
  onClick,
}: TreeItemProps) {
  return (
    <button
      type="button"
      onClick={onClick}
      style={depth > 0 ? { paddingLeft: `${8 + depth * 12}px` } : undefined}
      className={cn(
        "flex w-full items-center justify-between gap-2 rounded-md px-2 py-1.5 text-left text-xs transition-colors",
        selected ? "bg-selected text-fg" : "text-fg-2 hover:bg-hover hover:text-fg",
      )}
    >
      <span className="flex min-w-0 flex-1 items-center gap-2">
        {leading}
        <span className="min-w-0 flex-1 truncate">{label}</span>
      </span>
      {meta ? <span className="shrink-0 font-mono text-[11px] text-fg-3">{meta}</span> : null}
      {trailing}
    </button>
  );
}
