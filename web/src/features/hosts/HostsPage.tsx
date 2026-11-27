import { useState } from "react";
import { Button } from "../../components/ui/Button";
import { Input } from "../../components/ui/Input";
import { StatusDot } from "../../components/StatusDot";
import type { Catalog } from "../../lib/i18n/catalog";
import { useHostsStore } from "../../state/hosts";
import { useNotificationsStore } from "../../state/notifications";
import { addProject } from "../../transport/daemon-client";

interface HostsPageProps {
  labels: Catalog["pages"];
  repoPlaceholder: string;
  addRepoLabel: string;
}

export function HostsPage({ labels, repoPlaceholder, addRepoLabel }: HostsPageProps) {
  const health = useHostsStore((s) => s.health);
  const connected = useHostsStore((s) => s.connected);
  const projects = useHostsStore((s) => s.projects);
  const token = useHostsStore((s) => s.token);
  const addProjectLocal = useHostsStore((s) => s.addProjectLocal);
  const pushToast = useNotificationsStore((s) => s.pushToast);
  const [repoPath, setRepoPath] = useState("");
  const [busy, setBusy] = useState(false);

  const handleAdd = async (ev: React.FormEvent) => {
    ev.preventDefault();
    if (!token || !repoPath.trim()) {
      return;
    }
    setBusy(true);
    try {
      const added = await addProject(token, repoPath.trim());
      addProjectLocal(added);
      setRepoPath("");
      pushToast("success", `Project ${added.name} added`);
    } catch (err) {
      pushToast("danger", err instanceof Error ? err.message : "Failed adding project");
    } finally {
      setBusy(false);
    }
  };

  return (
    <div className="flex-1 space-y-6 overflow-y-auto p-5 text-xs">
      <div>
        <h2 className="text-base font-semibold text-fg">{labels.hostsHeading}</h2>
        <p className="mt-0.5 text-fg-3">{labels.hostsSubtitle}</p>
      </div>

      <div className="rounded-lg border border-line bg-surface p-4">
        <div className="flex items-center justify-between">
          <div className="flex items-center gap-2">
            <StatusDot status={connected ? "done" : "needs_you"} />
            <span className="font-medium text-fg">Local Daemon (127.0.0.1)</span>
          </div>
          <span className="font-mono text-fg-2">
            {health ? `v${health.version} · PID ${health.pid}` : "offline"}
          </span>
        </div>
        <div className="mt-3 grid grid-cols-2 gap-3 font-mono text-fg-2 sm:grid-cols-3">
          <div>
            Sandbox: <span className="text-fg">{health?.sandbox ?? "-"}</span>
          </div>
          <div>
            Active PTYs: <span className="text-fg">{health?.active_sessions ?? 0}</span>
          </div>
          <div>
            Projects: <span className="text-fg">{projects.length}</span>
          </div>
        </div>
        <p className="mt-3 rounded-md border border-line bg-app px-3 py-2 font-mono text-fg-2">
          {labels.remoteHint}
        </p>
      </div>

      <form onSubmit={(e) => void handleAdd(e)} className="flex flex-wrap gap-2">
        <Input
          type="text"
          value={repoPath}
          onChange={(e) => setRepoPath(e.target.value)}
          placeholder={repoPlaceholder}
          aria-label={repoPlaceholder}
          className="flex-1"
        />
        <Button type="submit" variant="primary" disabled={busy || !token || !repoPath.trim()}>
          {addRepoLabel}
        </Button>
      </form>

      <ul className="space-y-2">
        {projects.map((p) => (
          <li
            key={p.id}
            className="flex flex-wrap items-center justify-between gap-2 rounded-md border border-line bg-surface px-3 py-2.5"
          >
            <div>
              <p className="font-medium text-fg">{p.name}</p>
              <p className="font-mono text-[11px] text-fg-3">{p.repo_path}</p>
            </div>
            <span className="rounded-sm border border-line bg-raised px-2 py-0.5 font-mono text-[11px] text-fg-2">
              {p.default_branch}
            </span>
          </li>
        ))}
      </ul>
    </div>
  );
}
