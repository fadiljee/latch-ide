import { lazy, Suspense, useEffect, useState } from "react";
import { Button } from "../components/ui/Button";
import { Input } from "../components/ui/Input";
import { SplitLayout } from "../components/SplitLayout";
import { TabBar, type TabOption } from "../components/TabBar";
import { AutomationsPage } from "../features/automations/AutomationsPage";
import { BotsPage } from "../features/bots/BotsPage";
import { HostsPage } from "../features/hosts/HostsPage";
import { PluginsPage } from "../features/plugins/PluginsPage";
import { ChatView } from "../features/sessions/ChatView";
import { SettingsPage } from "../features/settings/SettingsPage";
import { UsagePage } from "../features/usage/UsagePage";
import { CATALOGS } from "../lib/i18n/catalog";
import { useHostsStore } from "../state/hosts";
import { useLayoutStore, type SessionTab } from "../state/layout";
import { useNotificationsStore } from "../state/notifications";
import { useSessionsStore } from "../state/sessions";
import { useUiStore } from "../state/ui";
import { useUsageStore } from "../state/usage";
import {
  fetchSessionDetail,
  fetchSessionDiff,
  fetchSessions,
  postSessionAction,
} from "../transport/daemon-client";
import type { Session, SessionDiff } from "../transport/protocol";

const TerminalView = lazy(() => import("../features/terminal/TerminalView"));
const TerminalGrid = lazy(() =>
  import("../features/terminal/TerminalGrid").then((m) => ({ default: m.TerminalGrid })),
);
const DiffView = lazy(() => import("../features/review/DiffView"));

interface SessionPaneProps {
  token: string;
  session: Session;
  tab: SessionTab;
  onTabChange: (tab: SessionTab) => void;
}

function SessionPane({ token, session, tab, onTabChange }: SessionPaneProps) {
  const locale = useUiStore((s) => s.locale);
  const upsertMany = useSessionsStore((s) => s.upsertMany);
  const setSessionSummary = useUsageStore((s) => s.setSessionSummary);
  const summary = useUsageStore((s) => s.bySessionId[session.id]);
  const pushToast = useNotificationsStore((s) => s.pushToast);
  const [diff, setDiff] = useState<SessionDiff | null>(null);
  const t = CATALOGS[locale];

  useEffect(() => {
    if (!token) {
      return;
    }
    let cancelled = false;
    void fetchSessionDetail(token, session.id)
      .then((detail) => {
        if (!cancelled) {
          setSessionSummary(session.id, detail.telemetry);
        }
      })
      .catch(() => {});

    if (tab === "diff") {
      void fetchSessionDiff(token, session.id)
        .then((d) => {
          if (!cancelled) {
            setDiff(d);
          }
        })
        .catch(() => {});
    }
    return () => {
      cancelled = true;
    };
  }, [token, session.id, tab, setSessionSummary]);

  const handleAction = (
    action: "pause" | "resume" | "hibernate" | "kill" | "merge" | "discard",
  ) => {
    if (!token) {
      return;
    }
    void postSessionAction(
      token,
      session.id,
      action,
      action === "merge" ? { strategy: "squash" } : undefined,
    )
      .then(() => fetchSessions(token))
      .then((list) => {
        upsertMany(list);
        pushToast("success", `Session ${action} complete`);
      })
      .catch((err) => pushToast("danger", err instanceof Error ? err.message : "Action failed"));
  };

  const tabs: readonly TabOption<SessionTab>[] = [
    { id: "terminal", label: t.workspace.terminalTab },
    { id: "chat", label: t.workspace.chatTab },
    { id: "diff", label: t.workspace.diffTab },
  ];

  const totalTokens = (summary?.prompt_tokens ?? 0) + (summary?.completion_tokens ?? 0);

  return (
    <div className="flex min-h-0 flex-1 flex-col overflow-hidden rounded-lg border border-line bg-surface shadow-[var(--panel-shadow)]">
      <div className="flex flex-wrap items-center justify-between gap-2 border-b border-line px-3.5 py-2">
        <div className="min-w-0">
          <p className="truncate text-xs font-semibold text-fg">
            {session.task ?? session.branch_name}
          </p>
          <p className="truncate font-mono text-[11px] text-fg-3">
            {session.agent_profile} · {session.branch_name}
            {totalTokens > 0
              ? ` · ${(totalTokens / 1000).toFixed(1)}k tok · $${(summary?.estimated_cost_usd ?? 0).toFixed(4)}`
              : ""}
          </p>
        </div>

        <TabBar label="Session view mode" tabs={tabs} active={tab} onSelect={onTabChange} />
      </div>

      {tab === "terminal" ? (
        <Suspense
          fallback={<p className="p-4 text-xs text-fg-3">{t.workspace.loadingDiff}</p>}
        >
          <TerminalView
            token={token}
            sessionId={session.id}
            workdir={session.worktree_path}
            agentProfile={session.agent_profile}
            resyncLabel={t.workspace.resync}
            inputPlaceholder={t.sessionActions.composerPlaceholder}
          />
        </Suspense>
      ) : tab === "chat" ? (
        <ChatView
          token={token}
          session={session}
          labels={t.sessionActions}
          onAction={handleAction}
        />
      ) : (
        <Suspense
          fallback={<p className="p-4 text-xs text-fg-3">{t.workspace.loadingDiff}</p>}
        >
          {diff ? (
            <DiffView
              diff={diff}
              title={t.sidePanel.diffTitle}
              closeLabel={t.sidePanel.close}
            />
          ) : (
            <p className="p-4 text-xs text-fg-3">{t.workspace.loadingDiff}</p>
          )}
        </Suspense>
      )}
    </div>
  );
}

