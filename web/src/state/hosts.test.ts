import { describe, expect, it } from "vitest";
import { extractTokenFromHash } from "./hosts";

describe("extractTokenFromHash (IT-103)", () => {
  it("extracts bearer token from URL fragment and immediately clears it from history", () => {
    window.history.replaceState(null, "", "/#token=latch-secret-fragment-1234567890");
    const token = extractTokenFromHash();
    expect(token).toBe("latch-secret-fragment-1234567890");
    expect(window.location.hash).toBe("");
  });

  it("returns empty string when hash does not include token", () => {
    window.history.replaceState(null, "", "/#section=docs");
    expect(extractTokenFromHash()).toBe("");
    expect(window.location.hash).toBe("#section=docs");
  });
});
