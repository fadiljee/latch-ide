import { useState } from "react";
import { Button } from "../../components/ui/Button";
import { Input } from "../../components/ui/Input";
import { Segmented } from "../../components/ui/Segmented";
import type { Catalog } from "../../lib/i18n/catalog";
import { useHostsStore } from "../../state/hosts";
import { useUiStore } from "../../state/ui";
import { ACCENT_PRESETS } from "../../theme/accent";
import type { ThemeMode, ThemeStyle } from "../../theme/apply-theme";
import { useThemeStore } from "../../theme/theme-store";

interface SettingsPageProps {
  labels: Catalog["pages"];
  connection: Catalog["connection"];
  localeToggleLabel: string;
}

const STYLE_OPTIONS: readonly { value: ThemeStyle; label: string }[] = [
  { value: "tonal", label: "Tonal" },
  { value: "ledger", label: "Ledger" },
  { value: "bento", label: "Bento" },
  { value: "glass", label: "Glass" },
];

const MODE_OPTIONS: readonly { value: ThemeMode; label: string }[] = [
  { value: "dark", label: "Dark" },
  { value: "light", label: "Light" },
  { value: "system", label: "System" },
];

export function SettingsPage({
  labels,
  connection,
  localeToggleLabel,
}: SettingsPageProps) {
  const style = useThemeStore((s) => s.style);
  const mode = useThemeStore((s) => s.mode);
  const accent = useThemeStore((s) => s.accent);
  const setStyle = useThemeStore((s) => s.setStyle);
  const setMode = useThemeStore((s) => s.setMode);
  const setAccent = useThemeStore((s) => s.setAccent);
  const locale = useUiStore((s) => s.locale);
  const setLocale = useUiStore((s) => s.setLocale);
  const token = useHostsStore((s) => s.token);
  const setToken = useHostsStore((s) => s.setToken);

  const [tokenDraft, setTokenDraft] = useState(token);

  const handleSaveToken = (ev: React.FormEvent) => {
    ev.preventDefault();
    setToken(tokenDraft);
  };

  return (
    <div className="flex-1 space-y-6 overflow-y-auto p-5 text-xs">
      <div>
        <h2 className="text-base font-semibold text-fg">{labels.settingsHeading}</h2>
        <p className="mt-0.5 text-fg-3">{labels.settingsSubtitle}</p>
      </div>

      <div className="grid max-w-2xl gap-5 rounded-lg border border-line bg-surface p-4">
        <div className="space-y-1.5">
          <span className="block font-medium text-fg-2">{labels.styleLabel}</span>
          <Segmented
            label={labels.styleLabel}
            value={style}
            options={STYLE_OPTIONS}
            onChange={setStyle}
          />
        </div>

        <div className="space-y-1.5">
          <span className="block font-medium text-fg-2">{labels.modeLabel}</span>
          <Segmented
            label={labels.modeLabel}
            value={mode}
            options={MODE_OPTIONS}
            onChange={setMode}
          />
        </div>

        <div className="space-y-1.5">
          <span className="block font-medium text-fg-2">{labels.accentLabel}</span>
          <div className="flex flex-wrap gap-2">
            {ACCENT_PRESETS.map((preset) => (
              <button
                key={preset.id}
                type="button"
                aria-label={preset.label}
                aria-pressed={accent === preset.hex}
                onClick={() => setAccent(preset.hex)}
                className="flex items-center gap-2 rounded-md border border-line bg-app px-2.5 py-1.5 text-fg-2 hover:bg-hover hover:text-fg"
              >
                <span
                  aria-hidden="true"
                  className="inline-block h-3 w-3 rounded-full"
                  style={{ backgroundColor: preset.hex }}
                />
                <span>{preset.label}</span>
              </button>
            ))}
          </div>
        </div>

        <div className="space-y-1.5">
          <span className="block font-medium text-fg-2">{labels.languageLabel}</span>
          <Button aria-label={localeToggleLabel} onClick={() => setLocale(locale === "id" ? "en" : "id")}>
            {locale.toUpperCase()}
          </Button>
        </div>

        <form onSubmit={handleSaveToken} className="space-y-2 border-t border-line pt-4">
          <label htmlFor="settings-token" className="block">
            <span className="mb-1 block font-medium text-fg-2">{labels.tokenLabel}</span>
            <div className="flex gap-2">
              <Input
                id="settings-token"
                type="password"
                value={tokenDraft}
                onChange={(e) => setTokenDraft(e.target.value)}
                placeholder={connection.tokenPlaceholder}
              />
              <Button type="submit" variant="primary">
                {connection.connectButton}
              </Button>
            </div>
          </label>
        </form>
      </div>
    </div>
  );
}
