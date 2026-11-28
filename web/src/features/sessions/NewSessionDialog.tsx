import { useEffect, useState } from "react";
import { Button } from "../../components/ui/Button";
import { Dialog } from "../../components/ui/Dialog";
import { Input } from "../../components/ui/Input";
import type { Catalog } from "../../lib/i18n/catalog";
import { useHostsStore } from "../../state/hosts";
import { useIntegrationsStore } from "../../state/integrations";
import { useLayoutStore } from "../../state/layout";
import { useNotificationsStore } from "../../state/notifications";
import { useSessionsStore } from "../../state/sessions";
import { useUiStore } from "../../state/ui";
import { addProject, createSession } from "../../transport/daemon-client";
import { ModelPicker } from "./ModelPicker";

interface NewSessionDialogProps {
  labels: Catalog["newSession"];
  repoPlaceholder: string;
  addRepoLabel: string;
}

export function NewSessionDialog({
  labels,
  repoPlaceholder,
  addRepoLabel,
}: NewSessionDialogProps) {
  const open = useUiStore((s) => s.newSessionOpen);
  const setOpen = useUiStore((s) => s.setNewSessionOpen);
  const setActiveRoute = useUiStore((s) => s.setActiveRoute);
  const projects = useHostsStore((s) => s.projects);
  const token = useHostsStore((s) => s.token);
  const addProjectLocal = useHostsStore((s) => s.addProjectLocal);
  const agents = useIntegrationsStore((s) => s.agents);
  const preferredProfile = useIntegrationsStore((s) => s.preferredProfile);
  const setPreferredProfile = useIntegrationsStore((s) => s.setPreferredProfile);
  const upsertOne = useSessionsStore((s) => s.upsertOne);
  const setActiveSession = useLayoutStore((s) => s.setActiveSession);
  const pushToast = useNotificationsStore((s) => s.pushToast);

  const defaultProfile =
    preferredProfile ||
    agents.find((a) => a.installed && a.profile.category !== "shell")?.id ||
    agents[0]?.id ||
    "antigravity";

  const [project, setProject] = useState("");
  const [profile, setProfile] = useState(defaultProfile);
  const [task, setTask] = useState("");
  const [baseRef, setBaseRef] = useState("");
  const [repoPath, setRepoPath] = useState("");
  const [busy, setBusy] = useState(false);

  useEffect(() => {
    if (open) {
      setProfile(defaultProfile);
    }
  }, [open, defaultProfile]);

  const selectedProject = project || projects[0]?.name || "";

  const handleAddRepo = async () => {
    if (!token || !repoPath.trim()) {
      return;
    }
    setBusy(true);
    try {
      const added = await addProject(token, repoPath.trim());
      addProjectLocal(added);
      setProject(added.name);
      setRepoPath("");
      pushToast("success", `Project ${added.name} registered`);
    } catch (err) {
      pushToast("danger", err instanceof Error ? err.message : "Failed adding repo");
    } finally {
      setBusy(false);
    }
  };

  const handleSubmit = async (ev: React.FormEvent) => {
    ev.preventDefault();
    if (!token || !selectedProject || !task.trim()) {
      return;
    }
    setBusy(true);
    try {
      const created = await createSession(token, {
        project: selectedProject,
        profile,
        task: task.trim(),
        base_ref: baseRef.trim() || undefined,
      });
      upsertOne(created);
      setActiveSession(created.id);
      setActiveRoute("workspace");
      setTask("");
      setBaseRef("");
      setPreferredProfile(null);
      setOpen(false);
    } catch (err) {
      pushToast("danger", err instanceof Error ? err.message : "Failed starting session");
    } finally {
      setBusy(false);
    }
  };

  return (
    <Dialog
      open={open}
      title={labels.title}
      titleId="new-session-dialog-title"
      onClose={() => setOpen(false)}
    >
      {projects.length === 0 ? (
        <div className="mb-4 rounded-md border border-line bg-app p-3 text-xs">
          <p className="mb-2 text-fg-2">{labels.needRepo}</p>
          <div className="flex gap-2">
            <Input
              type="text"
              value={repoPath}
              onChange={(e) => setRepoPath(e.target.value)}
              placeholder={repoPlaceholder}
              aria-label={repoPlaceholder}
            />
            <Button disabled={busy || !repoPath.trim()} onClick={() => void handleAddRepo()}>
              {addRepoLabel}
            </Button>
          </div>
        </div>
      ) : null}

      <form onSubmit={(e) => void handleSubmit(e)} className="space-y-3 text-xs">
        <label htmlFor="new-ses-project" className="block">
          <span className="mb-1 block text-fg-2">{labels.projectLabel}</span>
          <select
            id="new-ses-project"
            value={selectedProject}
            onChange={(e) => setProject(e.target.value)}
            className="w-full rounded-md border border-line bg-app px-2.5 py-1.5 text-xs text-fg focus:border-accent focus:outline-none"
          >
            {projects.map((p) => (
              <option key={p.id} value={p.name}>
                {p.name} ({p.default_branch})
              </option>
            ))}
          </select>
        </label>

        <ModelPicker
          id="new-ses-profile"
          label={labels.profileLabel}
          value={profile}
          onChange={setProfile}
        />

        <label htmlFor="new-ses-task" className="block">
          <span className="mb-1 block text-fg-2">{labels.taskLabel}</span>
          <textarea
            id="new-ses-task"
            rows={3}
            value={task}
            onChange={(e) => setTask(e.target.value)}
            placeholder={labels.taskPlaceholder}
            className="w-full rounded-md border border-line bg-app px-2.5 py-1.5 text-xs text-fg placeholder:text-fg-3 focus:border-accent focus:outline-none"
          />
        </label>

        <label htmlFor="new-ses-baseref" className="block">
          <span className="mb-1 block text-fg-2">{labels.baseRefLabel}</span>
          <Input
            id="new-ses-baseref"
            type="text"
            value={baseRef}
            onChange={(e) => setBaseRef(e.target.value)}
            placeholder={labels.baseRefPlaceholder}
          />
        </label>

        <div className="flex justify-end gap-2 pt-2">
          <Button variant="ghost" onClick={() => setOpen(false)}>
            {labels.cancel}
          </Button>
          <Button type="submit" variant="primary" disabled={busy || !selectedProject || !task.trim()}>
            {labels.launch}
          </Button>
        </div>
      </form>
    </Dialog>
  );
}
