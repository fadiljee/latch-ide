import { fireEvent, render, screen } from "@testing-library/react";
import { describe, expect, it } from "vitest";
import { App } from "./App";

describe("Latch IDE AppShell", () => {
  it("renders shell landmarks, navigates routes, opens/closes dialogs, and toggles locale", () => {
    render(<App />);

    expect(screen.getByRole("banner")).toBeInTheDocument();
    expect(screen.getByRole("heading", { name: "Latch IDE" })).toBeInTheDocument();
    expect(screen.getByRole("complementary", { name: "Daftar Sesi" })).toBeInTheDocument();
    expect(screen.getByRole("main")).toBeInTheDocument();
    expect(screen.getByRole("contentinfo")).toBeInTheDocument();

    // Buka dan tutup dialog New Session
    const newBtn = screen.getAllByRole("button", { name: "+ Sesi Baru" })[0];
    expect(newBtn).toBeDefined();
    if (newBtn) {
      fireEvent.click(newBtn);
    }
    expect(screen.getByRole("dialog", { name: "Buat Sesi Agen Baru" })).toBeInTheDocument();
    fireEvent.keyDown(window, { key: "Escape" });
    expect(screen.queryByRole("dialog")).not.toBeInTheDocument();

    // Buka dan tutup Command Palette via keyboard shortcut
    fireEvent.keyDown(window, { key: "k", ctrlKey: true });
    expect(screen.getByRole("dialog", { name: "Command Palette" })).toBeInTheDocument();
    fireEvent.keyDown(window, { key: "Escape" });
    expect(screen.queryByRole("dialog")).not.toBeInTheDocument();

    // Navigasi ke Settings dan ganti gaya tampilan
    fireEvent.click(screen.getByRole("button", { name: "Pengaturan" }));
    expect(screen.getByRole("heading", { name: "Pengaturan Tampilan & Koneksi" })).toBeInTheDocument();
    fireEvent.click(screen.getByRole("button", { name: "Bento" }));
    expect(document.documentElement.dataset.style).toBe("bento");

    // Navigasi ke Automasi
    fireEvent.click(screen.getByRole("button", { name: "Automasi" }));
    expect(screen.getByRole("heading", { name: "Pipeline DAG & Arena" })).toBeInTheDocument();

    // Toggle bahasa ID -> EN
    fireEvent.click(screen.getByRole("button", { name: "Ganti bahasa antarmuka" }));
    expect(screen.getByRole("heading", { name: "DAG Pipelines & Arena" })).toBeInTheDocument();
  });
});
