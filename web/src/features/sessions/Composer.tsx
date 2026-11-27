import { useState } from "react";
import { Button } from "../../components/ui/Button";
import type { Catalog } from "../../lib/i18n/catalog";
import { useMessagesStore } from "../../state/messages";
import { sendSessionInput } from "../../transport/daemon-client";

interface ComposerProps {
  token: string;
  sessionId: string;
  labels: Catalog["sessionActions"];
  disabled?: boolean;
}

export function Composer({ token, sessionId, labels, disabled = false }: ComposerProps) {
  const [text, setText] = useState("");
  const [busy, setBusy] = useState(false);
  const appendMessage = useMessagesStore((s) => s.appendMessage);

  const handleSubmit = async (ev: React.FormEvent) => {
    ev.preventDefault();
    const trimmed = text.trim();
    if (!trimmed || !token || disabled) {
      return;
    }
    setBusy(true);
    try {
      await sendSessionInput(token, sessionId, `${trimmed}\n`);
      appendMessage(sessionId, "user", trimmed);
      setText("");
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

  return (
    <form
      onSubmit={(e) => void handleSubmit(e)}
      className="flex items-center gap-2 border-t border-line bg-surface px-4 py-3"
    >
      <input
        type="text"
        value={text}
        onChange={(e) => setText(e.target.value)}
        disabled={disabled || busy || !token}
        placeholder={labels.composerPlaceholder}
        aria-label={labels.composerPlaceholder}
        className="min-w-0 flex-1 rounded-md border border-line bg-app px-3 py-1.5 text-xs text-fg placeholder:text-fg-3 focus:border-accent focus:outline-none"
      />
      <Button type="submit" variant="primary" size="md" disabled={disabled || busy || !text.trim()}>
        {labels.send}
      </Button>
    </form>
  );
}
