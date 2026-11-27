import { lazy, Suspense, useEffect, useMemo, useRef, useState } from "react";
import { DiffViewer } from "../features/diff/DiffViewer";
import {
  addProject,
  cancelPipeline,
  connectEventsSocket,
  createSession,
  decideApproval,
  fetchPendingApprovals,
  fetchPipelineArena,
  fetchPipelines,
  fetchProjects,
  fetchSessionDetail,
  fetchSessionDiff,
  fetchSessions,
  postSessionAction,
  selectPipelineArenaWinner,
} from "../lib/api/client";
import type {
  ApprovalRequest,
  PipelineArenaResponse,
  PipelineRun,
  Project,
  Session,
  SessionDetail,
  SessionDiff,
} from "../lib/api/types";
import { CATALOGS } from "../lib/i18n/catalog";
import { useWorkbenchStore } from "../lib/store/useWorkbenchStore";

const PipelineArenaPanel = lazy(() => import("../features/pipelines/PipelineArenaPanel"));
const TerminalPane = lazy(() => import("../features/terminal/TerminalPane"));
const NewSessionModal = lazy(() => import("../features/sessions/NewSessionModal"));

type MainView = "sessions" | "pipelines" | "approvals";
type StageTab = "terminal" | "diff";

function getInitialToken(): string {
  if (typeof window !== "undefined" && window.__LATCH_TOKEN__) {
    return window.__LATCH_TOKEN__;
  }
  try {
    return window.sessionStorage.getItem("latch_token") ?? "";
  } catch {
    return "";
  }
}

