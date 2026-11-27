import type { PipelineArenaResponse, PipelineRun } from "../../lib/api/types";
import type { Catalog } from "../../lib/i18n/catalog";

interface PipelineArenaPanelProps {
  pipelines: PipelineRun[];
  activeArena: PipelineArenaResponse | null;
  labels: Catalog["pipelines"];
  closeLabel: string;
  onInspectArena: (pipeline: PipelineRun) => void;
  onCancelPipeline: (pipelineId: string) => void;
  onSelectWinner: (pipelineId: string, stepId: string, candidateId: string) => void;
  onCloseArena: () => void;
}

export function PipelineArenaPanel({
  pipelines,
  activeArena,
  labels,
  closeLabel,
  onInspectArena,
  onCancelPipeline,
  onSelectWinner,
  onCloseArena,
}: PipelineArenaPanelProps) {
  return (
    <section aria-labelledby="pipelines-heading" className="mt-8">
      <div className="mb-3 flex items-center justify-between">
        <h2 id="pipelines-heading" className="text-base font-medium text-slate-200">
          {labels.heading}
        </h2>
        <span className="rounded bg-slate-800 px-2 py-0.5 text-xs text-slate-300">
          {pipelines.length}
        </span>
      </div>

      {pipelines.length === 0 ? (
        <p className="rounded-lg border border-slate-800 bg-slate-900/60 p-4 text-xs text-slate-400">
          {labels.empty}
        </p>
      ) : (
        <ul className="grid grid-cols-1 gap-3 md:grid-cols-2">
          {pipelines.map((run) => (
            <li
              key={run.id}
              className="rounded-lg border border-slate-800 bg-slate-900/60 p-3 text-xs"
            >
              <div className="flex items-center justify-between gap-2">
                <span className="font-mono font-medium text-cyan-300">{run.workflow_name}</span>
                <span className="rounded bg-slate-800 px-2 py-0.5 text-[11px] uppercase text-slate-200">
                  {run.status}
                </span>
              </div>
              <p className="mt-1 truncate text-slate-300">{run.task ?? run.id.slice(0, 8)}</p>
              <div className="mt-3 flex flex-wrap items-center justify-between gap-2">
                <span className="font-mono text-[11px] text-slate-400">
                  {labels.cost}: ${run.total_cost_usd.toFixed(4)}
                </span>
                <div className="flex items-center gap-2">
                  <button
                    type="button"
                    onClick={() => onInspectArena(run)}
                    className="rounded border border-slate-700 px-2.5 py-1 text-[11px] text-slate-200 hover:bg-slate-800 focus-visible:outline-2 focus-visible:outline-cyan-400"
                  >
                    {labels.viewArena}
                  </button>
                  {run.status === "running" || run.status === "pending" ? (
                    <button
                      type="button"
                      onClick={() => onCancelPipeline(run.id)}
                      className="rounded bg-rose-700 px-2.5 py-1 text-[11px] font-medium text-white hover:bg-rose-600 focus-visible:outline-2 focus-visible:outline-rose-400"
                    >
                      {labels.cancel}
                    </button>
                  ) : null}
                </div>
              </div>
            </li>
          ))}
        </ul>
      )}

      {activeArena ? (
        <div
          role="region"
          aria-label={labels.arenaHeading}
          className="mt-4 rounded-lg border border-cyan-900/70 bg-slate-900/90 p-4"
        >
          <div className="mb-3 flex flex-wrap items-center justify-between gap-2">
            <div>
              <h3 className="text-sm font-semibold text-cyan-200">
                {labels.arenaHeading} · {activeArena.workflow_name}
              </h3>
              <p className="font-mono text-[11px] text-slate-400">
                {activeArena.pipeline_id.slice(0, 8)} · {activeArena.status}
              </p>
            </div>
            <button
              type="button"
              onClick={onCloseArena}
              className="rounded border border-slate-700 px-2.5 py-1 text-xs text-slate-200 hover:bg-slate-800 focus-visible:outline-2 focus-visible:outline-cyan-400"
            >
              {closeLabel}
            </button>
          </div>

          {activeArena.steps.length === 0 ? (
            <p className="text-xs text-slate-400">{labels.noArenaSteps}</p>
          ) : (
            <div className="space-y-4">
              {activeArena.steps.map((step) => (
                <div key={step.step_id} className="overflow-x-auto">
                  <div className="mb-2 flex items-center justify-between text-xs">
                    <span className="font-mono font-semibold text-slate-200">{step.step_id}</span>
                    {step.winner ? (
                      <span className="rounded bg-emerald-900/60 px-2 py-0.5 text-[11px] text-emerald-200">
                        {labels.winner}: {step.winner}
                      </span>
                    ) : null}
                  </div>
                  <table className="w-full min-w-[480px] border-collapse text-left text-xs">
                    <thead>
                      <tr className="border-b border-slate-800 text-slate-400">
                        <th scope="col" className="py-1.5 pr-3">
                          {labels.profile}
                        </th>
                        <th scope="col" className="py-1.5 pr-3">
                          {labels.status}
                        </th>
                        <th scope="col" className="py-1.5 pr-3">
                          {labels.duration}
                        </th>
                        <th scope="col" className="py-1.5 pr-3">
                          {labels.diffLines}
                        </th>
                        <th scope="col" className="py-1.5 pr-3">
                          {labels.cost}
                        </th>
                        <th scope="col" className="py-1.5">
                          {labels.action}
                        </th>
                      </tr>
                    </thead>
                    <tbody>
                      {step.candidates.map((cand) => {
                        const isWinner = step.winner === cand.profile;
                        return (
                          <tr
                            key={`${step.step_id}-${cand.profile}`}
                            className="border-b border-slate-800/70"
                          >
                            <td className="py-2 pr-3 font-mono text-slate-100">
                              {cand.profile}
                              {isWinner ? (
                                <span className="ml-2 rounded bg-emerald-900/70 px-1.5 py-0.5 text-[10px] text-emerald-200">
                                  {labels.winner}
                                </span>
                              ) : null}
                            </td>
                            <td className="py-2 pr-3">
                              <span
                                className={
                                  cand.passed ? "text-emerald-400" : "text-rose-400"
                                }
                              >
                                {cand.passed ? labels.passed : labels.failed}
                              </span>
                            </td>
                            <td className="py-2 pr-3 font-mono text-slate-300">
                              {cand.duration_ms} ms
                            </td>
                            <td className="py-2 pr-3 font-mono text-slate-300">
                              {cand.diff_lines}
                            </td>
                            <td className="py-2 pr-3 font-mono text-slate-300">
                              ${cand.cost_usd.toFixed(4)}
                            </td>
                            <td className="py-2">
                              <button
                                type="button"
                                onClick={() =>
                                  onSelectWinner(
                                    activeArena.pipeline_id,
                                    step.step_id,
                                    cand.session_id ?? cand.profile,
                                  )
                                }
                                className="rounded border border-cyan-700 px-2 py-0.5 text-[11px] text-cyan-200 hover:bg-cyan-950 focus-visible:outline-2 focus-visible:outline-cyan-400"
                              >
                                {labels.selectWinner}
                              </button>
                            </td>
                          </tr>
                        );
                      })}
                    </tbody>
                  </table>
                </div>
              ))}
            </div>
          )}
        </div>
      ) : null}
    </section>
  );
}

export default PipelineArenaPanel;
