# Security Policy

Latch runs autonomous AI coding agents on your machine, so security is part of the design. This page explains the protections Latch provides, their limits, and how to report a problem.

## Reporting a Vulnerability

Do not report vulnerabilities through public issues, discussions or pull requests.

Report privately through GitHub Security Advisories:
https://github.com/fadiljee/latch-ide/security/advisories/new

Please include:

- a clear description of the problem and its impact;
- steps to reproduce, or a minimal proof of concept;
- the affected part (command, endpoint, sandbox rule or UI) and your environment (OS, kernel, versions).

We acknowledge reports within **72 hours**. We agree on a fix and disclosure timeline with you privately before anything is made public.

## Supported Versions

Security fixes go to the latest release and the `main` branch. Please upgrade to the latest release before reporting.

## Security Protections in Latch

### Local-only access
- The control channel is a local socket with owner-only permissions. Latch also checks that the connecting process belongs to the same user.
- The web and WebSocket interfaces listen on `127.0.0.1` only. Latch has no public relay. For remote use, tunnel over SSH or a private network such as Tailscale or WireGuard.
- Access tokens are random, at least 256 bits, stored in an owner-only file, compared in constant time, and never placed in URLs sent to servers.
- Tokens can be scoped, so a client gets only the access it needs. Requests with a missing, invalid or out-of-scope token are rejected and recorded in the audit log.
- Requests from a foreign web origin are rejected.

### Isolation between agents
- Each agent runs in its own git worktree and process group. One agent cannot overwrite another agent's files.
- A crash or forced kill of an agent does not stop Latch or other sessions. On restart, Latch cleans up dead sessions and orphaned worktrees.
- Agents start with a minimal environment. They inherit a secret variable only if you list it explicitly.

### Sandbox and approvals
- On Linux, agents run in a default-deny filesystem sandbox. They can write only to their worktree and a private temp directory, and can only read approved toolchain paths.
- Private locations such as `~/.ssh`, `~/.aws` and `~/.gnupg`, and other projects' files, are blocked. Symlink escapes are blocked too.
- Outbound network access can be limited to an allowlist of domains.
- Risky actions go through an approval gate. If you do not answer in time, the action is denied. Every decision is recorded.
- The built-in shell for agents runs inside the sandbox. A shell on the host is off by default and every attempt to use it is audited.

### Secrets and privacy
- Latch never stores API keys or `Authorization` headers.
- Prompts and responses are not stored by default. Latch keeps only metrics such as tokens, cost, model and latency.
- Logs and stored summaries pass through a redaction layer that masks common key and token patterns.
- Terminal logs are owner-only, and you can turn them off.
- Latch does not send your code, prompts or keys anywhere on its own.

### Safe editing and search
- File access is confined to the session worktree. Paths that escape it, including through symlinks, are refused.
- Project search runs in bounded time and memory, and cannot be used for regex denial-of-service.
- Language servers run inside the sandbox with resource limits. Dangerous commands, such as `workspace/executeCommand`, are blocked by default.
- Text that comes from agents is treated as data, never as a command to run.

### Untrusted repositories
- A workflow file from an untrusted repository is treated as untrusted code. Latch shows a summary and asks for confirmation before it runs.
- Workflow shell steps run under the same sandbox and can never relax it.

### Budget and loop protection
- Spending limits and loop detection stop a runaway agent in under one second.

### Supply chain
- CI scans dependencies for known vulnerabilities and bans risky or unknown sources.
- Lockfiles are committed, and installs are reproducible.
- The web interface ships inside the app with a strict content policy and loads nothing from the internet.
- Installers verify SHA-256 checksums.

## Limitations

- The filesystem sandbox is Linux-only. On macOS and Windows, Latch runs without it and shows a clear warning.
- Domain filtering of outbound traffic depends on the agent respecting proxy settings, unless kernel network rules are enabled.
- Approval gates cover the actions Latch can intercept. They do not cover every system call inside an agent.
- An agent that you allow to use its own CLI login can read its own login tokens.
- Prompt injection cannot be fully prevented. The sandbox and approvals limit the damage, but they do not stop the attempt.

For the full threat model, see [`docs/SECURITY.md`](docs/SECURITY.md) and [`docs/ide/IDE_SECURITY.md`](docs/ide/IDE_SECURITY.md).
