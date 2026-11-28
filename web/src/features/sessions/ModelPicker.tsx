import { useIntegrationsStore } from "../../state/integrations";
import type { AgentProfileStatus } from "../../transport/protocol";

export interface AgentProfileOption {
  id: string;
  displayName: string;
  provider: string;
  authMode: "oauth" | "api_key" | "none";
  installed?: boolean;
}

export const BUILTIN_PROFILES: readonly AgentProfileOption[] = [
  { id: "antigravity", displayName: "Antigravity (agy)", provider: "google", authMode: "oauth" },
  { id: "claude", displayName: "Claude Code", provider: "anthropic", authMode: "oauth" },
  { id: "codex", displayName: "OpenAI Codex", provider: "openai", authMode: "oauth" },
  { id: "gemini", displayName: "Gemini CLI", provider: "google", authMode: "oauth" },
  { id: "opencode", displayName: "OpenCode", provider: "openai", authMode: "oauth" },
  { id: "aider", displayName: "Aider", provider: "openai", authMode: "oauth" },
  { id: "goose", displayName: "Goose", provider: "openai", authMode: "oauth" },
  { id: "amp", displayName: "Amp", provider: "anthropic", authMode: "oauth" },
  { id: "cline", displayName: "Cline CLI", provider: "anthropic", authMode: "oauth" },
  { id: "cursor", displayName: "Cursor Agent", provider: "openai", authMode: "oauth" },
  { id: "copilot", displayName: "GitHub Copilot CLI", provider: "github", authMode: "oauth" },
  { id: "kimi", displayName: "Kimi CLI", provider: "openai", authMode: "oauth" },
  { id: "qwen", displayName: "Qwen Code", provider: "openai", authMode: "oauth" },
  { id: "shell", displayName: "Local Shell", provider: "local", authMode: "none" },
  { id: "sleeper", displayName: "Sleeper (Test)", provider: "local", authMode: "none" },
];

export function toProfileOptions(liveAgents: readonly AgentProfileStatus[]): AgentProfileOption[] {
  if (liveAgents.length === 0) {
    return [...BUILTIN_PROFILES];
  }
  return liveAgents.map((a) => ({
    id: a.id,
    displayName: a.profile.display_name || a.id,
    provider: a.profile.proxy_provider || (a.profile.category === "shell" ? "local" : "custom"),
    authMode: a.profile.auth_mode,
    installed: a.installed,
  }));
}

interface ModelPickerProps {
  id: string;
  label: string;
  value: string;
  onChange: (profile: string) => void;
}

export function ModelPicker({ id, label, value, onChange }: ModelPickerProps) {
  const liveAgents = useIntegrationsStore((s) => s.agents);
  const options = toProfileOptions(liveAgents);

  return (
    <label htmlFor={id} className="block text-xs">
      <span className="mb-1 block text-fg-2">{label}</span>
      <select
        id={id}
        value={value}
        onChange={(e) => onChange(e.target.value)}
        className="w-full rounded-md border border-line bg-app px-2.5 py-1.5 text-xs text-fg focus:border-accent focus:outline-none"
      >
        {options.map((p) => {
          const authBadge =
            p.authMode === "oauth" ? "OAuth" : p.authMode === "api_key" ? "API Key" : "Local";
          const installedMark = p.installed ? " [Installed]" : "";
          return (
            <option key={p.id} value={p.id}>
              {p.displayName} ({p.id} · {authBadge}){installedMark}
            </option>
          );
        })}
      </select>
    </label>
  );
}
