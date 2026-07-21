/**
 * Detect whether the app is running inside a Tauri desktop shell.
 * When running via `npm run dev` (pure web), this returns false.
 */
export function isTauri(): boolean {
  return typeof window !== "undefined" && "__TAURI_INTERNALS__" in window;
}

export type RunMode = "desktop" | "web";

export function getRunMode(): RunMode {
  return isTauri() ? "desktop" : "web";
}

/**
 * React hook: returns the platform display label ("Desktop" or "Web").
 */
export function usePlatform(): string {
  return isTauri() ? "Desktop" : "Web";
}
