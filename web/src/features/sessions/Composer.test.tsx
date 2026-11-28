import { fireEvent, render, screen } from "@testing-library/react";
import { describe, expect, it } from "vitest";
import { CATALOGS } from "../../lib/i18n/catalog";
import { Composer } from "./Composer";
import {
  BRACKETED_PASTE_END,
  BRACKETED_PASTE_START,
  formatPtyMessage,
  previewPtyMessage,
} from "./pty-message";

describe("PTY Composer & bracketed paste (IT-113)", () => {
  it("wraps message in bracketed paste sequences and neutralizes breakout terminators", () => {
    const raw = `line1\n${BRACKETED_PASTE_END}rm -rf /\nline2`;
    const encoded = formatPtyMessage(raw, { bracketedPaste: true });
    expect(encoded.startsWith(BRACKETED_PASTE_START)).toBe(true);
    expect(encoded.endsWith(`${BRACKETED_PASTE_END}\r`)).toBe(true);
    // Pastikan urutan penutup hanya muncul sekali di akhir (anti-breakout)
    expect(encoded.split(BRACKETED_PASTE_END).length).toBe(2);

    const plain = formatPtyMessage("ls -la", { bracketedPaste: false });
    expect(plain).toBe("ls -la\r");

    const preview = previewPtyMessage("hello\nworld", { bracketedPaste: true });
    expect(preview).toBe("[PASTE_START]hello\nworld[PASTE_END]↵");
  });

  it("populates profile templates and renders safe payload preview", () => {
    const t = CATALOGS.en;
    render(
      <Composer
        token="tok"
        sessionId="ses-1"
        agentProfile="claude"
        labels={t.sessionActions}
      />,
    );

    // Templat khusus claude tersedia
    fireEvent.click(screen.getByRole("button", { name: "/compact" }));
    const box = screen.getByPlaceholderText(t.sessionActions.composerPlaceholder);
    expect(box).toHaveValue("/compact");

    // Aktifkan pratinjau payload PTY
    fireEvent.click(screen.getByRole("button", { name: t.sessionActions.previewToggle }));
    expect(screen.getByText(t.sessionActions.previewHeading)).toBeInTheDocument();
    expect(screen.getByText("[PASTE_START]/compact[PASTE_END]↵")).toBeInTheDocument();
  });
});
