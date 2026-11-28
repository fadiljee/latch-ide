import { fireEvent, render, screen, waitFor } from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";
import { CATALOGS } from "../../lib/i18n/catalog";
import { useHostsStore } from "../../state/hosts";
import { useIntegrationsStore } from "../../state/integrations";
import { useUiStore } from "../../state/ui";
import { buildAntigravityArgs, NewSessionDialog } from "./NewSessionDialog";
import { templatesForProfile } from "./pty-message";

const createSessionMock = vi.fn();
const fetchAntigravityCliInfoMock = vi.fn();

vi.mock("../../transport/daemon-client", async () => {
  const actual = await vi.importActual<typeof import("../../transport/daemon-client")>(
    "../../transport/daemon-client",
  );
  return {
    ...actual,
    createSession: (...args: unknown[]) => createSessionMock(...args),
    fetchAntigravityCliInfo: (...args: unknown[]) => fetchAntigravityCliInfoMock(...args),
  };
});

describe("Antigravity CLI (agy) IDE integration", () => {
  beforeEach(() => {
    createSessionMock.mockReset();
    fetchAntigravityCliInfoMock.mockReset();
    fetchAntigravityCliInfoMock.mockResolvedValue({
      installed: true,
      command_path: "/home/user/.local/bin/agy",
      version: "1.5.19",
      acp_supported: true,
      modes: ["auto", "accept-edits", "plan"],
      efforts: ["low", "medium", "high", "xhigh", "max"],
      models: [
        { id: "gemini-4-neon-high", label: "Gemini 4 Neon (High)" },
        { id: "claude-opus-5-5-high", label: "Claude Opus 5.5 (High)" },
      ],
    });
    createSessionMock.mockResolvedValue({
      id: "ses-agy-1",
      project_id: "proj-1",
      agent_profile: "antigravity",
      worktree_path: "/tmp/wt-agy",
      branch_name: "latch/demo/agy1",
      status: "running",
      pid: 4242,
      started_at: 1700000000000,
      ended_at: null,
      task: "",
      pipeline_run_id: null,
      step_id: null,
      exit_code: null,
      trip_reason: null,
      proxy_mode: "bypass",
      sandbox_provider: "landlock",
    });

    useHostsStore.setState({
      token: "test-token",
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
    useIntegrationsStore.setState({
      agents: [],
      preferredProfile: "antigravity",
    });
    useUiStore.setState({
      newSessionOpen: true,
      locale: "en",
    });
  });

  it("builds CLI flags and exposes Antigravity slash templates", () => {
    expect(
      buildAntigravityArgs({
        mode: "plan",
        effort: "high",
        model: "gemini-4-neon-high",
        continueLast: true,
      }),
    ).toEqual([
      "--mode",
      "plan",
      "--effort",
      "high",
      "--model",
      "gemini-4-neon-high",
      "--continue",
    ]);

    const templates = templatesForProfile("antigravity");
    expect(templates.some((tpl) => tpl.label === "/plan")).toBe(true);
    expect(templates.some((tpl) => tpl.label === "/compact")).toBe(true);
    expect(templates.some((tpl) => tpl.label === "/goal")).toBe(true);
  });

  it("renders Antigravity CLI options, loads live models, and launches interactive session with empty task", async () => {
    const t = CATALOGS.en;
    render(
      <NewSessionDialog
        labels={t.newSession}
        repoPlaceholder={t.sidebar.addRepoPlaceholder}
        addRepoLabel={t.sidebar.addRepoButton}
      />,
    );

    await waitFor(() => {
      expect(screen.getByText("v1.5.19")).toBeInTheDocument();
    });

    fireEvent.change(screen.getByLabelText(t.newSession.agyModeLabel), {
      target: { value: "plan" },
    });
    fireEvent.change(screen.getByLabelText(t.newSession.agyEffortLabel), {
      target: { value: "high" },
    });
    fireEvent.change(screen.getByLabelText(t.newSession.agyModelLabel), {
      target: { value: "gemini-4-neon-high" },
    });
    fireEvent.click(screen.getByLabelText(t.newSession.agyContinueLabel));

    // Tombol launch aktif meski deskripsi tugas kosong (mode REPL interaktif agy)
    const launchBtn = screen.getByRole("button", { name: t.newSession.launch });
    expect(launchBtn).toBeEnabled();
    fireEvent.click(launchBtn);

    await waitFor(() => {
      expect(createSessionMock).toHaveBeenCalledWith("test-token", {
        project: "latch-ide",
        profile: "antigravity",
        task: "",
        base_ref: undefined,
        extra_args: [
          "--mode",
          "plan",
          "--effort",
          "high",
          "--model",
          "gemini-4-neon-high",
          "--continue",
        ],
      });
    });
  });
});
