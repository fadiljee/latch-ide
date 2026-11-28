import { fireEvent, render, screen, waitFor } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";
import { CATALOGS } from "../../lib/i18n/catalog";
import type { Session } from "../../transport/protocol";
import { TerminalGrid } from "./TerminalGrid";

vi.mock("./TerminalView", () => ({
  default: ({
    sessionId,
    focused,
    inputPlaceholder,
    onFocus,
  }: {
    sessionId: string;
    focused?: boolean;
    inputPlaceholder: string;
    onFocus?: () => void;
  }) => (
    <div data-testid={`mock-term-${sessionId}`}>
      <input
        type="text"
        aria-label={`${inputPlaceholder}-${sessionId}`}
        disabled={!focused}
        onFocus={onFocus}
      />
    </div>
  ),
}));

function makeSession(index: number): Session {
  return {
    id: `ses-${index}`,
    project_id: "proj-1",
    agent_profile: index % 2 === 0 ? "claude" : "codex",
    branch_name: `latch/task-${index}`,
    worktree_path: `/tmp/wt-${index}`,
    task: `Task ${index}`,
    status: "running",
    pid: 1000 + index,
    started_at: 1700000000000 + index,
    ended_at: null,
    pipeline_run_id: null,
    step_id: null,
    exit_code: null,
    trip_reason: null,
    proxy_mode: "injected",
    sandbox_provider: "landlock",
  };
}

describe("TerminalGrid 2x2 (IT-110)", () => {
  it("renders at most 4 cells, enforces focus-only input, switches focus, and expands to single pane", async () => {
    const t = CATALOGS.en;
    const sessions = [1, 2, 3, 4, 5].map(makeSession);
    const onFocusSession = vi.fn();
    const onExpandSession = vi.fn();

    render(
      <TerminalGrid
        token="tok"
        sessions={sessions}
        focusedSessionId="ses-2"
        resyncLabel={t.workspace.resync}
        inputPlaceholder="pty-input"
        focusedLabel={t.workspace.focusedCell}
        clickToFocusLabel={t.workspace.clickToFocus}
        expandLabel={t.workspace.expandPane}
        loadingLabel={t.workspace.loadingDiff}
        onFocusSession={onFocusSession}
        onExpandSession={onExpandSession}
      />,
    );

    // Maksimal 4 sel ditampilkan dalam grid 2x2
    await waitFor(() => {
      expect(screen.getByTestId("mock-term-ses-1")).toBeInTheDocument();
      expect(screen.getByTestId("mock-term-ses-4")).toBeInTheDocument();
    });
    expect(screen.queryByTestId("mock-term-ses-5")).not.toBeInTheDocument();

    // Hanya sel fokus (ses-2) yang mengaktifkan input
    expect(screen.getByLabelText("pty-input-ses-1")).toBeDisabled();
    expect(screen.getByLabelText("pty-input-ses-2")).toBeEnabled();
    expect(screen.getByLabelText("pty-input-ses-3")).toBeDisabled();
    expect(screen.getByLabelText("pty-input-ses-4")).toBeDisabled();

    // Klik header sel ses-3 memindahkan fokus ke ses-3
    fireEvent.click(screen.getByRole("button", { name: /Task 3/i }));
    expect(onFocusSession).toHaveBeenCalledWith("ses-3");

    // Klik tombol expand pada sel pertama (ses-1) memanggil onExpandSession("ses-1")
    const expandButtons = screen.getAllByRole("button", { name: t.workspace.expandPane });
    expect(expandButtons).toHaveLength(4);
    const firstExpand = expandButtons[0];
    expect(firstExpand).toBeDefined();
    if (firstExpand) {
      fireEvent.click(firstExpand);
    }
    expect(onExpandSession).toHaveBeenCalledWith("ses-1");
  });
});
