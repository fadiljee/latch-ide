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
import {
  addProject,
  createSession,
  fetchAntigravityCliInfo,
} from "../../transport/daemon-client";
import type { AntigravityCliInfo } from "../../transport/protocol";
import { ModelPicker } from "./ModelPicker";

const FALLBACK_AGY_MODES = ["auto", "accept-edits", "plan"] as const;
const FALLBACK_AGY_EFFORTS = ["low", "medium", "high", "xhigh", "max"] as const;

export function buildAntigravityArgs(opts: {
  mode: string;
  effort: string;
  model: string;
  continueLast: boolean;
}): string[] {
  const args: string[] = [];
  if (opts.mode.trim()) {
    args.push("--mode", opts.mode.trim());
  }
  if (opts.effort.trim()) {
    args.push("--effort", opts.effort.trim());
  }
  if (opts.model.trim()) {
    args.push("--model", opts.model.trim());
  }
  if (opts.continueLast) {
    args.push("--continue");
  }
  return args;
}

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
  const [agyInfo, setAgyInfo] = useState<AntigravityCliInfo | null>(null);
  const [agyMode, setAgyMode] = useState("");
  const [agyEffort, setAgyEffort] = useState("");
  const [agyModel, setAgyModel] = useState("");
  const [agyContinue, setAgyContinue] = useState(false);
  const [busy, setBusy] = useState(false);

  useEffect(() => {
    if (open) {
      setProfile(defaultProfile);
    }
  }, [open, defaultProfile]);

  useEffect(() => {
    if (!open || !token || profile !== "antigravity") {
      return;
    }
    let cancelled = false;
    void fetchAntigravityCliInfo(token)
      .then((info) => {
        if (!cancelled) {
          setAgyInfo(info);
        }
      })
      .catch(() => {
        // Gunakan opsi bawaan bila endpoint info belum tersedia
      });
    return () => {
      cancelled = true;
    };
  }, [open, token, profile]);

  const selectedProject = project || projects[0]?.name || "";
  const isAntigravity = profile === "antigravity";
  const agyModes = agyInfo?.modes.length ? agyInfo.modes : [...FALLBACK_AGY_MODES];
  const agyEfforts = agyInfo?.efforts.length ? agyInfo.efforts : [...FALLBACK_AGY_EFFORTS];
  const agyModels = agyInfo?.models ?? [];

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
    if (!token || !selectedProject) {
      return;
    }
    setBusy(true);
    try {
      const extraArgs = isAntigravity
        ? buildAntigravityArgs({
            mode: agyMode,
            effort: agyEffort,
            model: agyModel,
            continueLast: agyContinue,
          })
        : [];
      const created = await createSession(token, {
        project: selectedProject,
        profile,
        task: task.trim(),
        base_ref: baseRef.trim() || undefined,
        extra_args: extraArgs,
      });
      upsertOne(created);
      setActiveSession(created.id);
      setActiveRoute("workspace");
      setTask("");
      setBaseRef("");
      setAgyMode("");
      setAgyEffort("");
      setAgyModel("");
      setAgyContinue(false);
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

        {isAntigravity ? (
          <fieldset className="space-y-2 rounded-md border border-line bg-app p-3">
            <legend className="flex items-center gap-2 px-1 text-[11px] font-semibold text-fg">
              <span>{labels.agyOptionsHeading}</span>
              {agyInfo?.version ? (
                <span className="rounded-sm border border-done/40 bg-done/15 px-1.5 py-0.5 font-mono text-[10px] text-done">
                  v{agyInfo.version}
                </span>
              ) : null}
            </legend>
            <div className="grid grid-cols-1 gap-2 sm:grid-cols-2">
              <label htmlFor="new-ses-agy-mode" className="block">
                <span className="mb-1 block text-[11px] text-fg-2">{labels.agyModeLabel}</span>
                <select
                  id="new-ses-agy-mode"
                  value={agyMode}
                  onChange={(e) => setAgyMode(e.target.value)}
                  className="w-full rounded-md border border-line bg-surface px-2 py-1 text-xs text-fg focus:border-accent focus:outline-none"
                >
                  <option value="">{labels.agyDefaultOption}</option>
                  {agyModes.map((m) => (
                    <option key={m} value={m}>
                      {m}
                    </option>
                  ))}
                </select>
              </label>

              <label htmlFor="new-ses-agy-effort" className="block">
                <span className="mb-1 block text-[11px] text-fg-2">{labels.agyEffortLabel}</span>
                <select
                  id="new-ses-agy-effort"
                  value={agyEffort}
                  onChange={(e) => setAgyEffort(e.target.value)}
                  className="w-full rounded-md border border-line bg-surface px-2 py-1 text-xs text-fg focus:border-accent focus:outline-none"
                >
                  <option value="">{labels.agyDefaultOption}</option>
                  {agyEfforts.map((ef) => (
                    <option key={ef} value={ef}>
                      {ef}
                    </option>
                  ))}
                </select>
              </label>
            </div>

            <label htmlFor="new-ses-agy-model" className="block">
              <span className="mb-1 block text-[11px] text-fg-2">{labels.agyModelLabel}</span>
              {agyModels.length > 0 ? (
                <select
                  id="new-ses-agy-model"
                  value={agyModel}
                  onChange={(e) => setAgyModel(e.target.value)}
                  className="w-full rounded-md border border-line bg-surface px-2 py-1 text-xs text-fg focus:border-accent focus:outline-none"
                >
                  <option value="">{labels.agyDefaultOption}</option>
                  {agyModels.map((m) => (
                    <option key={m.id} value={m.id}>
                      {m.label} ({m.id})
                    </option>
                  ))}
                </select>
              ) : (
                <Input
                  id="new-ses-agy-model"
                  type="text"
                  value={agyModel}
                  onChange={(e) => setAgyModel(e.target.value)}
                  placeholder="gemini-4-neon-high"
                />
              )}
            </label>

            <label className="inline-flex items-center gap-2 text-[11px] text-fg-2">
              <input
                type="checkbox"
                checked={agyContinue}
                onChange={(e) => setAgyContinue(e.target.checked)}
                className="accent-[var(--accent)]"
              />
              <span>{labels.agyContinueLabel}</span>
            </label>
          </fieldset>
        ) : null}

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
          <span className="mt-1 block text-[11px] text-fg-3">{labels.taskOptionalHint}</span>
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
          <Button type="submit" variant="primary" disabled={busy || !selectedProject}>
            {labels.launch}
          </Button>
        </div>
      </form>
    </Dialog>
  );
}