export function Workspace() {
  const locale = useUiStore((s) => s.locale);
  const activeRoute = useUiStore((s) => s.activeRoute);
  const setNewSessionOpen = useUiStore((s) => s.setNewSessionOpen);

  const token = useHostsStore((s) => s.token);
  const setToken = useHostsStore((s) => s.setToken);
  const error = useHostsStore((s) => s.error);

  const byId = useSessionsStore((s) => s.byId);
  const order = useSessionsStore((s) => s.order);

  const activeSessionId = useLayoutStore((s) => s.activeSessionId);
  const secondarySessionId = useLayoutStore((s) => s.secondarySessionId);
  const focusedGridSessionId = useLayoutStore((s) => s.focusedGridSessionId);
  const splitMode = useLayoutStore((s) => s.splitMode);
  const primaryTab = useLayoutStore((s) => s.primaryTab);
  const secondaryTab = useLayoutStore((s) => s.secondaryTab);
  const setActiveSession = useLayoutStore((s) => s.setActiveSession);
  const setFocusedGridSession = useLayoutStore((s) => s.setFocusedGridSession);
  const setSplitMode = useLayoutStore((s) => s.setSplitMode);
  const setPrimaryTab = useLayoutStore((s) => s.setPrimaryTab);
  const setSecondaryTab = useLayoutStore((s) => s.setSecondaryTab);

  const [tokenDraft, setTokenDraft] = useState("");
  const t = CATALOGS[locale];

  const primarySession = activeSessionId ? byId[activeSessionId] : undefined;
  const fallbackSession = order[0] ? byId[order[0]] : undefined;
  const effectivePrimary = primarySession ?? fallbackSession;
  const effectiveSecondary =
    (secondarySessionId ? byId[secondarySessionId] : undefined) ?? effectivePrimary;

  const gridSessions: Session[] = [];
  if (effectivePrimary) {
    gridSessions.push(effectivePrimary);
  }
  for (const id of order) {
    if (gridSessions.length >= 4) {
      break;
    }
    const ses = byId[id];
    if (ses && !gridSessions.some((item) => item.id === ses.id)) {
      gridSessions.push(ses);
    }
  }
  const effectiveGridFocusId =
    focusedGridSessionId && gridSessions.some((item) => item.id === focusedGridSessionId)
      ? focusedGridSessionId
      : (gridSessions[0]?.id ?? "");

  if (activeRoute === "automations") {
    return (
      <main className="flex min-w-0 flex-1 flex-col overflow-hidden bg-app">
        <AutomationsPage
          labels={t.pipelines}
          closeLabel={t.sidePanel.close}
          projectLabel={t.newSession.projectLabel}
        />
      </main>
    );
  }
  if (activeRoute === "bots") {
    return (
      <main className="flex min-w-0 flex-1 flex-col overflow-hidden bg-app">
        <BotsPage
          labels={t.pages}
          agentLabels={t.agents}
          newSessionLabel={t.workspace.newSessionCta}
        />
      </main>
    );
  }
  if (activeRoute === "plugins") {
    return (
      <main className="flex min-w-0 flex-1 flex-col overflow-hidden bg-app">
        <PluginsPage labels={t.integrations} />
      </main>
    );
  }
  if (activeRoute === "hosts") {
    return (
      <main className="flex min-w-0 flex-1 flex-col overflow-hidden bg-app">
        <HostsPage
          labels={t.pages}
          repoPlaceholder={t.sidebar.addRepoPlaceholder}
          addRepoLabel={t.sidebar.addRepoButton}
        />
      </main>
    );
  }
  if (activeRoute === "usage") {
    return (
      <main className="flex min-w-0 flex-1 flex-col overflow-hidden bg-app">
        <UsagePage labels={t.pages} />
      </main>
    );
  }
  if (activeRoute === "settings") {
    return (
      <main className="flex min-w-0 flex-1 flex-col overflow-hidden bg-app">
        <SettingsPage
          labels={t.pages}
          connection={t.connection}
          localeToggleLabel={t.localeToggleLabel}
        />
      </main>
    );
  }

  return (
    <main className="flex min-w-0 flex-1 flex-col overflow-hidden bg-app p-[var(--gap-panel)]">
      {error ? (
        <p
          role="alert"
          className="mb-2 rounded-md border border-danger/50 bg-danger/10 px-3 py-1.5 text-xs text-danger"
        >
          {error}
        </p>
      ) : null}

      {!token ? (
        <div className="flex flex-1 items-center justify-center p-6">
          <form
            onSubmit={(ev) => {
              ev.preventDefault();
              setToken(tokenDraft);
            }}
            className="w-full max-w-md space-y-3 rounded-lg border border-line bg-surface p-5 text-xs"
          >
            <h2 className="text-sm font-semibold text-fg">{t.connection.label}</h2>
            <p className="text-fg-3">{t.connection.disconnectNote}</p>
            <div className="flex gap-2">
              <Input
                type="password"
                value={tokenDraft}
                onChange={(e) => setTokenDraft(e.target.value)}
                placeholder={t.connection.tokenPlaceholder}
                aria-label={t.connection.tokenPlaceholder}
              />
              <Button type="submit" variant="primary">
                {t.connection.connectButton}
              </Button>
            </div>
          </form>
        </div>
      ) : !effectivePrimary ? (
        <div className="flex flex-1 flex-col items-center justify-center rounded-lg border border-line bg-surface p-8 text-center">
          <h2 className="text-sm font-semibold text-fg">{t.workspace.emptyTitle}</h2>
          <p className="mt-1 max-w-md text-xs text-fg-3">{t.workspace.emptyBody}</p>
          <Button
            variant="primary"
            className="mt-4"
            onClick={() => setNewSessionOpen(true)}
          >
            {t.workspace.newSessionCta}
          </Button>
        </div>
      ) : splitMode === "grid" ? (
        <Suspense fallback={<p className="p-4 text-xs text-fg-3">{t.workspace.loadingDiff}</p>}>
          <TerminalGrid
            token={token}
            sessions={gridSessions}
            focusedSessionId={effectiveGridFocusId}
            resyncLabel={t.workspace.resync}
            inputPlaceholder={t.sessionActions.composerPlaceholder}
            focusedLabel={t.workspace.focusedCell}
            clickToFocusLabel={t.workspace.clickToFocus}
            expandLabel={t.workspace.expandPane}
            loadingLabel={t.workspace.loadingDiff}
            onFocusSession={(sid) => {
              setFocusedGridSession(sid);
              setActiveSession(sid);
            }}
            onExpandSession={(sid) => {
              setActiveSession(sid);
              setFocusedGridSession(sid);
              setSplitMode("single");
            }}
          />
        </Suspense>
      ) : (
        <SplitLayout
          mode={splitMode}
          primary={
            <SessionPane
              token={token}
              session={effectivePrimary}
              tab={primaryTab}
              onTabChange={setPrimaryTab}
            />
          }
          secondary={
            effectiveSecondary ? (
              <SessionPane
                token={token}
                session={effectiveSecondary}
                tab={secondaryTab}
                onTabChange={setSecondaryTab}
              />
            ) : undefined
          }
        />
      )}
    </main>
  );
}
