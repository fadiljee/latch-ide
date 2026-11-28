import { fireEvent, render, screen } from "@testing-library/react";
import { beforeEach, describe, expect, it } from "vitest";
import { CATALOGS } from "../../lib/i18n/catalog";
import { useHostsStore } from "../../state/hosts";
import { useIntegrationsStore } from "../../state/integrations";
import { useUiStore } from "../../state/ui";
import { BotsPage } from "./BotsPage";

describe("BotsPage", () => {
  beforeEach(() => {
    useHostsStore.setState({ token: "test-token" });
    useUiStore.setState({ newSessionOpen: false });
    useIntegrationsStore.setState({
      agents: [
        {
          id: "antigravity",
          installed: true,
          command_path: "/usr/local/bin/agy",
          builtin: true,
          api_key_configured: false,
          api_key_source: "none",
          profile: {
            display_name: "Antigravity (agy)",
            command: "agy",
            args: [],
            env: {},
            pass_env: [],
            auth_mode: "oauth",
            api_key_env: "GEMINI_API_KEY",
            login_command: ["agy"],
            proxy_mode: "bypass",
            proxy_provider: "google",
            resume_args: [],
            extra_rw: [],
            extra_ro: [],
            category: "coding_agent",
            description: "Google Antigravity coding agent CLI",
          },
        },
        {
          id: "my-custom-cli",
          installed: false,
          command_path: null,
          builtin: false,
          api_key_configured: true,
          api_key_source: "runtime_memory",
          profile: {
            display_name: "My Custom CLI",
            command: "my-cli",
            args: ["--fast"],
            env: {},
            pass_env: [],
            auth_mode: "api_key",
            api_key_env: "MY_API_KEY",
            login_command: [],
            proxy_mode: "injected",
            proxy_provider: "openai",
            resume_args: [],
            extra_rw: [],
            extra_ro: [],
            category: "custom",
            description: "Custom test agent",
          },
        },
      ],
      preferredProfile: null,
    });
  });

  it("filters agents by search and category, opens custom agent dialog, and launches preferred profile", () => {
    const t = CATALOGS.en;
    render(
      <BotsPage
        labels={t.pages}
        agentLabels={t.agents}
        newSessionLabel={t.workspace.newSessionCta}
      />,
    );

    expect(screen.getByText("Antigravity (agy)")).toBeInTheDocument();
    expect(screen.getByText("My Custom CLI")).toBeInTheDocument();
    expect(screen.getByText("Ready (Session RAM)")).toBeInTheDocument();

    // Filter custom only
    fireEvent.click(screen.getByRole("button", { name: "Custom" }));
    expect(screen.queryByText("Antigravity (agy)")).not.toBeInTheDocument();
    expect(screen.getByText("My Custom CLI")).toBeInTheDocument();

    // Filter installed only
    fireEvent.click(screen.getByRole("button", { name: "Installed" }));
    expect(screen.getByText("Antigravity (agy)")).toBeInTheDocument();
    expect(screen.queryByText("My Custom CLI")).not.toBeInTheDocument();

    // Launch sets preferredProfile and opens NewSessionDialog
    fireEvent.click(screen.getByRole("button", { name: "Launch" }));
    expect(useIntegrationsStore.getState().preferredProfile).toBe("antigravity");
    expect(useUiStore.getState().newSessionOpen).toBe(true);

    // Search empty state
    fireEvent.change(screen.getByPlaceholderText(t.agents.searchPlaceholder), {
      target: { value: "nonexistent-xyz" },
    });
    expect(screen.getByText(t.agents.emptySearch)).toBeInTheDocument();

    // Open Add Agent dialog
    fireEvent.click(screen.getByRole("button", { name: t.agents.addCustom }));
    expect(screen.getByRole("dialog", { name: t.agents.customDialogTitle })).toBeInTheDocument();
  });
});
