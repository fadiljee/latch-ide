import { useState } from "react";
import { StatusDot } from "../../components/StatusDot";
import { Button } from "../../components/ui/Button";
import { Input } from "../../components/ui/Input";
import type { Catalog } from "../../lib/i18n/catalog";
import { useHostsStore } from "../../state/hosts";
import { useIntegrationsStore } from "../../state/integrations";
import { useLayoutStore } from "../../state/layout";
import { useNotificationsStore } from "../../state/notifications";
import { useSessionsStore } from "../../state/sessions";
import { useUiStore } from "../../state/ui";
import {
  cloneGitHubRepo,
  fetchGitHubRepos,
  fetchGitHubStatus,
  startGitHubLogin,
} from "../../transport/daemon-client";

interface GitHubSectionProps {
  labels: Catalog["integrations"];
}

export function GitHubSection({ labels }: GitHubSectionProps) {
  const token = useHostsStore((s) => s.token);
  const addProjectLocal = useHostsStore((s) => s.addProjectLocal);
  const github = useIntegrationsStore((s) => s.github);
  const githubRepos = useIntegrationsStore((s) => s.githubRepos);
  const setGitHub = useIntegrationsStore((s) => s.setGitHub);
  const setGitHubRepos = useIntegrationsStore((s) => s.setGitHubRepos);
  const upsertSession = useSessionsStore((s) => s.upsertOne);
  const setActiveSession = useLayoutStore((s) => s.setActiveSession);
  const setPrimaryTab = useLayoutStore((s) => s.setPrimaryTab);
  const setActiveRoute = useUiStore((s) => s.setActiveRoute);
  const pushToast = useNotificationsStore((s) => s.pushToast);

  const [repoInput, setRepoInput] = useState("");
  const [busyAction, setBusyAction] = useState<string | null>(null);

  const connected = Boolean(token);
  const ghInstalled = github?.gh_installed ?? false;
  const ghAuthenticated = github?.authenticated ?? false;

  const handleOAuthLogin = () => {
    if (!token) {
      return;
    }
    setBusyAction("login");
    void startGitHubLogin(token)
      .then((session) => {
        upsertSession(session);
        setActiveSession(session.id);
        setPrimaryTab("terminal");
        setActiveRoute("workspace");
        pushToast("info", "GitHub OAuth terminal opened");
      })
      .catch((err) =>
        pushToast("danger", err instanceof Error ? err.message : "Failed starting GitHub login"),
      )
      .finally(() => setBusyAction(null));
  };

  const handleLoadRepos = () => {
    if (!token) {
      return;
    }
    setBusyAction("repos");
    void Promise.all([fetchGitHubStatus(token), fetchGitHubRepos(token)])
      .then(([status, repos]) => {
        setGitHub(status);
        setGitHubRepos(repos);
      })
      .catch((err) =>
        pushToast("danger", err instanceof Error ? err.message : "Failed loading GitHub repos"),
      )
      .finally(() => setBusyAction(null));
  };

  const handleClone = async (repo: string) => {
    const clean = repo.trim();
    if (!token || !clean) {
      return;
    }
    setBusyAction(`clone:${clean}`);
    try {
      const project = await cloneGitHubRepo(token, clean);
      addProjectLocal(project);
      if (clean === repoInput.trim()) {
        setRepoInput("");
      }
      pushToast("success", `Project ${project.name} cloned & added`);
    } catch (err) {
      pushToast("danger", err instanceof Error ? err.message : "Failed cloning repository");
    } finally {
      setBusyAction(null);
    }
  };

  const statusTone = !ghInstalled ? "error" : ghAuthenticated ? "done" : "needs_you";
  const statusHeading = !ghInstalled
    ? labels.ghNotInstalled
    : ghAuthenticated
      ? labels.ghConnected
      : labels.ghDisconnected;

  return (
    <div className="space-y-4">
      <div className="rounded-lg border border-line bg-surface p-4 shadow-[var(--panel-shadow)]">
        <div className="flex flex-wrap items-center justify-between gap-3">
          <div className="space-y-1">
            <div className="flex items-center gap-2">
              <StatusDot status={statusTone} />
              <h3 className="text-sm font-semibold text-fg">{statusHeading}</h3>
            </div>
            {github?.account ? (
              <p className="font-mono text-[11px] text-fg-2">
                {github.host} · @{github.account}
                {github.protocol ? ` · ${github.protocol}` : ""}
              </p>
            ) : github?.gh_path ? (
              <p className="font-mono text-[11px] text-fg-3">{github.gh_path}</p>
            ) : null}
            {github && github.scopes.length > 0 ? (
              <div className="flex flex-wrap gap-1 pt-1">
                {github.scopes.map((scope) => (
                  <span
                    key={scope}
                    className="rounded-sm border border-line bg-raised px-1.5 py-0.5 font-mono text-[10px] text-fg-2"
                  >
                    {scope}
                  </span>
                ))}
              </div>
            ) : null}
          </div>

          <div className="flex flex-wrap items-center gap-2">
            <Button
              variant="primary"
              size="sm"
              disabled={!connected || !ghInstalled || busyAction !== null}
              onClick={handleOAuthLogin}
            >
              {labels.ghConnectOauth}
            </Button>
            <Button
              variant="secondary"
              size="sm"
              disabled={!connected || !ghInstalled || busyAction !== null}
              onClick={handleLoadRepos}
            >
              {labels.ghRefreshRepos}
            </Button>
          </div>
        </div>
      </div>

      <form
        onSubmit={(e) => {
          e.preventDefault();
          void handleClone(repoInput);
        }}
        className="flex flex-wrap gap-2"
      >
        <Input
          type="text"
          value={repoInput}
          onChange={(e) => setRepoInput(e.target.value)}
          placeholder={labels.ghClonePlaceholder}
          aria-label={labels.ghClonePlaceholder}
          className="min-w-52 flex-1"
          disabled={!connected || !ghInstalled || busyAction !== null}
        />
        <Button
          type="submit"
          variant="primary"
          disabled={!connected || !ghInstalled || busyAction !== null || !repoInput.trim()}
        >
          {labels.ghCloneButton}
        </Button>
      </form>

      {githubRepos.length === 0 ? (
        <p role="status" className="rounded-lg border border-line bg-surface p-4 text-fg-3">
          {labels.ghNoRepos}
        </p>
      ) : (
        <ul className="space-y-2">
          {githubRepos.map((repo) => (
            <li
              key={repo.name_with_owner}
              className="flex flex-wrap items-center justify-between gap-3 rounded-lg border border-line bg-surface px-4 py-3"
            >
              <div className="min-w-0 space-y-0.5">
                <div className="flex flex-wrap items-center gap-2">
                  <span className="font-mono font-semibold text-fg">{repo.name_with_owner}</span>
                  <span className="rounded-sm border border-line bg-raised px-1.5 py-0.5 font-mono text-[10px] text-fg-2">
                    {repo.is_private ? "private" : "public"}
                  </span>
                  {repo.default_branch ? (
                    <span className="font-mono text-[10px] text-fg-3">{repo.default_branch}</span>
                  ) : null}
                </div>
                {repo.description ? (
                  <p className="truncate text-fg-2">{repo.description}</p>
                ) : null}
              </div>

              <Button
                size="sm"
                variant="secondary"
                disabled={!connected || busyAction !== null}
                onClick={() => void handleClone(repo.name_with_owner)}
              >
                {labels.ghCloneAndAdd}
              </Button>
            </li>
          ))}
        </ul>
      )}
    </div>
  );
}
