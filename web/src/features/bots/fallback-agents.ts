import type { AgentProfileStatus } from "../../transport/protocol";

interface FallbackSpec {
  id: string;
  displayName: string;
  command: string;
  args?: string[];
  provider?: string;
  apiKeyEnv?: string;
  loginCommand?: string[];
  category?: string;
  description: string;
}

const SPECS: readonly FallbackSpec[] = [
  {
    id: "antigravity",
    displayName: "Antigravity (agy)",
    command: "agy",
    provider: "google",
    apiKeyEnv: "GEMINI_API_KEY",
    loginCommand: ["agy"],
    description: "Google Antigravity coding agent CLI (OAuth default, API key optional)",
  },
  {
    id: "claude",
    displayName: "Claude Code",
    command: "claude",
    provider: "anthropic",
    apiKeyEnv: "ANTHROPIC_API_KEY",
    loginCommand: ["claude", "login"],
    description: "Anthropic Claude Code CLI (OAuth default, ANTHROPIC_API_KEY optional)",
  },
  {
    id: "codex",
    displayName: "OpenAI Codex",
    command: "codex",
    provider: "openai",
    apiKeyEnv: "OPENAI_API_KEY",
    loginCommand: ["codex", "login"],
    description: "OpenAI Codex CLI (ChatGPT OAuth default, OPENAI_API_KEY optional)",
  },
  {
    id: "gemini",
    displayName: "Gemini CLI",
    command: "gemini",
    provider: "google",
    apiKeyEnv: "GEMINI_API_KEY",
    loginCommand: ["gemini"],
    description: "Google Gemini CLI (Google OAuth default, GEMINI_API_KEY optional)",
  },
  {
    id: "opencode",
    displayName: "OpenCode",
    command: "opencode",
    provider: "openai",
    apiKeyEnv: "OPENAI_API_KEY",
    loginCommand: ["opencode", "auth", "login"],
    description: "OpenCode multi-provider coding CLI (OAuth or API key)",
  },
  {
    id: "aider",
    displayName: "Aider",
    command: "aider",
    provider: "openai",
    apiKeyEnv: "OPENAI_API_KEY",
    loginCommand: ["aider"],
    description: "Aider AI pair programmer in terminal",
  },
  {
    id: "goose",
    displayName: "Goose",
    command: "goose",
    provider: "openai",
    apiKeyEnv: "OPENAI_API_KEY",
    loginCommand: ["goose", "configure"],
    description: "Block Goose extensible developer agent with native MCP",
  },
  {
    id: "amp",
    displayName: "Amp",
    command: "amp",
    provider: "anthropic",
    apiKeyEnv: "AMP_API_KEY",
    loginCommand: ["amp", "login"],
    description: "Sourcegraph Amp coding agent CLI",
  },
  {
    id: "cline",
    displayName: "Cline CLI",
    command: "cline",
    provider: "anthropic",
    apiKeyEnv: "ANTHROPIC_API_KEY",
    loginCommand: ["cline", "auth"],
    description: "Cline autonomous coding agent CLI",
  },
  {
    id: "cursor",
    displayName: "Cursor Agent",
    command: "cursor-agent",
    provider: "openai",
    apiKeyEnv: "CURSOR_API_KEY",
    loginCommand: ["cursor-agent", "login"],
    description: "Cursor headless CLI agent",
  },
  {
    id: "copilot",
    displayName: "GitHub Copilot CLI",
    command: "gh",
    args: ["copilot"],
    apiKeyEnv: "GITHUB_TOKEN",
    loginCommand: ["gh", "auth", "login", "--web"],
    description: "GitHub Copilot CLI via GitHub OAuth",
  },
  {
    id: "kimi",
    displayName: "Kimi CLI",
    command: "kimi",
    provider: "openai",
    apiKeyEnv: "MOONSHOT_API_KEY",
    loginCommand: ["kimi", "login"],
    description: "Moonshot Kimi coding agent CLI",
  },
  {
    id: "qwen",
    displayName: "Qwen Code",
    command: "qwen",
    provider: "openai",
    apiKeyEnv: "DASHSCOPE_API_KEY",
    loginCommand: ["qwen"],
    description: "Qwen Code terminal coding agent",
  },
  {
    id: "shell",
    displayName: "Local Shell",
    command: "sh",
    category: "shell",
    description: "Interactive POSIX shell inside isolated worktree",
  },
];

export const FALLBACK_AGENTS: readonly AgentProfileStatus[] = SPECS.map((spec) => ({
  id: spec.id,
  installed: false,
  command_path: null,
  builtin: true,
  api_key_configured: false,
  api_key_source: "none",
  profile: {
    display_name: spec.displayName,
    command: spec.command,
    args: spec.args ?? [],
    env: {},
    pass_env: [],
    auth_mode: spec.category === "shell" ? "none" : "oauth",
    api_key_env: spec.apiKeyEnv ?? null,
    login_command: spec.loginCommand ?? [],
    proxy_mode: "bypass",
    proxy_provider: spec.provider ?? null,
    resume_args: [],
    extra_rw: [],
    extra_ro: [],
    category: spec.category ?? "coding_agent",
    description: spec.description,
  },
}));
