# Agent CLI Compatibility Matrix (`T-902`, `IT-904`)

Latch orchestrates any terminal-based coding agent via `portable-pty` inside an isolated `git worktree` with Landlock sandboxing (Linux >= 5.13) and optional local LLM proxy telemetry.

## Built-in Agent Profiles

All coding agents default to **native OAuth (`proxy_mode = "bypass"`)** for zero-friction setup, and can switch to **API Key (`proxy_mode = "injected"`)** per profile in the Agents view (`BotsPage`) or `~/.config/latch/agents.toml`.

| Profile ID | Agent CLI | Binary | Default Auth | OAuth Login Command | Proxy Provider (API Key mode) | Env Var (`api_key_env`) | Sandbox `extra_rw` | Status |
| --- | --- | --- | --- | --- | --- | --- | --- | --- |
| `antigravity` | Antigravity | `agy` (`--prompt-interactive`, `--continue`, `--mode`, `--effort`, `--model`) | OAuth | `agy` | `google` | `GEMINI_API_KEY` | `~/.gemini`, `~/.config/antigravity` | Supported (First-class) |
| `claude` | Claude Code | `claude` | OAuth | `claude login` | `anthropic` | `ANTHROPIC_API_KEY` | `~/.claude` | Supported |
| `codex` | OpenAI Codex | `codex` | OAuth | `codex login` | `openai` | `OPENAI_API_KEY` | `~/.codex` | Supported |
| `gemini` | Gemini CLI | `gemini` | OAuth | `gemini` | `google` | `GEMINI_API_KEY` | `~/.gemini` | Supported |
| `opencode` | OpenCode | `opencode` | OAuth | `opencode auth login` | `openai` | `OPENAI_API_KEY` | `~/.config/opencode`, `~/.local/share/opencode` | Supported |
| `aider` | Aider | `aider` | OAuth / Env | `aider` | `openai` | `OPENAI_API_KEY` | — | Supported |
| `goose` | Block Goose | `goose` | OAuth | `goose configure` | `openai` | `OPENAI_API_KEY` | `~/.config/goose` | Supported |
| `amp` | Sourcegraph Amp | `amp` | OAuth | `amp login` | `anthropic` | `AMP_API_KEY` | `~/.config/amp` | Supported |
| `cline` | Cline CLI | `cline` | OAuth | `cline auth` | `anthropic` | `ANTHROPIC_API_KEY` | `~/.cline` | Supported |
| `cursor` | Cursor Agent | `cursor-agent` | OAuth | `cursor-agent login` | `openai` | `CURSOR_API_KEY` | `~/.cursor` | Supported |
| `copilot` | GitHub Copilot CLI | `gh copilot` | GitHub OAuth | `gh auth login --web` | Bypass only | `GITHUB_TOKEN` | `~/.config/gh` | Supported |
| `kimi` | Kimi CLI | `kimi` | OAuth | `kimi login` | `openai` | `MOONSHOT_API_KEY` | `~/.kimi` | Supported |
| `qwen` | Qwen Code | `qwen` | OAuth | `qwen` | `openai` | `DASHSCOPE_API_KEY` | `~/.qwen` | Supported |
| `shell` | Local POSIX Shell | `sh` | None | — | Bypass | — | — | Built-in utility |

## Feature Matrix by Mode

| Capability | `proxy_mode = "bypass"` (OAuth default) | `proxy_mode = "injected"` (API Key) |
| --- | --- | --- |
| Isolated `git worktree` & branch | Yes | Yes |
| Interactive PTY + `@xterm/xterm` + TUI attach | Yes | Yes |
| Landlock filesystem sandbox (Linux >= 5.13) | Yes | Yes |
| `git push` PATH shim approval gate | Yes | Yes |
| Exact LLM token/cost telemetry & SSE tee | Heuristic output monitoring | Exact via local loopback proxy |
| Circuit breaker (`max_consecutive_errors` / budget) | Heuristic error loop detection | Exact HTTP/SSE + USD budget trip (< 1s) |
| Zero secret persistence (`NFR-05`) | Native CLI keyring/OAuth | Host env or daemon RAM-only (`runtime-key`) |

## Custom CLI Agents

Any CLI tool can be registered without modifying Latch via `examples/agents.toml` or the Web Workbench **Agents → + Add Agent** dialog (`PUT /api/v1/agents/{id}`).
