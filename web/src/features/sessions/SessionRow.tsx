import { ModelBadge } from "../../components/ModelBadge";
import { StatusDot } from "../../components/StatusDot";
import { formatRelativeShort } from "../../lib/time";
import { cn } from "../../lib/cn";
import { mapSessionUiStatus, useSessionsStore } from "../../state/sessions";
import type { Session } from "../../transport/protocol";

interface SessionRowProps {
  session: Session;
  projectName?: string;
  selected: boolean;
  onSelect: (id: string) => void;
}

export function SessionRow({ session, projectName, selected, onSelect }: SessionRowProps) {
  const unread = useSessionsStore((s) => s.unreadById[session.id] ?? 0);
  const uiStatus = mapSessionUiStatus(session.status);

  return (
    <button
      type="button"
      onClick={() => onSelect(session.id)}
      className={cn(
        "flex w-full flex-col gap-1 rounded-md px-2.5 py-2 text-left transition-colors",
        selected ? "bg-selected text-fg" : "text-fg-2 hover:bg-hover hover:text-fg",
      )}
    >
      <div className="flex items-center justify-between gap-2">
        <span className="flex min-w-0 flex-1 items-center gap-2">
          <StatusDot status={uiStatus} />
          <span className="truncate text-xs font-medium">
            {session.task ?? session.branch_name}
          </span>
        </span>
        {unread > 0 ? (
          <span className="rounded-sm bg-needs/20 px-1 font-mono text-[10px] text-needs">
            {unread}
          </span>
        ) : (
          <span className="shrink-0 font-mono text-[10px] text-fg-3">
            {formatRelativeShort(session.started_at)}
          </span>
        )}
      </div>

      <div className="flex items-center justify-between gap-2 pl-4">
        <span className="truncate font-mono text-[11px] text-fg-3">
          {projectName ?? session.branch_name}
        </span>
        <ModelBadge profile={session.agent_profile} />
      </div>
    </button>
  );
}
