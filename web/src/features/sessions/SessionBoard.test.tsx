import { act, fireEvent, render, within } from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";
import { CATALOGS } from "../../lib/i18n/catalog";
import { useHostsStore } from "../../state/hosts";
import { useSessionsStore } from "../../state/sessions";
import { useUiStore } from "../../state/ui";
import type { Session } from "../../transport/protocol";
import { SessionBoard } from "./SessionBoard";

function makeSession(id: string, status: Session["status"], task: string): Session {
  return {
    id,
    project_id: "proj-1",
    agent_profile: "antigravity",
    worktree_path: `/tmp/wt-${id}`,
    branch_name: `latch/demo/${id}`,
    status,
    pid: 1001,
    started_at: 1700000000000,
    ended_at: null,
    task,
    pipeline_run_id: null,
    step_id: null,
    exit_code: null,
    trip_reason: null,
    proxy_mode: "bypass",
    sandbox_provider: "landlock",
  };
}

describe("Mission Control SessionBoard (IT-107)", () => {
  beforeEach(() => {
    useHostsStore.setState({
      token: "tok",
      projects: [
        {
          id: "proj-1",
          name: "latch-ide",
          repo_path: "/tmp/latch-ide",
          default_branch: "main",
          created_at: 1700000000000,
        },
      ],
      selectedProjectId: "all",
    });
    useUiStore.setState({ searchQuery: "" });
    useSessionsStore.getState().upsertMany([
      makeSession("s-wait", "waiting_approval", "Approve git push"),
      makeSession("s-run", "running", "Implement fs-service"),
      makeSession("s-err", "failed", "Broken migration"),
      makeSession("s-done", "completed", "Fix lint"),
      makeSession("s-hib", "hibernated", "Paused refactor"),
    ]);
  });

  it("groups cards into all 5 status columns, reflects live updates immediately, and opens terminal/diff", () => {
    const t = CATALOGS.en;
    const onOpenSession = vi.fn();
    const onNewSession = vi.fn();

    const { container } = render(
      <SessionBoard
        token="tok"
        statusLabels={t.statusLabels}
        actions={t.sessionActions}
        workspaceLabels={t.workspace}
        onOpenSession={onOpenSession}
        onNewSession={onNewSession}
      />,
    );

    const needsCol = container.querySelector('[data-board-column="needs_you"]') as HTMLElement;
    const workingCol = container.querySelector('[data-board-column="working"]') as HTMLElement;
    const errorCol = container.querySelector('[data-board-column="error"]') as HTMLElement;
    const doneCol = container.querySelector('[data-board-column="done"]') as HTMLElement;
    const idleCol = container.querySelector('[data-board-column="idle"]') as HTMLElement;

    expect(within(needsCol).getByText("Approve git push")).toBeInTheDocument();
    expect(within(workingCol).getByText("Implement fs-service")).toBeInTheDocument();
    expect(within(errorCol).getByText("Broken migration")).toBeInTheDocument();
    expect(within(doneCol).getByText("Fix lint")).toBeInTheDocument();
    expect(within(idleCol).getByText("Paused refactor")).toBeInTheDocument();

    // Update live di store memindahkan kartu dari working -> done secara seketika (< 500 ms)
    act(() => {
      useSessionsStore
        .getState()
        .upsertOne(makeSession("s-run", "completed", "Implement fs-service"));
    });
    expect(within(doneCol).getByText("Implement fs-service")).toBeInTheDocument();
    expect(within(workingCol).queryByText("Implement fs-service")).not.toBeInTheDocument();

    // Klik judul kartu membuka sesi di tab terminal
    fireEvent.click(within(needsCol).getByText("Approve git push"));
    expect(onOpenSession).toHaveBeenCalledWith("s-wait", "terminal");

    // Klik tombol Diff membuka sesi di tab diff
    fireEvent.click(within(needsCol).getByRole("button", { name: t.workspace.diffTab }));
    expect(onOpenSession).toHaveBeenCalledWith("s-wait", "diff");
  });
});
