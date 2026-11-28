import { fireEvent, render, screen } from "@testing-library/react";
import { beforeEach, describe, expect, it } from "vitest";
import { CATALOGS } from "../../lib/i18n/catalog";
import { useHostsStore } from "../../state/hosts";
import { useIntegrationsStore } from "../../state/integrations";
import { PluginsPage } from "./PluginsPage";

describe("PluginsPage", () => {
  beforeEach(() => {
    useHostsStore.setState({ token: "test-token" });
    useIntegrationsStore.setState({
      github: {
        gh_installed: true,
        gh_path: "/usr/bin/gh",
        authenticated: true,
        host: "github.com",
        account: "fadiljee",
        protocol: "https",
        scopes: ["repo", "workflow"],
      },
      githubRepos: [
        {
          name_with_owner: "fadiljee/latch-ide",
          url: "https://github.com/fadiljee/latch-ide",
          default_branch: "main",
          is_private: false,
          description: "Rust agent orchestrator",
        },
      ],
      mcpServers: [
        {
          id: "github",
          name: "GitHub MCP",
          transport: "stdio",
          command: "npx",
          args: ["-y", "@modelcontextprotocol/server-github"],
          url: null,
          env_keys: ["GITHUB_PERSONAL_ACCESS_TOKEN"],
          enabled: true,
          description: "GitHub tools",
        },
      ],
      plugins: [
        {
          id: "rust-ci-workflows",
          name: "Rust CI Workflow Pack",
          kind: "workflow_pack",
          source: "github:latch-ide/workflows-rust",
          command: null,
          enabled: true,
          description: "Reusable Rust CI workflows",
        },
      ],
    });
  });

  it("renders GitHub status/repos and switches cleanly across MCP Servers and Plugins tabs", () => {
    const t = CATALOGS.en;
    render(<PluginsPage labels={t.integrations} />);

    // Default tab: GitHub
    expect(screen.getByText(t.integrations.ghConnected)).toBeInTheDocument();
    expect(screen.getByText(/fadiljee\/latch-ide/)).toBeInTheDocument();
    expect(screen.getByText("workflow")).toBeInTheDocument();

    // Switch to MCP Servers tab
    fireEvent.click(screen.getByRole("tab", { name: /MCP Servers/ }));
    expect(screen.getByText(t.integrations.mcpPresetsHeading)).toBeInTheDocument();
    expect(screen.getAllByText("GitHub MCP").length).toBeGreaterThanOrEqual(2);
    expect(screen.getAllByText("GITHUB_PERSONAL_ACCESS_TOKEN").length).toBeGreaterThanOrEqual(2);

    // Toggle custom MCP transport to http reveals URL input
    fireEvent.click(screen.getByRole("button", { name: "http" }));
    expect(screen.getByLabelText(t.integrations.mcpUrlLabel)).toBeInTheDocument();

    // Switch to Plugins tab
    fireEvent.click(screen.getByRole("tab", { name: /Plugins/ }));
    expect(screen.getByText(t.integrations.pluginsPresetsHeading)).toBeInTheDocument();
    expect(screen.getAllByText("Rust CI Workflow Pack").length).toBeGreaterThanOrEqual(2);
    expect(screen.getByLabelText(t.integrations.pluginSourceLabel)).toBeInTheDocument();
  });
});
