import { useState } from "react";
import type { Project } from "../../lib/api/types";
import type { Catalog } from "../../lib/i18n/catalog";

interface NewSessionModalProps {
  projects: Project[];
  labels: Catalog["nav"];
  onAddRepo: (repoPath: string) => Promise<void>;
  onCreate: (params: { project: string; profile: string; task: string }) => Promise<void>;
  onClose: () => void;
}

const DEFAULT_PROFILES = ["claude", "codex", "gemini", "opencode", "shell", "sleeper"];

export function NewSessionModal({
  projects,
  labels,
  onAddRepo,
  onCreate,
  onClose,
}: NewSessionModalProps) {
  const [project, setProject] = useState(projects[0]?.name ?? "");
  const [profile, setProfile] = useState("claude");
  const [task, setTask] = useState("");
  const [repoPath, setRepoPath] = useState("");
  const [busy, setBusy] = useState(false);

  const selectedProject = project || projects[0]?.name || "";

  const handleSubmit = async (ev: React.FormEvent) => {
    ev.preventDefault();
    if (!selectedProject || !task.trim()) {
      return;
    }
    setBusy(true);
    try {
      await onCreate({ project: selectedProject, profile, task: task.trim() });
      onClose();
    } finally {
      setBusy(false);
    }
  };

  const handleAddRepo = async () => {
    if (!repoPath.trim()) {
      return;
    }
    setBusy(true);
    try {
      await onAddRepo(repoPath.trim());
      setRepoPath("");
    } finally {
      setBusy(false);
    }
  };

  return (
    <div
      role="dialog"
      aria-modal="true"
      aria-labelledby="new-session-title"
      className="fixed inset-0 z-50 flex items-center justify-center bg-black/70 p-4 backdrop-blur-sm"
    >
      <div className="w-full max-w-lg rounded-2xl border border-slate-700/80 bg-[#121624] p-5 shadow-2xl">
        <h2 id="new-session-title" className="text-base font-semibold text-slate-100">
          {labels.createTitle}
        </h2>

        {projects.length === 0 ? (
          <div className="mt-4 flex gap-2">
            <input
              type="text"
              value={repoPath}
              onChange={(e) => setRepoPath(e.target.value)}
              placeholder={labels.addRepoPlaceholder}
              aria-label={labels.addRepoPlaceholder}
              className="min-w-0 flex-1 rounded-lg border border-slate-700 bg-[#0b0e17] px-3 py-2 text-xs text-slate-100 placeholder:text-slate-500 focus:border-cyan-500 focus:outline-none"
            />
            <button
              type="button"
              disabled={busy}
              onClick={() => void handleAddRepo()}
              className="rounded-lg bg-slate-800 px-3 py-2 text-xs font-medium text-slate-100 hover:bg-slate-700"
            >
              {labels.addRepoButton}
            </button>
          </div>
        ) : null}

        <form onSubmit={(e) => void handleSubmit(e)} className="mt-4 space-y-3 text-xs">
          <div>
            <label htmlFor="ns-project" className="mb-1 block text-slate-400">
              {labels.projectLabel}
            </label>
            <select
              id="ns-project"
              value={selectedProject}
              onChange={(e) => setProject(e.target.value)}
              className="w-full rounded-lg border border-slate-700 bg-[#0b0e17] px-3 py-2 text-slate-100 focus:border-cyan-500 focus:outline-none"
            >
              {projects.map((p) => (
                <option key={p.id} value={p.name}>
                  {p.name} ({p.default_branch})
                </option>
              ))}
            </select>
          </div>

          <div>
            <label htmlFor="ns-profile" className="mb-1 block text-slate-400">
              {labels.profileLabel}
            </label>
            <select
              id="ns-profile"
              value={profile}
              onChange={(e) => setProfile(e.target.value)}
              className="w-full rounded-lg border border-slate-700 bg-[#0b0e17] px-3 py-2 text-slate-100 focus:border-cyan-500 focus:outline-none"
            >
              {DEFAULT_PROFILES.map((p) => (
                <option key={p} value={p}>
                  {p}
                </option>
              ))}
            </select>
          </div>

          <div>
            <label htmlFor="ns-task" className="mb-1 block text-slate-400">
              {labels.taskLabel}
            </label>
            <textarea
              id="ns-task"
              rows={3}
              value={task}
              onChange={(e) => setTask(e.target.value)}
              placeholder={labels.taskPlaceholder}
              className="w-full rounded-lg border border-slate-700 bg-[#0b0e17] px-3 py-2 text-slate-100 placeholder:text-slate-500 focus:border-cyan-500 focus:outline-none"
            />
          </div>

          <div className="flex justify-end gap-2 pt-2">
            <button
              type="button"
              onClick={onClose}
              className="rounded-lg border border-slate-700 px-3 py-1.5 text-slate-300 hover:bg-slate-800"
            >
              {labels.cancel}
            </button>
            <button
              type="submit"
              disabled={busy || !selectedProject || !task.trim()}
              className="rounded-lg bg-cyan-600 px-4 py-1.5 font-medium text-white hover:bg-cyan-500 disabled:opacity-50"
            >
              {labels.launch}
            </button>
          </div>
        </form>
      </div>
    </div>
  );
}

export default NewSessionModal;
