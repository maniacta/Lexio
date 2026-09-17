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
 * Display label for the current runtime ("Desktop" or "Web").
 * Not a React Hook — named without a `use` prefix on purpose.
 */
export function getPlatformLabel(): string {
  return isTauri() ? "Desktop" : "Web";
}
