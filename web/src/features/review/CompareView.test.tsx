import { fireEvent, render, screen } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";
import { CATALOGS } from "../../lib/i18n/catalog";
import { CompareView } from "./CompareView";

describe("CompareView (Fan-Out Arena)", () => {
  it("renders candidate comparison matrix and selects a winner", () => {
    const onSelect = vi.fn();
    const onClose = vi.fn();

    render(
      <CompareView
        arena={{
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
        closeLabel={CATALOGS.id.sidePanel.close}
        onSelectWinner={onSelect}
        onClose={onClose}
      />,
    );

    expect(screen.getByText("420 ms")).toBeInTheDocument();
    expect(screen.getByText("110 ms")).toBeInTheDocument();

    const selectButtons = screen.getAllByRole("button", { name: "Pilih Pemenang" });
    const second = selectButtons[1];
    expect(second).toBeDefined();
    if (second) {
      fireEvent.click(second);
    }
    expect(onSelect).toHaveBeenCalledWith(
      "01950000-0000-7000-8000-000000000001",
      "implement",
      "ses-fast",
    );
  });
});
