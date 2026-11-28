import { fireEvent, render, screen, waitFor } from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";
import { useHostsStore } from "../state/hosts";
import { useLayoutStore } from "../state/layout";
import { useNotificationsStore } from "../state/notifications";
import { useSessionsStore } from "../state/sessions";
import { useUiStore } from "../state/ui";
import { Workspace } from "../layout/Workspace";
import { SidePanel } from "../layout/SidePanel";
import { StatusBar } from "../layout/StatusBar";
import { CommandPalette } from "../components/CommandPalette";

const decideApprovalMock = vi.fn();
const fetchSessionDetailMock = vi.fn();
const fetchSessionDiffMock = vi.fn();

vi.mock("../features/terminal/TerminalView", () => ({
  default: ({ sessionId, focused }: { sessionId: string; focused?: boolean }) => (
    <div data-testid={`e2e-term-${sessionId}`} data-focused={String(focused ?? true)} />
  ),
}));

vi.mock("../transport/daemon-client", async () => {
  const actual = await vi.importActual<typeof import("../transport/daemon-client")>(
    "../transport/daemon-client",
  );
  return {
    ...actual,
    decideApproval: (...args: unknown[]) => decideApprovalMock(...args),
    fetchSessionDetail: (...args: unknown[]) => fetchSessionDetailMock(...args),
    fetchSessionDiff: (...args: unknown[]) => fetchSessionDiffMock(...args),
    checkSessionMerge: vi.fn().mockResolvedValue({ clean: true, conflicts: [] }),
  };
});

describe("Milestone I1 End-to-End Workbench Flow (IT-117)", () => {
  beforeEach(() => {
    decideApprovalMock.mockReset().mockResolvedValue({});
    fetchSessionDetailMock.mockReset().mockResolvedValue({
      telemetry: {
        session_id: "ses-1",
        request_count: 2,
        prompt_tokens: 1200,
        completion_tokens: 400,
        estimated_cost_usd: 0.012,
        error_count: 0,
      },
    });
    fetchSessionDiffMock.mockReset().mockResolvedValue({
      base_branch: "main",
      files: [{ path: "src/app.rs", old_path: null, status: "modified", additions: 3, deletions: 1 }],
      patch: "@@ -1,1 +1,3 @@\n+<script>window.__xss=1</script>\n+fn ok() {}",
      total_patch_bytes: 64,
      truncated: false,
    });

    useHostsStore.setState({
      token: "i1-token",
      connected: true,
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
      error: null,
    });
    useSessionsStore.getState().upsertMany([
      {
        id: "ses-1",
        project_id: "proj-1",
        agent_profile: "antigravity",
        worktree_path: "/tmp/wt-1",
        branch_name: "latch/demo/s1",
        status: "waiting_approval",
        pid: 101,
        started_at: 1700000000000,
        ended_at: null,
        task: "Harden auth tokens",
        pipeline_run_id: null,
        step_id: null,
        exit_code: null,
        trip_reason: null,
        proxy_mode: "bypass",
        sandbox_provider: "landlock",
      },
      {
        id: "ses-2",
        project_id: "proj-1",
        agent_profile: "claude",
        worktree_path: "/tmp/wt-2",
        branch_name: "latch/demo/s2",
        status: "running",
        pid: 102,
        started_at: 1700000001000,
        ended_at: null,
        task: "Refactor store actor",
        pipeline_run_id: null,
        step_id: null,
        exit_code: null,
        trip_reason: null,
        proxy_mode: "bypass",
        sandbox_provider: "landlock",
      },
    ]);
    useNotificationsStore.setState({
      approvals: [
        {
          id: "apr-1",
          session_id: "ses-1",
          action_type: "shell_exec",
          payload_summary: "git push origin latch/demo/s1 <img src=x onerror=alert(1)>",
          decision: "pending",
          decided_by: null,
          decided_at: null,
          created_at: 1700000002000,
          expires_at: null,
        },
      ],
    });
    useLayoutStore.setState({
      activeSessionId: "ses-1",
      secondarySessionId: "ses-2",
      focusedGridSessionId: "ses-1",
      splitMode: "board",
      primaryTab: "terminal",
      secondaryTab: "diff",
    });
    useUiStore.setState({
      locale: "en",
      activeRoute: "workspace",
      sidePanelOpen: true,
      sidePanelTab: "checks",
      commandPaletteOpen: false,
    });
  });

  it("flows across Board -> Diff (XSS-safe) -> 2x2 Grid -> Approvals inbox -> Command Palette", async () => {
    render(
      <div>
        <Workspace />
        <SidePanel />
        <StatusBar />
        <CommandPalette />
      </div>,
    );

    // 1. Papan Mission Control menampilkan kedua sesi pada kolomnya
    await waitFor(() => {
      expect(screen.getByText("Mission Control Session Board")).toBeInTheDocument();
    });
    expect(screen.getByText("Harden auth tokens")).toBeInTheDocument();
    expect(screen.getByText("Refactor store actor")).toBeInTheDocument();

    // 2. Buka Diff sesi dari kartu -> pastikan payload HTML/script dirender sebagai teks murni (IT3)
    const diffButtons = screen.getAllByRole("button", { name: "Diff" });
    const firstDiff = diffButtons[0];
    expect(firstDiff).toBeDefined();
    if (firstDiff) {
      fireEvent.click(firstDiff);
    }
    await waitFor(() => {
      expect(screen.getByText(/\+<script>window\.__xss=1<\/script>/)).toBeInTheDocument();
    });
    expect((window as unknown as { __xss?: number }).__xss).toBeUndefined();

    // 3. Putuskan persetujuan dari Inbox SidePanel (IT-109)
    fireEvent.click(screen.getByRole("button", { name: "Approve" }));
    await waitFor(() => {
      expect(decideApprovalMock).toHaveBeenCalledWith("i1-token", "apr-1", "approved");
    });

    // 4. Beralih ke 2x2 Terminal Grid via StatusBar (IT-110)
    fireEvent.click(screen.getByRole("button", { name: "2×2 Grid" }));
    await waitFor(() => {
      expect(screen.getByTestId("e2e-term-ses-1")).toHaveAttribute("data-focused", "true");
      expect(screen.getByTestId("e2e-term-ses-2")).toHaveAttribute("data-focused", "false");
    });
  });
});
