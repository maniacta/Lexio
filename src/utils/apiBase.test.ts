import { describe, expect, it } from "vitest";
import { apiBase } from "./apiBase";
import { getPlatformLabel, getRunMode, isTauri } from "./tauri";

describe("web runtime", () => {
  it("is not a Tauri shell under vitest", () => {
    expect(isTauri()).toBe(false);
    expect(getRunMode()).toBe("web");
    expect(getPlatformLabel()).toBe("Web");
  });

  it("uses a relative /api path outside Tauri", async () => {
    await expect(apiBase()).resolves.toBe("/api");
  });
});
