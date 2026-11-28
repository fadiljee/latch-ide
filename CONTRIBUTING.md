# Contribution Rules

These rules apply to every contribution to Latch (`latch-ide`), from a typo fix to a new feature. Read them before you open a pull request.

## 1. Before You Start

- Open or pick an issue before large changes. Wait for agreement on the approach.
- Work on one task per pull request. Keep each PR small and focused.
- Ask a maintainer first before you:
  - add a large dependency or replace a core library;
  - change a released API or data schema;
  - loosen any sandbox or security rule;
  - add a new target OS or a public network feature.

## 2. Branches and Commits

- Branch from `dev`. Name branches `feat/<area>-<summary>` or `fix/<area>-<summary>`.
- Open pull requests against `dev`. Only maintainers merge `dev` into `main`.
- Write commit messages in English using [Conventional Commits](https://www.conventionalcommits.org/): `feat`, `fix`, `docs`, `refactor`, `test`, `perf`, `build`, `ci`, `chore`.
- Use the affected area as the scope, for example `fix(pty-pool): drop stale subscribers`.
- Never rewrite shared history or force-push to `main` or `dev`.

## 3. Code Rules

- No `unsafe` code unless it is unavoidable. Every `unsafe` block needs a `// SAFETY:` comment.
- No `unwrap()`, `expect()` or `panic!()` in production code paths. They are allowed in tests.
- Never block the async runtime. Keep blocking work on dedicated threads.
- Use bounded channels and bounded buffers.
- Use structured logging, never `println!`, in library and daemon code.
- Never log or store API keys, `Authorization` headers, tokens or full prompts.
- Keep crate dependencies one-way, with no cycles. The core crate depends on no other workspace crate.
- Keep the application small. Check binary size and memory impact before adding a dependency, and disable unused features.
- Do not add heavyweight runtimes or UI suites such as Electron or Monaco.
- Frontend code uses strict TypeScript, accessible markup and keyboard navigation. All user-facing text goes through the translation catalog.
- Database schema changes go in a new migration. Never edit an old migration.
- Store time as UTC. Use UUID v7 for IDs unless the docs say otherwise.

## 4. Tests and Checks

Every change needs tests. A bug fix needs a test that fails before the fix.

All checks must pass before you request review:

```bash
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
cargo nextest run --workspace      # or: cargo test --workspace

# only when you touch web/
(cd web && npm ci && npm run lint && npm run typecheck && npm run test && npm run build)
```

Do not weaken, skip or delete a test to make CI pass. Tests must also pass on Linux, macOS and Windows unless they are explicitly Linux-only.

## 5. Documentation

- Update the affected docs in the same pull request.
- Add an entry under `[Unreleased]` in [`CHANGELOG.md`](CHANGELOG.md).
- If code and docs disagree, do not pick one silently. Fix both, or propose a decision record.

## 6. Pull Requests

Every pull request must include:

- a summary of what changed and why;
- the requirement or task IDs it touches (`FR-xx`, `NFR-xx`, `T-xx`);
- the commands you ran to test it;
- the impact on memory use and binary size, if any.

Checklist:

- [ ] Format, lint and tests pass
- [ ] Docs updated
- [ ] `CHANGELOG.md` updated under `[Unreleased]`
- [ ] No secrets, keys or sensitive headers logged or stored
- [ ] No security or sandbox rule loosened without maintainer approval

## 7. Security

Never open a public issue or pull request for a vulnerability. Follow [`SECURITY.md`](SECURITY.md).

Never commit secrets, real `.env` files, or user telemetry. Never run destructive commands such as `rm -rf`, `git clean -fdx` or `git push --force` outside a worktree that Latch created.

## 8. AI-Assisted Contributions

AI agents may help, but the human author is responsible for the result. Read every line you submit. Agents working in this repository must follow [`AGENTS.md`](AGENTS.md).

## 9. Code of Conduct

Be respectful, direct and constructive. Critique code and design decisions, never people. Maintainers may remove comments or contributions that break this rule.

## 10. License

By contributing, you agree that your work is released under the [Apache License, Version 2.0](LICENSE).
