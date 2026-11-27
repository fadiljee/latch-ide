import { useEffect, useState } from "react";
import { DiffViewer } from "../features/diff/DiffViewer";
import {
  connectEventsSocket,
  decideApproval,
  fetchPendingApprovals,
  fetchSessionDiff,
  fetchSessions,
} from "../lib/api/client";
import type { ApprovalRequest, Session, SessionDiff } from "../lib/api/types";
import { CATALOGS } from "../lib/i18n/catalog";
import { useWorkbenchStore } from "../lib/store/useWorkbenchStore";

export function App() {
  const locale = useWorkbenchStore((s) => s.locale);
  const sessions = useWorkbenchStore((s) => s.sessions);
  const connected = useWorkbenchStore((s) => s.connected);
  const setLocale = useWorkbenchStore((s) => s.setLocale);
  const setSessions = useWorkbenchStore((s) => s.setSessions);
  const setConnected = useWorkbenchStore((s) => s.setConnected);
  const t = CATALOGS[locale];

  const [tokenInput, setTokenInput] = useState(() => {
    try {
      return window.sessionStorage.getItem("latch_token") ?? "";
    } catch {
      return "";
    }
  });
  const [activeToken, setActiveToken] = useState(tokenInput);
  const [activeDiff, setActiveDiff] = useState<SessionDiff | null>(null);
  const [approvals, setApprovals] = useState<ApprovalRequest[]>([]);
  const [errorMsg, setErrorMsg] = useState<string | null>(null);

  useEffect(() => {
    if (!activeToken) {
      return;
    }
    let cancelled = false;
    const refresh = () => {
      void fetchSessions(activeToken)
        .then((list) => {
          if (!cancelled) {
            setSessions(list);
            setErrorMsg(null);
          }
        })
        .catch((err: unknown) => {
          if (!cancelled) {
            setErrorMsg(err instanceof Error ? err.message : "Failed to load sessions");
          }
        });
      void fetchPendingApprovals(activeToken)
        .then((list) => {
          if (!cancelled) {
            setApprovals(list);
          }
        })
        .catch(() => {
          // Abaikan bila endpoint belum tersedia
        });
    };
    refresh();
    const disconnect = connectEventsSocket(
      activeToken,
      () => refresh(),
      (isOk) => {
        if (!cancelled) {
          setConnected(isOk);
        }
      },
    );
    return () => {
      cancelled = true;
      disconnect();
    };
  }, [activeToken, setConnected, setSessions]);

  const handleConnect = (ev: React.FormEvent) => {
    ev.preventDefault();
    const trimmed = tokenInput.trim();
    try {
      window.sessionStorage.setItem("latch_token", trimmed);
    } catch {
      // Abaikan bila storage tidak tersedia
    }
    setActiveToken(trimmed);
  };

  const handleOpenDiff = (session: Session) => {
    if (!activeToken) {
      return;
    }
    void fetchSessionDiff(activeToken, session.id)
      .then((diff) => {
        setActiveDiff(diff);
        setErrorMsg(null);
      })
      .catch((err: unknown) => {
        setErrorMsg(err instanceof Error ? err.message : "Failed to load diff");
      });
  };

  const columns = [
    {
      key: "running",
      title: t.columns.running,
      items: sessions.filter((s) => s.status === "running" || s.status === "paused"),
    },
    {
      key: "waiting",
      title: t.columns.waitingApproval,
      items: sessions.filter((s) => s.status === "waiting_approval"),
    },
    {
      key: "review",
      title: t.columns.readyReview,
      items: sessions.filter((s) => s.status === "completed"),
    },
    {
      key: "attention",
      title: t.columns.attention,
      items: sessions.filter((s) => s.status === "failed" || s.status === "tripped"),
    },
  ];

  return (
    <div className="min-h-screen bg-slate-950 text-slate-100">
      <header className="flex flex-wrap items-center justify-between gap-3 border-b border-slate-800 px-4 py-4 sm:px-6">
        <div>
          <h1 className="text-lg font-semibold tracking-tight">{t.appName}</h1>
          <p className="text-sm text-slate-400">{t.subtitle}</p>
        </div>
        <div className="flex flex-wrap items-center gap-3">
          <span
            className={`text-xs ${connected ? "text-emerald-400" : "text-amber-300"}`}
            aria-label={t.connectionLabel}
          >
            {connected ? t.connected : t.disconnected}
          </span>
          <button
            type="button"
            onClick={() => setLocale(locale === "id" ? "en" : "id")}
            aria-label={t.localeToggleLabel}
            className="rounded border border-slate-700 px-3 py-1 text-xs font-medium hover:bg-slate-800 focus-visible:outline-2 focus-visible:outline-cyan-400"
          >
            {locale.toUpperCase()}
          </button>
        </div>
      </header>

      <main className="mx-auto max-w-6xl px-4 py-6 sm:px-6">
        <form onSubmit={handleConnect} className="mb-6 flex flex-wrap items-center gap-2">
          <input
            type="password"
            value={tokenInput}
            onChange={(e) => setTokenInput(e.target.value)}
            placeholder={t.tokenPlaceholder}
            aria-label={t.tokenPlaceholder}
            className="min-w-0 flex-1 rounded border border-slate-800 bg-slate-900 px-3 py-1.5 text-xs text-slate-100 placeholder:text-slate-500 focus:border-cyan-500 focus:outline-none"
          />
          <button
            type="submit"
            className="rounded bg-cyan-600 px-3 py-1.5 text-xs font-medium text-white hover:bg-cyan-500 focus-visible:outline-2 focus-visible:outline-cyan-400"
          >
            {t.connectButton}
          </button>
        </form>

        {errorMsg ? (
          <p role="alert" className="mb-4 rounded border border-rose-800 bg-rose-950/50 px-3 py-2 text-xs text-rose-200">
            {errorMsg}
          </p>
        ) : null}

        <section aria-labelledby="board-heading">
          <h2 id="board-heading" className="mb-4 text-base font-medium text-slate-200">
            {t.boardHeading}
          </h2>
          <div className="grid grid-cols-1 gap-4 md:grid-cols-4">
            {columns.map((col) => (
              <div
                key={col.key}
                className="rounded-lg border border-slate-800 bg-slate-900/60 p-4"
              >
                <div className="mb-2 flex items-center justify-between">
                  <span className="text-sm font-medium">{col.title}</span>
                  <span className="rounded bg-slate-800 px-2 py-0.5 text-xs text-slate-300">
                    {col.items.length}
                  </span>
                </div>
                {col.items.length === 0 ? (
                  <p className="text-xs text-slate-400">{t.emptySessions}</p>
                ) : (
                  <ul className="space-y-2">
                    {col.items.map((ses) => (
                      <li
                        key={ses.id}
                        className="rounded border border-slate-800 bg-slate-950/70 p-2.5 text-xs"
                      >
                        <div className="flex items-center justify-between gap-2">
                          <span className="font-mono text-cyan-300">{ses.id.slice(0, 8)}</span>
                          <span className="rounded bg-slate-800 px-1.5 py-0.5 text-[11px] text-slate-300">
                            {ses.agent_profile}
                          </span>
                        </div>
                        <p className="mt-1 truncate text-slate-300">{ses.task ?? ses.branch_name}</p>
                        <div className="mt-2 flex items-center justify-between">
                          <span className="font-mono text-[11px] text-slate-500">
                            {ses.branch_name}
                          </span>
                          <button
                            type="button"
                            onClick={() => handleOpenDiff(ses)}
                            className="rounded border border-slate-700 px-2 py-0.5 text-[11px] hover:bg-slate-800"
                          >
                            {t.viewDiff}
                          </button>
                        </div>
                      </li>
                    ))}
                  </ul>
                )}
              </div>
            ))}
          </div>
        </section>

        {approvals.length > 0 ? (
          <section
            aria-label="Pending Approvals"
            className="mt-6 rounded-lg border border-amber-800/70 bg-amber-950/30 p-4"
          >
            <h2 className="mb-3 text-sm font-semibold text-amber-200">
              {t.columns.waitingApproval} ({approvals.length})
            </h2>
            <ul className="space-y-2">
              {approvals.map((req) => (
                <li
                  key={req.id}
                  className="flex flex-wrap items-center justify-between gap-2 rounded border border-slate-800 bg-slate-950/80 p-3 text-xs"
                >
                  <div className="min-w-0 flex-1">
                    <span className="mr-2 rounded bg-slate-800 px-1.5 py-0.5 font-mono text-[11px] text-amber-300">
                      {req.action_type}
                    </span>
                    <span className="font-mono text-slate-200">{req.payload_summary}</span>
                  </div>
                  <div className="flex items-center gap-2">
                    <button
                      type="button"
                      onClick={() => {
                        void decideApproval(activeToken, req.id, "approved").then(() => {
                          setApprovals((prev) => prev.filter((item) => item.id !== req.id));
                        });
                      }}
                      className="rounded bg-emerald-600 px-2.5 py-1 text-xs font-medium text-white hover:bg-emerald-500"
                    >
                      Approve
                    </button>
                    <button
                      type="button"
                      onClick={() => {
                        void decideApproval(activeToken, req.id, "denied").then(() => {
                          setApprovals((prev) => prev.filter((item) => item.id !== req.id));
                        });
                      }}
                      className="rounded bg-rose-700 px-2.5 py-1 text-xs font-medium text-white hover:bg-rose-600"
                    >
                      Deny
                    </button>
                  </div>
                </li>
              ))}
            </ul>
          </section>
        ) : null}

        {activeDiff ? (
          <DiffViewer
            diff={activeDiff}
            onClose={() => setActiveDiff(null)}
            title={t.diffTitle}
            closeLabel={t.closeLabel}
          />
        ) : null}
      </main>
    </div>
  );
}
export default App;