export function App() {
  const locale = useWorkbenchStore((s) => s.locale);
  const sessions = useWorkbenchStore((s) => s.sessions);
  const connected = useWorkbenchStore((s) => s.connected);
  const setLocale = useWorkbenchStore((s) => s.setLocale);
  const setSessions = useWorkbenchStore((s) => s.setSessions);
  const setConnected = useWorkbenchStore((s) => s.setConnected);
  const t = CATALOGS[locale];

  const [tokenInput, setTokenInput] = useState(getInitialToken);
  const [activeToken, setActiveToken] = useState(getInitialToken);
  const [showTokenBar, setShowTokenBar] = useState(() => getInitialToken().length === 0);

  const [projects, setProjects] = useState<Project[]>([]);
  const [selectedProjectId, setSelectedProjectId] = useState<string>("all");
  const [sidebarMode, setSidebarMode] = useState<"sessions" | "project">("sessions");
  const [mainView, setMainView] = useState<MainView>("sessions");
  const [stageTab, setStageTab] = useState<StageTab>("terminal");
  const [selectedSessionId, setSelectedSessionId] = useState<string | null>(null);
  const [sessionDetail, setSessionDetail] = useState<SessionDetail | null>(null);
  const [activeDiff, setActiveDiff] = useState<SessionDiff | null>(null);
  const [approvals, setApprovals] = useState<ApprovalRequest[]>([]);
  const [pipelines, setPipelines] = useState<PipelineRun[]>([]);
  const [activeArena, setActiveArena] = useState<PipelineArenaResponse | null>(null);
  const [searchQuery, setSearchQuery] = useState("");
  const [showNewSession, setShowNewSession] = useState(false);
  const [errorMsg, setErrorMsg] = useState<string | null>(null);
  const searchRef = useRef<HTMLInputElement | null>(null);

  useEffect(() => {
    const onKeyDown = (ev: KeyboardEvent) => {
      if ((ev.metaKey || ev.ctrlKey) && ev.key.toLowerCase() === "k") {
        ev.preventDefault();
        searchRef.current?.focus();
      }
    };
    window.addEventListener("keydown", onKeyDown);
    return () => window.removeEventListener("keydown", onKeyDown);
  }, []);

  useEffect(() => {
    if (!activeToken) {
      return;
    }
    let cancelled = false;
    const refresh = () => {
      void fetchProjects(activeToken)
        .then((list) => {
          if (!cancelled) {
            setProjects(list);
          }
        })
        .catch(() => {});
      void fetchSessions(activeToken)
        .then((list) => {
          if (!cancelled) {
            setSessions(list);
            setSelectedSessionId((prev) => prev ?? list[0]?.id ?? null);
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
        .catch(() => {});
      void fetchPipelines(activeToken)
        .then((list) => {
          if (!cancelled) {
            setPipelines(list);
          }
        })
        .catch(() => {});
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

  const selectedSession = useMemo(
    () => sessions.find((s) => s.id === selectedSessionId) ?? sessions[0] ?? null,
    [sessions, selectedSessionId],
  );

  useEffect(() => {
    if (!activeToken || !selectedSession) {
      setSessionDetail(null);
      return;
    }
    let cancelled = false;
    void fetchSessionDetail(activeToken, selectedSession.id)
      .then((detail) => {
        if (!cancelled) {
          setSessionDetail(detail);
        }
      })
      .catch(() => {});
    if (stageTab === "diff") {
      void fetchSessionDiff(activeToken, selectedSession.id)
        .then((diff) => {
          if (!cancelled) {
            setActiveDiff(diff);
          }
        })
        .catch(() => {});
    }
    return () => {
      cancelled = true;
    };
  }, [activeToken, selectedSession, stageTab]);

  const projectById = useMemo(() => {
    const map = new Map<string, Project>();
    for (const p of projects) {
      map.set(p.id, p);
    }
    return map;
  }, [projects]);

  const filteredSessions = useMemo(() => {
    const q = searchQuery.trim().toLowerCase();
    return sessions.filter((s) => {
      if (selectedProjectId !== "all" && s.project_id !== selectedProjectId) {
        return false;
      }
      if (!q) {
        return true;
      }
      return (
        (s.task ?? "").toLowerCase().includes(q) ||
        s.branch_name.toLowerCase().includes(q) ||
        s.agent_profile.toLowerCase().includes(q) ||
        s.id.toLowerCase().includes(q)
      );
    });
  }, [sessions, selectedProjectId, searchQuery]);

  const statusGroups = useMemo(
    () => [
      {
        key: "waiting",
        label: t.columns.waitingApproval,
        dot: "bg-amber-400",
        items: filteredSessions.filter((s) => s.status === "waiting_approval"),
      },
      {
        key: "working",
        label: t.columns.running,
        dot: "bg-sky-400",
        items: filteredSessions.filter((s) => s.status === "running" || s.status === "paused"),
      },
      {
        key: "failed",
        label: t.columns.attention,
        dot: "bg-rose-400",
        items: filteredSessions.filter((s) => s.status === "failed" || s.status === "tripped"),
      },
      {
        key: "done",
        label: t.columns.readyReview,
        dot: "bg-emerald-400",
        items: filteredSessions.filter((s) => s.status === "completed"),
      },
      {
        key: "idle",
        label: t.columns.idle,
        dot: "bg-slate-400",
        items: filteredSessions.filter((s) => s.status === "hibernated"),
      },
    ],
    [filteredSessions, t.columns],
  );

  const handleConnect = (ev: React.FormEvent) => {
    ev.preventDefault();
    const trimmed = tokenInput.trim();
    try {
      window.sessionStorage.setItem("latch_token", trimmed);
    } catch {
      // Abaikan bila storage tidak tersedia
    }
    setActiveToken(trimmed);
    if (trimmed) {
      setShowTokenBar(false);
    }
  };

  const runAction = (
    ses: Session,
    action: "pause" | "resume" | "hibernate" | "kill" | "discard" | "merge",
  ) => {
    if (!activeToken) {
      return;
    }
    void postSessionAction(
      activeToken,
      ses.id,
      action,
      action === "merge" ? { strategy: "squash" } : undefined,
    )
      .then(() => fetchSessions(activeToken))
      .then((list) => setSessions(list))
      .catch((err: unknown) => {
        setErrorMsg(err instanceof Error ? err.message : `Failed to ${action} session`);
      });
  };

  const totalTokens =
    (sessionDetail?.telemetry.prompt_tokens ?? 0) +
    (sessionDetail?.telemetry.completion_tokens ?? 0);

  return (
    <div className="flex h-screen flex-col overflow-hidden bg-[#0b0e17] text-slate-100">
      {/* Top Desktop Header Bar */}
      <header className="flex flex-wrap items-center justify-between gap-3 border-b border-slate-800/80 bg-[#101422]/90 px-4 py-2.5 backdrop-blur">
        <div className="flex items-center gap-3">
          <h1 className="text-sm font-semibold tracking-tight text-slate-100">{t.appName}</h1>
          <select
            aria-label={t.nav.projectLabel}
            value={selectedProjectId}
            onChange={(e) => setSelectedProjectId(e.target.value)}
            className="rounded-full border border-slate-700/80 bg-[#161b2c] px-3 py-1 text-xs text-slate-200 focus:border-cyan-500 focus:outline-none"
          >
            <option value="all">{t.nav.allProjects}</option>
            {projects.map((p) => (
              <option key={p.id} value={p.id}>
                {p.name}
              </option>
            ))}
          </select>
        </div>

        {/* Center Navigation Pills */}
        <nav aria-label="Main views" className="flex items-center gap-1 rounded-full border border-slate-800 bg-[#161b2c]/90 p-1 text-xs">
          <button
            type="button"
            onClick={() => setMainView("sessions")}
            className={`rounded-full px-3 py-1 font-medium transition ${
              mainView === "sessions"
                ? "bg-slate-700/90 text-white"
                : "text-slate-400 hover:text-slate-200"
            }`}
          >
            {t.nav.sessions}
          </button>
          <button
            type="button"
            onClick={() => setMainView("pipelines")}
            className={`rounded-full px-3 py-1 font-medium transition ${
              mainView === "pipelines"
                ? "bg-slate-700/90 text-white"
                : "text-slate-400 hover:text-slate-200"
            }`}
          >
            {t.nav.pipelines}
          </button>
          <button
            type="button"
            onClick={() => setMainView("approvals")}
            className={`flex items-center gap-1.5 rounded-full px-3 py-1 font-medium transition ${
              mainView === "approvals"
                ? "bg-slate-700/90 text-white"
                : "text-slate-400 hover:text-slate-200"
            }`}
          >
            <span>{t.nav.approvals}</span>
            {approvals.length > 0 ? (
              <span className="rounded-full bg-amber-500/20 px-1.5 py-0.2 text-[10px] text-amber-300">
                {approvals.length}
              </span>
            ) : null}
          </button>
        </nav>

        {/* Search + Actions */}
        <div className="flex items-center gap-2">
          <div className="relative hidden md:block">
            <input
              ref={searchRef}
              type="search"
              value={searchQuery}
              onChange={(e) => setSearchQuery(e.target.value)}
              placeholder={t.nav.searchPlaceholder}
              aria-label={t.nav.searchPlaceholder}
              className="w-56 rounded-full border border-slate-800 bg-[#161b2c] px-3 py-1 text-xs text-slate-200 placeholder:text-slate-500 focus:border-cyan-500 focus:outline-none"
            />
          </div>
          <button
            type="button"
            onClick={() => setShowNewSession(true)}
            className="rounded-full border border-cyan-500/40 bg-cyan-600/90 px-3 py-1 text-xs font-medium text-white hover:bg-cyan-500 focus-visible:outline-2 focus-visible:outline-cyan-400"
          >
            {t.nav.newSession}
          </button>
          <span
            title={connected ? t.connected : t.disconnected}
            aria-label={t.connectionLabel}
            className={`inline-block h-2.5 w-2.5 rounded-full ${
              connected ? "bg-emerald-400" : "bg-amber-400"
            }`}
          />
          <button
            type="button"
            onClick={() => setShowTokenBar((v) => !v)}
            className="rounded-full border border-slate-700 px-2.5 py-1 text-[11px] text-slate-300 hover:bg-slate-800"
          >
            Token
          </button>
          <button
            type="button"
            onClick={() => setLocale(locale === "id" ? "en" : "id")}
            aria-label={t.localeToggleLabel}
            className="rounded-full border border-slate-700 px-2.5 py-1 text-[11px] font-medium text-slate-300 hover:bg-slate-800 focus-visible:outline-2 focus-visible:outline-cyan-400"
          >
            {locale.toUpperCase()}
          </button>
        </div>
      </header>

      {/* Collapsible Token Connect Bar (hidden automatically when auto-connected in Desktop) */}
      {showTokenBar ? (
        <form
          onSubmit={handleConnect}
          className="flex flex-wrap items-center gap-2 border-b border-slate-800 bg-[#131828] px-4 py-2"
        >
          <input
            type="password"
            value={tokenInput}
            onChange={(e) => setTokenInput(e.target.value)}
            placeholder={t.tokenPlaceholder}
            aria-label={t.tokenPlaceholder}
            className="min-w-0 flex-1 rounded-lg border border-slate-700 bg-[#0b0e17] px-3 py-1.5 text-xs text-slate-100 placeholder:text-slate-500 focus:border-cyan-500 focus:outline-none"
          />
          <button
            type="submit"
            className="rounded-lg bg-cyan-600 px-3 py-1.5 text-xs font-medium text-white hover:bg-cyan-500"
          >
            {t.connectButton}
          </button>
        </form>
      ) : null}

      {errorMsg ? (
        <p
          role="alert"
          className="border-b border-rose-800/80 bg-rose-950/60 px-4 py-1.5 text-xs text-rose-200"
        >
          {errorMsg}
        </p>
      ) : null}

      {/* Main 3-Pane Desktop Shell */}
      <div className="flex min-h-0 flex-1 overflow-hidden">
        {/* Left Sidebar */}
        <aside
          aria-label={t.boardHeading}
          className="flex w-64 shrink-0 flex-col border-r border-slate-800/80 bg-[#0f1320] p-3"
        >
          <div className="mb-3 flex rounded-full border border-slate-800 bg-[#161b2c] p-0.5 text-[11px]">
            <button
              type="button"
              onClick={() => setSidebarMode("sessions")}
              className={`flex-1 rounded-full py-1 font-medium ${
                sidebarMode === "sessions" ? "bg-slate-700 text-white" : "text-slate-400"
              }`}
            >
              {t.nav.sessionsOnly}
            </button>
            <button
              type="button"
              onClick={() => setSidebarMode("project")}
              className={`flex-1 rounded-full py-1 font-medium ${
                sidebarMode === "project" ? "bg-slate-700 text-white" : "text-slate-400"
              }`}
            >
              {t.nav.byProject}
            </button>
          </div>

          <h2 className="sr-only">{t.boardHeading}</h2>

          <div className="flex-1 space-y-4 overflow-y-auto pr-1">
            {filteredSessions.length === 0 ? (
              <p className="px-2 py-4 text-xs text-slate-500">{t.emptySessions}</p>
            ) : (
              statusGroups
                .filter((g) => g.items.length > 0)
                .map((group) => (
                  <div key={group.key}>
                    <div className="mb-1.5 px-2 text-[11px] font-medium text-slate-500">
                      {group.label}
                    </div>
                    <ul className="space-y-1">
                      {group.items.map((ses) => {
                        const isSelected = selectedSession?.id === ses.id && mainView === "sessions";
                        const projName =
                          projectById.get(ses.project_id)?.name ?? ses.agent_profile;
                        return (
                          <li key={ses.id}>
                            <button
                              type="button"
                              onClick={() => {
                                setSelectedSessionId(ses.id);
                                setMainView("sessions");
                              }}
                              className={`flex w-full items-center justify-between gap-2 rounded-xl px-2.5 py-2 text-left transition ${
                                isSelected
                                  ? "bg-slate-700/70 text-white"
                                  : "text-slate-300 hover:bg-slate-800/60"
                              }`}
                            >
                              <div className="min-w-0 flex-1">
                                <div className="truncate text-xs font-medium">
                                  {ses.task ?? ses.branch_name}
                                </div>
                                <div className="truncate text-[11px] text-slate-400">
                                  {sidebarMode === "project" ? projName : ses.agent_profile}
                                </div>
                              </div>
                              <span
                                className={`h-2 w-2 shrink-0 rounded-full ${group.dot}`}
                                aria-hidden="true"
                              />
                            </button>
                          </li>
                        );
                      })}
                    </ul>
                  </div>
                ))
            )}
          </div>
        </aside>

        {/* Center Main Stage */}
        <main className="flex min-w-0 flex-1 flex-col overflow-hidden p-3">
          {mainView === "sessions" ? (
            selectedSession ? (
              <div className="flex min-h-0 flex-1 flex-col overflow-hidden rounded-2xl border border-slate-800/90 bg-[#121624] shadow-xl">
                {/* Session Stage Header */}
                <div className="flex flex-wrap items-center justify-between gap-2 border-b border-slate-800/80 px-4 py-3">
                  <div className="min-w-0">
                    <div className="flex flex-wrap items-center gap-2">
                      <h2 className="truncate text-sm font-semibold text-white">
                        {selectedSession.task ?? selectedSession.branch_name}
                      </h2>
                      <span className="text-xs text-slate-400">
                        {selectedSession.agent_profile} · {selectedSession.branch_name}
                        {totalTokens > 0
                          ? ` · ${(totalTokens / 1000).toFixed(1)}k tokens · $${(sessionDetail?.telemetry.estimated_cost_usd ?? 0).toFixed(4)}`
                          : ""}
                      </span>
                    </div>
                  </div>

                  <div className="flex flex-wrap items-center gap-2">
                    <div className="flex rounded-full border border-slate-700 bg-[#0b0e17] p-0.5 text-xs">
                      <button
                        type="button"
                        onClick={() => setStageTab("terminal")}
                        className={`rounded-full px-3 py-1 font-medium ${
                          stageTab === "terminal"
                            ? "bg-slate-700 text-white"
                            : "text-slate-400 hover:text-slate-200"
                        }`}
                      >
                        {t.nav.terminalTab}
                      </button>
                      <button
                        type="button"
                        onClick={() => setStageTab("diff")}
                        className={`rounded-full px-3 py-1 font-medium ${
                          stageTab === "diff"
                            ? "bg-slate-700 text-white"
                            : "text-slate-400 hover:text-slate-200"
                        }`}
                      >
                        {t.nav.diffTab}
                      </button>
                    </div>

                    {selectedSession.status === "running" ? (
                      <>
                        <button
                          type="button"
                          onClick={() => runAction(selectedSession, "pause")}
                          className="rounded-lg border border-slate-700 px-2.5 py-1 text-[11px] text-slate-200 hover:bg-slate-800"
                        >
                          {t.nav.pause}
                        </button>
                        <button
                          type="button"
                          onClick={() => runAction(selectedSession, "hibernate")}
                          className="rounded-lg border border-slate-700 px-2.5 py-1 text-[11px] text-slate-200 hover:bg-slate-800"
                        >
                          {t.nav.hibernate}
                        </button>
                        <button
                          type="button"
                          onClick={() => runAction(selectedSession, "kill")}
                          className="rounded-lg border border-rose-800/80 px-2.5 py-1 text-[11px] text-rose-300 hover:bg-rose-950"
                        >
                          {t.nav.kill}
                        </button>
                      </>
                    ) : selectedSession.status === "paused" ||
                      selectedSession.status === "hibernated" ? (
                      <button
                        type="button"
                        onClick={() => runAction(selectedSession, "resume")}
                        className="rounded-lg bg-emerald-700 px-2.5 py-1 text-[11px] font-medium text-white hover:bg-emerald-600"
                      >
                        {t.nav.resume}
                      </button>
                    ) : (
                      <>
                        <button
                          type="button"
                          onClick={() => runAction(selectedSession, "merge")}
                          className="rounded-lg bg-emerald-700 px-2.5 py-1 text-[11px] font-medium text-white hover:bg-emerald-600"
                        >
                          {t.nav.merge}
                        </button>
                        <button
                          type="button"
                          onClick={() => runAction(selectedSession, "discard")}
                          className="rounded-lg border border-rose-800/80 px-2.5 py-1 text-[11px] text-rose-300 hover:bg-rose-950"
                        >
                          {t.nav.discard}
                        </button>
                      </>
                    )}
                  </div>
                </div>

                {/* Stage Content: Live Terminal or Diff */}
                {stageTab === "terminal" ? (
                  <Suspense fallback={null}>
                    <TerminalPane
                      token={activeToken}
                      sessionId={selectedSession.id}
                      workdir={selectedSession.worktree_path}
                      agentProfile={selectedSession.agent_profile}
                      sendPlaceholder={t.nav.sendPlaceholder}
                    />
                  </Suspense>
                ) : (
                  <div className="flex-1 overflow-auto p-4">
                    {activeDiff ? (
                      <DiffViewer
                        diff={activeDiff}
                        onClose={() => setStageTab("terminal")}
                        title={t.diffTitle}
                        closeLabel={t.closeLabel}
                      />
                    ) : (
                      <p className="text-xs text-slate-400">Loading diff…</p>
                    )}
                  </div>
                )}
              </div>
            ) : (
              <div className="flex flex-1 flex-col items-center justify-center rounded-2xl border border-slate-800/80 bg-[#121624]/60 p-8 text-center">
                <p className="text-sm font-medium text-slate-200">{t.nav.noSessionSelected}</p>
                <p className="mt-1 text-xs text-slate-400">{t.nav.selectOrCreate}</p>
                <button
                  type="button"
                  onClick={() => setShowNewSession(true)}
                  className="mt-4 rounded-full bg-cyan-600 px-4 py-1.5 text-xs font-medium text-white hover:bg-cyan-500"
                >
                  {t.nav.newSession}
                </button>
              </div>
            )
          ) : mainView === "pipelines" ? (
            <div className="flex-1 overflow-auto rounded-2xl border border-slate-800/90 bg-[#121624] p-4">
              <Suspense fallback={null}>
                <PipelineArenaPanel
                  pipelines={pipelines}
                  activeArena={activeArena}
                  labels={t.pipelines}
                  closeLabel={t.closeLabel}
                  onInspectArena={(run) => {
                    if (!activeToken) {
                      return;
                    }
                    void fetchPipelineArena(activeToken, run.id)
                      .then((arena) => {
                        setActiveArena(arena);
                        setErrorMsg(null);
                      })
                      .catch((err: unknown) => {
                        setErrorMsg(err instanceof Error ? err.message : "Failed to load arena");
                      });
                  }}
                  onCancelPipeline={(id) => {
                    if (!activeToken) {
                      return;
                    }
                    void cancelPipeline(activeToken, id)
                      .then((updated) => {
                        setPipelines((prev) =>
                          prev.map((p) => (p.id === updated.id ? updated : p)),
                        );
                      })
                      .catch((err: unknown) => {
                        setErrorMsg(
                          err instanceof Error ? err.message : "Failed to cancel pipeline",
                        );
                      });
                  }}
                  onSelectWinner={(pipelineId, stepId, candidateId) => {
                    if (!activeToken) {
                      return;
                    }
                    void selectPipelineArenaWinner(activeToken, pipelineId, stepId, candidateId)
                      .then(() => fetchPipelineArena(activeToken, pipelineId))
                      .then((arena) => {
                        setActiveArena(arena);
                        setErrorMsg(null);
                      })
                      .catch((err: unknown) => {
                        setErrorMsg(
                          err instanceof Error ? err.message : "Failed to select winner",
                        );
                      });
                  }}
                  onCloseArena={() => setActiveArena(null)}
                />
              </Suspense>
            </div>
          ) : (
            <div className="flex-1 overflow-auto rounded-2xl border border-slate-800/90 bg-[#121624] p-5">
              <h2 className="mb-4 text-sm font-semibold text-amber-200">
                {t.nav.approvals} ({approvals.length})
              </h2>
              {approvals.length === 0 ? (
                <p className="text-xs text-slate-400">{t.nav.noApprovals}</p>
              ) : (
                <ul className="space-y-2">
                  {approvals.map((req) => (
                    <li
                      key={req.id}
                      className="flex flex-wrap items-center justify-between gap-2 rounded-xl border border-slate-800 bg-[#0b0e17] p-3 text-xs"
                    >
                      <div className="min-w-0 flex-1">
                        <span className="mr-2 rounded bg-amber-950/80 px-2 py-0.5 font-mono text-[11px] text-amber-300">
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
                          className="rounded-lg bg-emerald-600 px-3 py-1 text-xs font-medium text-white hover:bg-emerald-500"
                        >
                          {t.nav.approve}
                        </button>
                        <button
                          type="button"
                          onClick={() => {
                            void decideApproval(activeToken, req.id, "denied").then(() => {
                              setApprovals((prev) => prev.filter((item) => item.id !== req.id));
                            });
                          }}
                          className="rounded-lg bg-rose-700 px-3 py-1 text-xs font-medium text-white hover:bg-rose-600"
                        >
                          {t.nav.deny}
                        </button>
                      </div>
                    </li>
                  ))}
                </ul>
              )}
            </div>
          )}
        </main>

        {/* Right Floating Action Rail */}
        <aside
          aria-label="Quick actions"
          className="hidden w-12 shrink-0 flex-col items-center justify-center py-4 lg:flex"
        >
          <div className="flex flex-col items-center gap-2 rounded-full border border-slate-800 bg-[#141929]/90 p-1.5 shadow-lg">
            <button
              type="button"
              title={t.nav.terminalTab}
              aria-label={t.nav.terminalTab}
              onClick={() => {
                setMainView("sessions");
                setStageTab("terminal");
              }}
              className="rounded-full p-2 text-slate-400 hover:bg-slate-800 hover:text-white"
            >
              <span className="font-mono text-xs">&gt;_</span>
            </button>
            <button
              type="button"
              title={t.nav.diffTab}
              aria-label={t.nav.diffTab}
              onClick={() => {
                setMainView("sessions");
                setStageTab("diff");
              }}
              className="rounded-full p-2 text-slate-400 hover:bg-slate-800 hover:text-white"
            >
              <span className="font-mono text-xs">+/-</span>
            </button>
            <button
              type="button"
              title={t.nav.pipelines}
              aria-label={t.nav.pipelines}
              onClick={() => setMainView("pipelines")}
              className="rounded-full p-2 text-slate-400 hover:bg-slate-800 hover:text-white"
            >
              <span className="font-mono text-xs">DAG</span>
            </button>
            <button
              type="button"
              title={t.nav.approvals}
              aria-label={t.nav.approvals}
              onClick={() => setMainView("approvals")}
              className="rounded-full p-2 text-slate-400 hover:bg-slate-800 hover:text-white"
            >
              <span className="font-mono text-xs">OK</span>
            </button>
          </div>
        </aside>
      </div>

      {showNewSession ? (
        <Suspense fallback={null}>
          <NewSessionModal
            projects={projects}
            labels={t.nav}
            onAddRepo={async (repoPath) => {
              if (!activeToken) {
                return;
              }
              const added = await addProject(activeToken, repoPath);
              setProjects((prev) => [...prev, added]);
            }}
            onCreate={async (params) => {
              if (!activeToken) {
                return;
              }
              const created = await createSession(activeToken, params);
              setSessions([created, ...sessions]);
              setSelectedSessionId(created.id);
              setMainView("sessions");
              setStageTab("terminal");
            }}
            onClose={() => setShowNewSession(false)}
          />
        </Suspense>
      ) : null}
    </div>
  );
}

export default App;
