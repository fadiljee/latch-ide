import { describe, expect, it } from "vitest";
import { computeAccentSoft, contrastOnAccent } from "./accent";
import { applyTheme, type ThemeMode, type ThemeStyle } from "./apply-theme";

describe("Theme manager", () => {
  it("applies all 4 styles across dark and light modes and sets accent tokens", () => {
    const styles: ThemeStyle[] = ["tonal", "ledger", "bento", "glass"];
    const modes: ThemeMode[] = ["dark", "light"];

    for (const style of styles) {
      for (const mode of modes) {
        const resolved = applyTheme({ style, mode, accent: "#10b981" });
        expect(resolved).toBe(mode);
        expect(document.documentElement.dataset.style).toBe(style);
        expect(document.documentElement.dataset.mode).toBe(mode);
        expect(document.documentElement.style.getPropertyValue("--accent")).toBe("#10b981");
        expect(document.documentElement.style.getPropertyValue("--accent-soft")).toBe(
          computeAccentSoft("#10b981"),
        );
      }
    }

    expect(contrastOnAccent("#111827")).toBe("#ffffff");
    expect(contrastOnAccent("#f8fafc")).toBe("#0f1114");
  });
});
