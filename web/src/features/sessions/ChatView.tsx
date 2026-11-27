import { useEffect, useState } from "react";
import { Button } from "../../components/ui/Button";
import { ModelBadge } from "../../components/ModelBadge";
import { StatusDot } from "../../components/StatusDot";
import type { Catalog } from "../../lib/i18n/catalog";
import { useMessagesStore } from "../../state/messages";
import { mapSessionUiStatus } from "../../state/sessions";
import { fetchSessionScreen } from "../../transport/daemon-client";
import type { Session } from "../../transport/protocol";
import { Composer } from "./Composer";

const EMPTY_MESSAGES: readonly { id: string; role: "user" | "system"; text: string }[] = [];

interface ChatViewProps {
  token: string;
  session: Session;
  labels: Catalog["sessionActions"];
  onAction: (action: "pause" | "resume" | "hibernate" | "kill" | "merge" | "discard") => void;
}

export function ChatView({ token, session, labels, onAction }: ChatViewProps) {
  const messages = useMessagesStore((s) => s.bySessionId[session.id] ?? EMPTY_MESSAGES);
  const [screenText, setScreenText] = useState("");
  const uiStatus = mapSessionUiStatus(session.status);

  useEffect(() => {
    if (!token) {
      return;
    }
    let cancelled = false;
    void fetchSessionScreen(token, session.id)
      .then((snap) => {
        if (!cancelled) {
          setScreenText(snap.text);
        }
      })
      .catch(() => {
        if (!cancelled) {
          setScreenText("");
        }
      });
    return () => {
      cancelled = true;
    };
  }, [token, session.id, session.status]);

  return (
    <div className="flex min-h-0 flex-1 flex-col bg-surface">
      <div className="flex flex-wrap items-center justify-between gap-2 border-b border-line px-4 py-2.5">
        <div className="flex min-w-0 items-center gap-2 text-xs">
          <StatusDot status={uiStatus} />
          <span className="font-mono text-fg-2">{session.branch_name}</span>
          <ModelBadge profile={session.agent_profile} />
        </div>
        <div className="flex flex-wrap items-center gap-1.5">
          {session.status === "running" ? (
            <>
              <Button size="sm" onClick={() => onAction("pause")}>
                {labels.pause}
              </Button>
              <Button size="sm" onClick={() => onAction("hibernate")}>
                {labels.hibernate}
              </Button>
              <Button size="sm" variant="danger" onClick={() => onAction("kill")}>
                {labels.kill}
              </Button>
            </>
          ) : session.status === "paused" || session.status === "hibernated" ? (
            <Button size="sm" variant="primary" onClick={() => onAction("resume")}>
              {labels.resume}
            </Button>
          ) : (
            <>
              <Button size="sm" variant="primary" onClick={() => onAction("merge")}>
                {labels.merge}
              </Button>
              <Button size="sm" variant="danger" onClick={() => onAction("discard")}>
                {labels.discard}
              </Button>
            </>
          )}
        </div>
      </div>

      <div className="flex-1 space-y-4 overflow-y-auto p-4 text-xs">
        <section>
          <h3 className="mb-1.5 font-medium text-fg-2">{labels.screenPreview}</h3>
          <pre className="max-h-56 overflow-auto rounded-md border border-line bg-terminal p-3 font-mono text-[11px] whitespace-pre-wrap text-terminal-fg">
            {screenText || `worktree: ${session.worktree_path}`}
          </pre>
        </section>

        <section>
          <h3 className="mb-2 font-medium text-fg-2">{labels.historyTitle}</h3>
          {messages.length === 0 ? (
            <p className="rounded-md border border-line bg-app px-3 py-3 text-fg-3">
              {labels.noMessages}
            </p>
          ) : (
            <ul className="space-y-2">
              {messages.map((m) => (
                <li
                  key={m.id}
                  className="rounded-md border border-line bg-raised px-3 py-2 font-mono text-fg"
                >
                  <span className="mr-2 text-fg-3">{m.role === "user" ? ">" : "!"}</span>
                  <span>{m.text}</span>
                </li>
              ))}
            </ul>
          )}
        </section>
      </div>

      <Composer
        token={token}
        sessionId={session.id}
        labels={labels}
        disabled={session.status !== "running" && session.status !== "waiting_approval"}
      />
    </div>
  );
}
