import { useEffect, useState } from "react";
import { Button } from "../../components/ui/Button";
import type { Catalog } from "../../lib/i18n/catalog";
import { checkSessionMerge, fetchSessionDiff } from "../../transport/daemon-client";
import type { MergeCheckResult, Session, SessionDiff } from "../../transport/protocol";

interface GitPanelProps {
  token: string;
  session: Session;
  labels: Catalog["sidePanel"];
  actions: Catalog["sessionActions"];
  onAction: (action: "merge" | "discard") => void;
}

export function GitPanel({ token, session, labels, actions, onAction }: GitPanelProps) {
  const [diff, setDiff] = useState<SessionDiff | null>(null);
  const [mergeCheck, setMergeCheck] = useState<MergeCheckResult | null>(null);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    if (!token) {
      return;
    }
    let cancelled = false;
    void fetchSessionDiff(token, session.id)
      .then((d) => {
        if (!cancelled) {
          setDiff(d);
          setError(null);
        }
      })
      .catch((err) => {
        if (!cancelled) {
          setError(err instanceof Error ? err.message : "Failed loading diff");
        }
      });
    return () => {
      cancelled = true;
    };
  }, [token, session.id]);

  const handleCheck = () => {
    if (!token) {
      return;
    }
    void checkSessionMerge(token, session.id)
      .then((res) => setMergeCheck(res))
      .catch((err) => setError(err instanceof Error ? err.message : "Merge check failed"));
  };

  return (
    <div className="space-y-4 text-xs">
      <div className="flex flex-wrap items-center gap-2">
        <Button size="sm" onClick={handleCheck}>
          {labels.conflictCheck}
        </Button>
        {!session.status || session.status === "completed" || session.status === "failed" ? (
          <>
            <Button size="sm" variant="primary" onClick={() => onAction("merge")}>
              {actions.merge}
            </Button>
            <Button size="sm" variant="danger" onClick={() => onAction("discard")}>
              {actions.discard}
            </Button>
          </>
        ) : null}
      </div>

      {error ? (
        <p role="alert" className="rounded-md border border-danger/50 bg-danger/10 p-2 text-danger">
          {error}
        </p>
      ) : null}

      {mergeCheck ? (
        <div
          className={
            mergeCheck.clean
              ? "rounded-md border border-done/50 bg-done/10 p-2.5 text-done"
              : "rounded-md border border-danger/50 bg-danger/10 p-2.5 text-danger"
          }
        >
          {mergeCheck.clean ? (
            <p>{labels.cleanMerge}</p>
          ) : (
            <div>
              <p className="font-medium">{labels.conflictFound}</p>
              <ul className="mt-1 list-disc pl-4 font-mono">
                {mergeCheck.conflicts.map((c) => (
                  <li key={c}>{c}</li>
                ))}
              </ul>
            </div>
          )}
        </div>
      ) : null}

      {!diff || diff.files.length === 0 ? (
        <p className="text-fg-3">{labels.noChanges}</p>
      ) : (
        <ul className="space-y-1.5">
          {diff.files.map((f) => (
            <li
              key={f.path}
              className="flex items-center justify-between gap-2 rounded-md border border-line bg-app px-2.5 py-1.5 font-mono"
            >
              <span className="truncate text-fg">{f.path}</span>
              <span className="shrink-0">
                <span className="text-done">+{f.additions}</span>{" "}
                <span className="text-danger">-{f.deletions}</span>
              </span>
            </li>
          ))}
        </ul>
      )}
    </div>
  );
}
