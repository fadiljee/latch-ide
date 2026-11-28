import { useEffect, useState } from "react";
import { Button } from "../../components/ui/Button";
import type { Catalog } from "../../lib/i18n/catalog";
import { useMessagesStore } from "../../state/messages";
import { sendSessionInput } from "../../transport/daemon-client";
import {
  defaultBracketedPaste,
  formatPtyMessage,
  previewPtyMessage,
  templatesForProfile,
} from "./pty-message";

interface ComposerProps {
  token: string;
  sessionId: string;
  agentProfile: string;
  labels: Catalog["sessionActions"];
  disabled?: boolean;
}

export function Composer({
  token,
  sessionId,
  agentProfile,
  labels,
  disabled = false,
}: ComposerProps) {
  const [text, setText] = useState("");
  const [busy, setBusy] = useState(false);
  const [bracketedPaste, setBracketedPaste] = useState(() =>
    defaultBracketedPaste(agentProfile),
  );
  const [showPreview, setShowPreview] = useState(false);
  const appendMessage = useMessagesStore((s) => s.appendMessage);

  useEffect(() => {
    setBracketedPaste(defaultBracketedPaste(agentProfile));
  }, [agentProfile]);

  const templates = templatesForProfile(agentProfile);

  const sendPayload = async () => {
    const trimmed = text.trim();
    if (!trimmed || !token || disabled) {
      return;
    }
    setBusy(true);
    try {
      const encoded = formatPtyMessage(trimmed, { bracketedPaste });
      await sendSessionInput(token, sessionId, encoded);
      appendMessage(sessionId, "user", trimmed);
      setText("");
      setShowPreview(false);
    } catch (err) {
      appendMessage(
        sessionId,
        "system",
        err instanceof Error ? err.message : "Failed sending input",
      );
    } finally {
      setBusy(false);
    }
  };

  const handleSubmit = (ev: React.FormEvent) => {
    ev.preventDefault();
    void sendPayload();
  };

  const handleKeyDown = (ev: React.KeyboardEvent<HTMLTextAreaElement>) => {
    if (ev.key === "Enter" && !ev.shiftKey) {
      ev.preventDefault();
      void sendPayload();
    }
  };

  return (
    <form
      onSubmit={handleSubmit}
      className="space-y-2 border-t border-line bg-surface px-4 py-3 text-xs"
    >
      <div className="flex flex-wrap items-center justify-between gap-2">
        <div
          role="group"
          aria-label={labels.templatesLabel}
          className="flex flex-wrap items-center gap-1"
        >
          {templates.map((tpl) => (
            <button
              key={tpl.id}
              type="button"
              disabled={disabled || busy}
              onClick={() => setText(tpl.text)}
              className="rounded-sm border border-line bg-app px-2 py-0.5 text-[11px] text-fg-2 transition-colors hover:bg-hover hover:text-fg disabled:opacity-50"
            >
              {tpl.label}
            </button>
          ))}
        </div>

        <div className="flex items-center gap-1.5">
          <button
            type="button"
            aria-pressed={bracketedPaste}
            onClick={() => setBracketedPaste((v) => !v)}
            className={
              bracketedPaste
                ? "rounded-sm border border-accent/50 bg-accent-soft px-2 py-0.5 font-mono text-[11px] text-accent"
                : "rounded-sm border border-line bg-app px-2 py-0.5 font-mono text-[11px] text-fg-3 hover:text-fg"
            }
          >
            {labels.bracketedPaste}
          </button>
          <button
            type="button"
            aria-pressed={showPreview}
            onClick={() => setShowPreview((v) => !v)}
            className={
              showPreview
                ? "rounded-sm border border-line-strong bg-raised px-2 py-0.5 text-[11px] text-fg"
                : "rounded-sm border border-line bg-app px-2 py-0.5 text-[11px] text-fg-3 hover:text-fg"
            }
          >
            {labels.previewToggle}
          </button>
        </div>
      </div>

      {showPreview && text.trim() ? (
        <div className="rounded-md border border-line bg-app p-2">
          <p className="mb-1 text-[10px] font-medium text-fg-3">{labels.previewHeading}</p>
          <pre className="max-h-28 overflow-auto font-mono text-[11px] whitespace-pre-wrap text-fg-2">
            {previewPtyMessage(text.trim(), { bracketedPaste })}
          </pre>
        </div>
      ) : null}

      <div className="flex items-end gap-2">
        <textarea
          rows={2}
          value={text}
          onChange={(e) => setText(e.target.value)}
          onKeyDown={handleKeyDown}
          disabled={disabled || busy || !token}
          placeholder={labels.composerPlaceholder}
          aria-label={labels.composerPlaceholder}
          className="min-w-0 flex-1 resize-none rounded-md border border-line bg-app px-3 py-1.5 text-xs text-fg placeholder:text-fg-3 focus:border-accent focus:outline-none"
        />
        <Button type="submit" variant="primary" size="md" disabled={disabled || busy || !text.trim()}>
          {labels.send}
        </Button>
      </div>
    </form>
  );
}
