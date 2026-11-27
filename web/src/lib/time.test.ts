import { describe, expect, it } from "vitest";
import { formatRelativeShort } from "./time";

describe("formatRelativeShort", () => {
  it("formats elapsed durations concisely", () => {
    const now = 1_700_000_000_000;
    expect(formatRelativeShort(now - 20_000, now)).toBe("now");
    expect(formatRelativeShort(now - 49 * 60_000, now)).toBe("49m");
    expect(formatRelativeShort(now - 3 * 3_600_000, now)).toBe("3h");
    expect(formatRelativeShort(now - (4 * 24 + 6) * 3_600_000, now)).toBe("4d 6h");
  });
});
