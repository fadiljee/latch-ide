export interface AgentProfileOption {
  id: string;
  provider: string;
}

export const BUILTIN_PROFILES: readonly AgentProfileOption[] = [
  { id: "claude", provider: "anthropic" },
  { id: "codex", provider: "openai" },
  { id: "gemini", provider: "google" },
  { id: "opencode", provider: "openai" },
  { id: "shell", provider: "local" },
  { id: "sleeper", provider: "local" },
];

interface ModelPickerProps {
  id: string;
  label: string;
  value: string;
  onChange: (profile: string) => void;
}

export function ModelPicker({ id, label, value, onChange }: ModelPickerProps) {
  return (
    <label htmlFor={id} className="block text-xs">
      <span className="mb-1 block text-fg-2">{label}</span>
      <select
        id={id}
        value={value}
        onChange={(e) => onChange(e.target.value)}
        className="w-full rounded-md border border-line bg-app px-2.5 py-1.5 text-xs text-fg focus:border-accent focus:outline-none"
      >
        {BUILTIN_PROFILES.map((p) => (
          <option key={p.id} value={p.id}>
            {p.id} ({p.provider})
          </option>
        ))}
      </select>
    </label>
  );
}
