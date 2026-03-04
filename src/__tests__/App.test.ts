import { invoke } from "@tauri-apps/api/core";
import { describe, expect, it, vi } from "vitest";
import { pingBackend } from "../lib/tauriCommands";

vi.mock("@tauri-apps/api/core", () => ({
  invoke: vi.fn(),
}));

describe("App hello world", () => {
  it("maps frontend call to backend ping command", async () => {
    const mockedInvoke = vi.mocked(invoke);
    mockedInvoke.mockResolvedValue("Hello from Rust: Hello world");
    const result = await pingBackend("Hello world");

    expect(result).toBe("Hello from Rust: Hello world");
    expect(mockedInvoke).toHaveBeenCalledWith("ping_backend", {
      message: "Hello world",
    });
  });

  it("propagates backend command errors", async () => {
    const mockedInvoke = vi.mocked(invoke);
    mockedInvoke.mockRejectedValue(new Error("backend unavailable"));
    await expect(pingBackend("Hello world")).rejects.toThrow(
      "backend unavailable",
    );
  });
});
