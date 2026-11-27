import { Button } from "../../components/ui/Button";
import type { Catalog } from "../../lib/i18n/catalog";
import type { PipelineArenaResponse } from "../../transport/protocol";

interface CompareViewProps {
  arena: PipelineArenaResponse;
  labels: Catalog["pipelines"];
  closeLabel: string;
  onSelectWinner: (pipelineId: string, stepId: string, candidateId: string) => void;
  onClose: () => void;
}

export function CompareView({
  arena,
  labels,
  closeLabel,
  onSelectWinner,
  onClose,
}: CompareViewProps) {
  return (
    <div
      role="region"
      aria-label={labels.arenaHeading}
      className="mt-4 rounded-lg border border-accent/40 bg-surface p-4"
    >
      <div className="mb-3 flex flex-wrap items-center justify-between gap-2">
        <div>
          <h3 className="text-sm font-semibold text-fg">
            {labels.arenaHeading} : {arena.workflow_name}
          </h3>
          <p className="font-mono text-[11px] text-fg-3">
            {arena.pipeline_id.slice(0, 8)} · {arena.status}
          </p>
        </div>
        <Button size="sm" variant="ghost" onClick={onClose}>
          {closeLabel}
        </Button>
      </div>

      {arena.steps.length === 0 ? (
        <p className="text-xs text-fg-3">{labels.noArenaSteps}</p>
      ) : (
        <div className="space-y-4">
          {arena.steps.map((step) => (
            <div key={step.step_id} className="overflow-x-auto">
              <div className="mb-2 flex items-center justify-between text-xs">
                <span className="font-mono font-semibold text-fg">{step.step_id}</span>
                {step.winner ? (
                  <span className="rounded-sm bg-done/15 px-2 py-0.5 text-[11px] text-done">
                    {labels.winner}: {step.winner}
                  </span>
                ) : null}
              </div>
              <table className="w-full min-w-[480px] border-collapse text-left text-xs">
                <thead>
                  <tr className="border-b border-line text-fg-3">
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
                        className="border-b border-line/70"
                      >
                        <td className="py-2 pr-3 font-mono text-fg">
                          {cand.profile}
                          {isWinner ? (
                            <span className="ml-2 rounded-sm bg-done/15 px-1.5 py-0.5 text-[10px] text-done">
                              {labels.winner}
                            </span>
                          ) : null}
                        </td>
                        <td className="py-2 pr-3">
                          <span className={cand.passed ? "text-done" : "text-danger"}>
                            {cand.passed ? labels.passed : labels.failed}
                          </span>
                        </td>
                        <td className="py-2 pr-3 font-mono text-fg-2">{cand.duration_ms} ms</td>
                        <td className="py-2 pr-3 font-mono text-fg-2">{cand.diff_lines}</td>
                        <td className="py-2 pr-3 font-mono text-fg-2">
                          ${cand.cost_usd.toFixed(4)}
                        </td>
                        <td className="py-2">
                          <Button
                            size="sm"
                            onClick={() =>
                              onSelectWinner(
                                arena.pipeline_id,
                                step.step_id,
                                cand.session_id ?? cand.profile,
                              )
                            }
                          >
                            {labels.selectWinner}
                          </Button>
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
  );
}

export default CompareView;
