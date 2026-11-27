import { lazy, Suspense, useState } from "react";
import { Button } from "../../components/ui/Button";
import { Input } from "../../components/ui/Input";
import type { Catalog } from "../../lib/i18n/catalog";
import { useHostsStore } from "../../state/hosts";
import { useNotificationsStore } from "../../state/notifications";
import { useUsageStore } from "../../state/usage";
import {
  cancelPipeline,
  fetchPipelineArena,
  runPipeline,
  selectPipelineArenaWinner,
} from "../../transport/daemon-client";
import type { PipelineRun } from "../../transport/protocol";

const CompareView = lazy(() => import("../review/CompareView"));

interface AutomationsPageProps {
  labels: Catalog["pipelines"];
  closeLabel: string;
  projectLabel: string;
}

const SAMPLE_WORKFLOW = `[workflow]
name = "verify-and-build"
max_parallel = 2

[[steps]]
id = "check"
type = "shell"
command = "git status -s"
`;

export function AutomationsPage({ labels, closeLabel, projectLabel }: AutomationsPageProps) {
  const token = useHostsStore((s) => s.token);
  const projects = useHostsStore((s) => s.projects);
  const pipelines = useUsageStore((s) => s.pipelines);
  const activeArena = useUsageStore((s) => s.activeArena);
  const upsertPipeline = useUsageStore((s) => s.upsertPipeline);
  const setActiveArena = useUsageStore((s) => s.setActiveArena);
  const pushToast = useNotificationsStore((s) => s.pushToast);

  const [project, setProject] = useState("");
  const [task, setTask] = useState("");
  const [workflowToml, setWorkflowToml] = useState(SAMPLE_WORKFLOW);
  const [busy, setBusy] = useState(false);

  const selectedProject = project || projects[0]?.name || "";

  const handleRun = async (ev: React.FormEvent) => {
    ev.preventDefault();
    if (!token || !selectedProject || !task.trim()) {
      return;
    }
    setBusy(true);
    try {
      const run = await runPipeline(token, {
        project: selectedProject,
        workflow_toml: workflowToml,
        task: task.trim(),
      });
      upsertPipeline(run);
      setTask("");
      pushToast("success", `Pipeline ${run.workflow_name} started`);
    } catch (err) {
      pushToast("danger", err instanceof Error ? err.message : "Failed starting pipeline");
    } finally {
      setBusy(false);
    }
  };

  const handleInspect = (run: PipelineRun) => {
    if (!token) {
      return;
    }
    void fetchPipelineArena(token, run.id)
      .then((arena) => setActiveArena(arena))
      .catch((err) => pushToast("danger", err instanceof Error ? err.message : "Failed loading arena"));
  };

  const handleCancel = (id: string) => {
    if (!token) {
      return;
    }
    void cancelPipeline(token, id)
      .then((updated) => upsertPipeline(updated))
      .catch((err) => pushToast("danger", err instanceof Error ? err.message : "Cancel failed"));
  };

  const handleSelectWinner = (pipelineId: string, stepId: string, candidateId: string) => {
    if (!token) {
      return;
    }
    void selectPipelineArenaWinner(token, pipelineId, stepId, candidateId)
      .then(() => fetchPipelineArena(token, pipelineId))
      .then((arena) => setActiveArena(arena))
      .catch((err) => pushToast("danger", err instanceof Error ? err.message : "Select failed"));
  };

  return (
    <div className="flex-1 space-y-6 overflow-y-auto p-5">
      <div className="flex items-center justify-between">
        <h2 className="text-base font-semibold text-fg">{labels.heading}</h2>
        <span className="rounded-sm border border-line bg-raised px-2 py-0.5 font-mono text-xs text-fg-2">
          {pipelines.length}
        </span>
      </div>

      {pipelines.length === 0 ? (
        <p className="rounded-lg border border-line bg-surface p-4 text-xs text-fg-3">
          {labels.empty}
        </p>
      ) : (
        <ul className="grid grid-cols-1 gap-3 md:grid-cols-2">
          {pipelines.map((run) => (
            <li
              key={run.id}
              className="rounded-lg border border-line bg-surface p-3.5 text-xs shadow-[var(--panel-shadow)]"
            >
              <div className="flex items-center justify-between gap-2">
                <span className="font-mono font-semibold text-fg">{run.workflow_name}</span>
                <span className="rounded-sm border border-line bg-raised px-2 py-0.5 font-mono text-[11px] text-fg-2">
                  {run.status}
                </span>
              </div>
              <p className="mt-1 truncate text-fg-2">{run.task ?? run.id.slice(0, 8)}</p>
              <div className="mt-3 flex flex-wrap items-center justify-between gap-2">
                <span className="font-mono text-[11px] text-fg-3">
                  {labels.cost}: ${run.total_cost_usd.toFixed(4)}
                </span>
                <div className="flex items-center gap-2">
                  <Button size="sm" onClick={() => handleInspect(run)}>
                    {labels.viewArena}
                  </Button>
                  {run.status === "running" || run.status === "pending" ? (
                    <Button size="sm" variant="danger" onClick={() => handleCancel(run.id)}>
                      {labels.cancel}
                    </Button>
                  ) : null}
                </div>
              </div>
            </li>
          ))}
        </ul>
      )}

      {activeArena ? (
        <Suspense fallback={null}>
          <CompareView
            arena={activeArena}
            labels={labels}
            closeLabel={closeLabel}
            onSelectWinner={handleSelectWinner}
            onClose={() => setActiveArena(null)}
          />
        </Suspense>
      ) : null}

      <form
        onSubmit={(e) => void handleRun(e)}
        className="space-y-3 rounded-lg border border-line bg-surface p-4 text-xs"
      >
        <h3 className="font-semibold text-fg">{labels.launchTitle}</h3>
        <div className="grid grid-cols-1 gap-3 sm:grid-cols-2">
          <label htmlFor="pipe-proj" className="block">
            <span className="mb-1 block text-fg-2">{projectLabel}</span>
            <select
              id="pipe-proj"
              value={selectedProject}
              onChange={(e) => setProject(e.target.value)}
              className="w-full rounded-md border border-line bg-app px-2.5 py-1.5 text-xs text-fg focus:border-accent focus:outline-none"
            >
              {projects.map((p) => (
                <option key={p.id} value={p.name}>
                  {p.name}
                </option>
              ))}
            </select>
          </label>
          <label htmlFor="pipe-task" className="block">
            <span className="mb-1 block text-fg-2">Task</span>
            <Input
              id="pipe-task"
              type="text"
              value={task}
              onChange={(e) => setTask(e.target.value)}
              placeholder={labels.taskPlaceholder}
            />
          </label>
        </div>
        <label htmlFor="pipe-toml" className="block">
          <span className="mb-1 block text-fg-2">workflow.toml</span>
          <textarea
            id="pipe-toml"
            rows={5}
            value={workflowToml}
            onChange={(e) => setWorkflowToml(e.target.value)}
            placeholder={labels.workflowPlaceholder}
            className="w-full rounded-md border border-line bg-app p-2.5 font-mono text-[11px] text-fg placeholder:text-fg-3 focus:border-accent focus:outline-none"
          />
        </label>
        <div className="flex justify-end">
          <Button
            type="submit"
            variant="primary"
            disabled={busy || !token || !selectedProject || !task.trim()}
          >
            {labels.runButton}
          </Button>
        </div>
      </form>
    </div>
  );
}
