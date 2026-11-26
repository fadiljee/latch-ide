import { fireEvent, render, screen } from "@testing-library/react";
import { describe, expect, it } from "vitest";
import { App } from "./App";

describe("Latch Workbench App", () => {
  it("renders accessible mission control shell and toggles i18n locale", () => {
    render(<App />);
    expect(screen.getByRole("heading", { name: "Latch Workbench" })).toBeInTheDocument();
    expect(screen.getByRole("heading", { name: "Papan Sesi" })).toBeInTheDocument();

    const toggle = screen.getByRole("button", { name: "Ganti bahasa antarmuka" });
    fireEvent.click(toggle);
    expect(screen.getByRole("heading", { name: "Session Board" })).toBeInTheDocument();
  });
});
