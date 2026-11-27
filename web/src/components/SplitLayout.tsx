import type { ReactNode } from "react";
import type { SplitMode } from "../state/layout";

interface SplitLayoutProps {
  mode: SplitMode;
  primary: ReactNode;
  secondary?: ReactNode;
}

export function SplitLayout({ mode, primary, secondary }: SplitLayoutProps) {
  if (mode === "single" || !secondary) {
    return <div className="flex min-h-0 flex-1 flex-col overflow-hidden">{primary}</div>;
  }

  return (
    <div className="grid min-h-0 flex-1 grid-cols-1 gap-[var(--gap-panel)] overflow-hidden lg:grid-cols-2">
      <div className="flex min-h-0 flex-col overflow-hidden border-r border-line">{primary}</div>
      <div className="flex min-h-0 flex-col overflow-hidden">{secondary}</div>
    </div>
  );
}
