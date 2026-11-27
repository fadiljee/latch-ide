import { fireEvent, render, screen } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";
import { CATALOGS } from "../../lib/i18n/catalog";
import { PipelineArenaPanel } from "./PipelineArenaPanel";

describe("PipelineArenaPanel", () => {
  it("renders pipeline runs and fan-out arena comparison matrix with winner selection", () => {
    const onInspect = vi.fn();
    const onCancel = vi.fn();
    const onSelect = vi.fn();
    const onClose = vi.fn();

    render(
      <PipelineArenaPanel
        pipelines={[
          {
            id: "01950000-0000-7000-8000-000000000001",
            project_id: "proj-1",
            workflow_name: "feature-arena",
            dag_state_json: "{}",
            status: "succeeded",
            created_at: 1700000000000,
            updated_at: 1700000005000,
            task: "Implement endpoint",
            total_cost_usd: 0.35,
          },
        ]}
        activeArena={{
          pipeline_id: "01950000-0000-7000-8000-000000000001",
          workflow_name: "feature-arena",
          status: "succeeded",
          steps: [
            {
              step_id: "implement",
              status: "succeeded",
              winner: "cheap",
              candidates: [
                {
                  profile: "cheap",
                  session_id: "ses-cheap",
                  passed: true,
                  duration_ms: 420,
                  diff_lines: 18,
                  cost_usd: 0.1,
                  output: "ok",
                },
                {
                  profile: "fast",
                  session_id: "ses-fast",
                  passed: true,
                  duration_ms: 110,
                  diff_lines: 24,
                  cost_usd: 0.25,
                  output: "ok",
                },
              ],
            },
          ],
        }}
        labels={CATALOGS.id.pipelines}
        closeLabel={CATALOGS.id.closeLabel}
        onInspectArena={onInspect}
        onCancelPipeline={onCancel}
        onSelectWinner={onSelect}
        onCloseArena={onClose}
      />,
    );

    expect(screen.getByRole("heading", { name: "Pipeline DAG & Arena" })).toBeInTheDocument();
    expect(screen.getByText("feature-arena")).toBeInTheDocument();
    expect(screen.getByText("420 ms")).toBeInTheDocument();
    expect(screen.getByText("110 ms")).toBeInTheDocument();

    const selectButtons = screen.getAllByRole("button", { name: "Pilih Pemenang" });
    const secondButton = selectButtons[1];
    expect(secondButton).toBeDefined();
    if (secondButton) {
      fireEvent.click(secondButton);
    }
    expect(onSelect).toHaveBeenCalledWith(
      "01950000-0000-7000-8000-000000000001",
      "implement",
      "ses-fast",
    );
  });
});
