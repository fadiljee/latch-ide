import { fireEvent, render, screen } from "@testing-library/react";
import { describe, expect, it } from "vitest";
import { App } from "./App";

describe("Latch Workbench App", () => {
  it("renders accessible desktop shell and toggles i18n locale", () => {
    render(<App />);
    expect(screen.getByRole("heading", { name: "Latch Workbench" })).toBeInTheDocument();
    expect(screen.getAllByRole("button", { name: "+ Sesi baru" }).length).toBeGreaterThan(0);

    const toggle = screen.getByRole("button", { name: "Ganti bahasa antarmuka" });
    fireEvent.click(toggle);
    expect(screen.getAllByRole("button", { name: "+ New session" }).length).toBeGreaterThan(0);
  });
});
