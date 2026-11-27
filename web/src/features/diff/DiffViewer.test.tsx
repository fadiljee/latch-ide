import { fireEvent, render, screen } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";
import { DiffViewer } from "./DiffViewer";

describe("DiffViewer", () => {
  it("renders file stats, patch lines, and toggles unified mode", () => {
    const onClose = vi.fn();
    render(
      <DiffViewer
        title="Session Diff Review"
        closeLabel="Close"
        onClose={onClose}
        diff={{
          base_branch: "main",
          files: [
            {
              path: "src/lib.rs",
              old_path: null,
              status: "modified",
              additions: 1,
              deletions: 1,
            },
          ],
          patch: "--- a/src/lib.rs\n+++ b/src/lib.rs\n-old\n+new",
          total_patch_bytes: 42,
          truncated: false,
        }}
      />,
    );

    expect(screen.getByText("src/lib.rs")).toBeInTheDocument();
    expect(screen.getByText("-old")).toBeInTheDocument();
    expect(screen.getByText("+new")).toBeInTheDocument();

    fireEvent.click(screen.getByRole("button", { name: "Unified" }));
    expect(screen.getByRole("button", { name: "Side-by-Side" })).toBeInTheDocument();

    fireEvent.click(screen.getByRole("button", { name: "Close" }));
    expect(onClose).toHaveBeenCalledTimes(1);
  });
});
