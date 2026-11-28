import { cleanup, fireEvent, render, screen, waitFor } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { useHostsStore } from "../state/hosts";
import { UpdateBanner } from "./UpdateBanner";

function mockUpdate(body: unknown) {
  vi.stubGlobal(
    "fetch",
    vi.fn(async () => new Response(JSON.stringify(body), { status: 200 })),
  );
}

describe("UpdateBanner", () => {
  beforeEach(() => {
    window.localStorage.clear();
    useHostsStore.setState({ token: "tok", connected: true });
  });
  afterEach(() => {
    cleanup();
    vi.unstubAllGlobals();
  });

  it("shows a notice when a newer version exists and remembers dismissal", async () => {
    mockUpdate({
      current: "1.2.0",
      latest: "v1.3.0",
      update_available: true,
      release_url: "https://example.test/r",
      check_enabled: true,
    });
    render(<UpdateBanner />);
    expect(await screen.findByRole("status")).toHaveTextContent("v1.3.0");
    expect(screen.getByText("latch update")).toBeInTheDocument();

    fireEvent.click(screen.getByRole("button"));
    await waitFor(() => expect(screen.queryByRole("status")).not.toBeInTheDocument());
    expect(window.localStorage.getItem("latch_update_dismissed")).toBe("v1.3.0");
  });

  it("renders nothing when already up to date", async () => {
    mockUpdate({
      current: "1.3.0",
      latest: "v1.3.0",
      update_available: false,
      release_url: null,
      check_enabled: true,
    });
    render(<UpdateBanner />);
    await waitFor(() => expect(fetch).toHaveBeenCalled());
    expect(screen.queryByRole("status")).not.toBeInTheDocument();
  });
});
