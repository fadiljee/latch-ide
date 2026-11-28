import { create } from "zustand";
import type {
  AgentProfileStatus,
  GitHubRepoItem,
  GitHubStatus,
  McpServerConfig,
  PluginConfig,
} from "../transport/protocol";

interface IntegrationsState {
  agents: AgentProfileStatus[];
  github: GitHubStatus | null;
  githubRepos: GitHubRepoItem[];
  mcpServers: McpServerConfig[];
  plugins: PluginConfig[];
  preferredProfile: string | null;
  setAgents: (agents: AgentProfileStatus[]) => void;
  upsertAgentLocal: (agent: AgentProfileStatus) => void;
  removeAgentLocal: (id: string) => void;
  setGitHub: (status: GitHubStatus | null) => void;
  setGitHubRepos: (repos: GitHubRepoItem[]) => void;
  setMcpServers: (servers: McpServerConfig[]) => void;
  upsertMcpLocal: (server: McpServerConfig) => void;
  removeMcpLocal: (id: string) => void;
  setPlugins: (plugins: PluginConfig[]) => void;
  upsertPluginLocal: (plugin: PluginConfig) => void;
  removePluginLocal: (id: string) => void;
  setPreferredProfile: (profile: string | null) => void;
}

export const useIntegrationsStore = create<IntegrationsState>()((set) => ({
  agents: [],
  github: null,
  githubRepos: [],
  mcpServers: [],
  plugins: [],
  preferredProfile: null,
  setAgents: (agents) => set({ agents }),
  upsertAgentLocal: (agent) =>
    set((s) => {
      const idx = s.agents.findIndex((a) => a.id === agent.id);
      if (idx >= 0) {
        const next = [...s.agents];
        next[idx] = agent;
        return { agents: next };
      }
      return { agents: [...s.agents, agent] };
    }),
  removeAgentLocal: (id) => set((s) => ({ agents: s.agents.filter((a) => a.id !== id) })),
  setGitHub: (github) => set({ github }),
  setGitHubRepos: (githubRepos) => set({ githubRepos }),
  setMcpServers: (mcpServers) => set({ mcpServers }),
  upsertMcpLocal: (server) =>
    set((s) => {
      const idx = s.mcpServers.findIndex((m) => m.id === server.id);
      if (idx >= 0) {
        const next = [...s.mcpServers];
        next[idx] = server;
        return { mcpServers: next };
      }
      return { mcpServers: [...s.mcpServers, server] };
    }),
  removeMcpLocal: (id) => set((s) => ({ mcpServers: s.mcpServers.filter((m) => m.id !== id) })),
  setPlugins: (plugins) => set({ plugins }),
  upsertPluginLocal: (plugin) =>
    set((s) => {
      const idx = s.plugins.findIndex((p) => p.id === plugin.id);
      if (idx >= 0) {
        const next = [...s.plugins];
        next[idx] = plugin;
        return { plugins: next };
      }
      return { plugins: [...s.plugins, plugin] };
    }),
  removePluginLocal: (id) => set((s) => ({ plugins: s.plugins.filter((p) => p.id !== id) })),
  setPreferredProfile: (preferredProfile) => set({ preferredProfile }),
}));
