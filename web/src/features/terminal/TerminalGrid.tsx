import { lazy, Suspense } from "react";
import { Button } from "../../components/ui/Button";
import { cn } from "../../lib/cn";
import type { Session } from "../../transport/protocol";

const TerminalView = lazy(() => import("./TerminalView"));

interface TerminalGridProps {
  token: string;
  sessions: readonly Session[];
  focusedSessionId: string;
  resyncLabel: string;
  inputPlaceholder: string;
  focusedLabel: string;
  clickToFocusLabel: string;
  expandLabel: string;
  loadingLabel: string;
  onFocusSession: (sessionId: string) => void;
  onExpandSession: (sessionId: string) => void;
}

export function TerminalGrid({
  token,
  sessions,
  focusedSessionId,
  resyncLabel,
  inputPlaceholder,
  focusedLabel,
  clickToFocusLabel,
  expandLabel,
  loadingLabel,
  onFocusSession,
  onExpandSession,
}: TerminalGridProps) {
  const cells = sessions.slice(0, 4);

  return (
    <div
      role="region"
      aria-label="2x2 Terminal Grid"
      className="grid min-h-0 flex-1 grid-cols-1 gap-[var(--gap-panel)] overflow-hidden md:grid-cols-2"
    >
      {cells.map((ses) => {
        const isFocused = ses.id === focusedSessionId;
        return (
          <div
            key={ses.id}
            data-focused={isFocused ? "true" : "false"}
            className={cn(
              "flex min-h-0 flex-col overflow-hidden rounded-lg border bg-surface shadow-[var(--panel-shadow)]",
              isFocused ? "border-accent ring-1 ring-accent" : "border-line",
            )}
          >
            <div className="flex items-center justify-between gap-2 border-b border-line px-3 py-1.5 text-xs">
              <button
                type="button"
                aria-pressed={isFocused}
                onClick={() => onFocusSession(ses.id)}
                className="flex min-w-0 items-center gap-2 text-left"
              >
                <span className="truncate font-semibold text-fg">
                  {ses.task ?? ses.branch_name}
                </span>
                <span
                  className={cn(
                    "shrink-0 rounded-sm px-1.5 py-0.5 font-mono text-[10px]",
                    isFocused
                      ? "border border-accent/50 bg-accent-soft text-accent"
                      : "border border-line bg-app text-fg-3",
                  )}
                >
                  {isFocused ? focusedLabel : clickToFocusLabel}
                </span>
              </button>
              <Button size="sm" variant="ghost" onClick={() => onExpandSession(ses.id)}>
                {expandLabel}
              </Button>
            </div>

            <Suspense fallback={<p className="p-3 text-xs text-fg-3">{loadingLabel}</p>}>
              <TerminalView
                token={token}
                sessionId={ses.id}
                workdir={ses.worktree_path}
                agentProfile={ses.agent_profile}
                resyncLabel={resyncLabel}
                inputPlaceholder={inputPlaceholder}
                focused={isFocused}
                onFocus={() => onFocusSession(ses.id)}
              />
            </Suspense>
          </div>
        );
      })}
    </div>
  );
}
