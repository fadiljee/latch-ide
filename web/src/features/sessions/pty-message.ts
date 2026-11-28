export const BRACKETED_PASTE_START = "\x1b[200~";
export const BRACKETED_PASTE_END = "\x1b[201~";

export interface PromptTemplate {
  id: string;
  label: string;
  text: string;
}

const DEFAULT_AGENT_TEMPLATES: readonly PromptTemplate[] = [
  {
    id: "run-tests",
    label: "Run tests & fix",
    text: "Run the test suite, diagnose any failures, and fix them cleanly.",
  },
  {
    id: "summarize-diff",
    label: "Summarize diff",
    text: "Summarize the current git diff, highlight potential edge cases or regressions, and list remaining TODOs.",
  },
  {
    id: "add-tests",
    label: "Add unit tests",
    text: "Add focused unit tests covering happy paths and edge cases for the modified files.",
  },
];

const SHELL_TEMPLATES: readonly PromptTemplate[] = [
  { id: "git-status", label: "git status", text: "git status -sb" },
  { id: "git-diff", label: "git diff --stat", text: "git diff --stat" },
];

const PROFILE_SPECIFIC_TEMPLATES: Record<string, readonly PromptTemplate[]> = {
  claude: [
    { id: "compact", label: "/compact", text: "/compact" },
    ...DEFAULT_AGENT_TEMPLATES,
  ],
  codex: [
    { id: "review", label: "Review changes", text: "Review all uncommitted changes in this worktree for bugs and security issues." },
    ...DEFAULT_AGENT_TEMPLATES,
  ],
  antigravity: [
    { id: "agy-plan", label: "/plan", text: "/plan" },
    { id: "agy-compact", label: "/compact", text: "/compact" },
    { id: "agy-goal", label: "/goal", text: "/goal" },
    { id: "agy-grill", label: "/grill-me", text: "/grill-me" },
    { id: "agy-boost", label: "/boost", text: "/boost" },
    { id: "agy-help", label: "/help", text: "/help" },
    {
      id: "verify",
      label: "Verify & lint",
      text: "Run lint, typecheck, and tests; fix every warning before reporting done.",
    },
    ...DEFAULT_AGENT_TEMPLATES,
  ],
  aider: [
    { id: "aider-diff", label: "/diff", text: "/diff" },
    { id: "aider-test", label: "/test", text: "/test" },
    ...DEFAULT_AGENT_TEMPLATES,
  ],
  shell: SHELL_TEMPLATES,
  sleeper: SHELL_TEMPLATES,
};

export function templatesForProfile(profile: string): readonly PromptTemplate[] {
  return PROFILE_SPECIFIC_TEMPLATES[profile] ?? DEFAULT_AGENT_TEMPLATES;
}

export function defaultBracketedPaste(profile: string): boolean {
  return profile !== "shell" && profile !== "sleeper";
}

/**
 * Memformat pesan teks untuk dikirim ke PTY dengan opsi bracketed paste (IT-113).
 * Urutan penutup `\x1b[201~` di dalam teks mentah dibuang agar tidak terjadi breakout injeksi.
 */
export function formatPtyMessage(
  rawText: string,
  options: { bracketedPaste: boolean; trailingEnter?: boolean },
): string {
  const normalized = rawText.replace(/\r\n/g, "\n");
  const enter = options.trailingEnter === false ? "" : "\r";
  if (!options.bracketedPaste) {
    return `${normalized}${enter}`;
  }
  const safe = normalized.split(BRACKETED_PASTE_END).join("");
  return `${BRACKETED_PASTE_START}${safe}${BRACKETED_PASTE_END}${enter}`;
}

/**
 * Merender pratinjau aman (tanpa karakter kontrol mentah) atas payload PTY yang akan dikirim.
 */
export function previewPtyMessage(
  rawText: string,
  options: { bracketedPaste: boolean },
): string {
  const payload = formatPtyMessage(rawText, options);
  return payload
    .split(BRACKETED_PASTE_START)
    .join("[PASTE_START]")
    .split(BRACKETED_PASTE_END)
    .join("[PASTE_END]")
    .replace(/\r/g, "↵");
}
