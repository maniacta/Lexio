import { isTauri } from "./tauri";

/**
 * Base URL for the local backend API.
 *
 * Web mode runs behind the Vite dev server, which proxies `/api` to the
 * backend, so a relative path is correct there. The packaged desktop app is
 * different: its page is served from the `tauri://` protocol, where a relative
 * `/api` would be resolved against the bundled assets (and answered with
 * `index.html`), never reaching Axum. Desktop therefore needs an absolute
 * loopback URL, built from the port the Rust side actually bound.
 *
 * The port is resolved once and memoized, and is fetched over Tauri IPC, so it
 * never depends on a value the webview could have been tricked into reading.
 */
let basePromise: Promise<string> | null = null;

/** Port fallback when invoked outside a Tauri context (tests, web mode). */
const DEFAULT_PORT = 3001;

async function resolveBase(): Promise<string> {
  if (!isTauri()) return "/api";
  try {
    const { invoke } = await import("@tauri-apps/api/core");
    const port = await invoke<number>("get_api_port");
    if (typeof port === "number" && Number.isFinite(port) && port > 0) {
      return `http://127.0.0.1:${port}/api`;
    }
  } catch {
    // Falls through to the default: the request then surfaces a normal
    // NETWORK_ERROR rather than failing at an unexpected place.
  }
  return `http://127.0.0.1:${DEFAULT_PORT}/api`;
}

/** Resolve (and memoize) the API base URL. */
export async function apiBase(): Promise<string> {
  if (!basePromise) {
    basePromise = resolveBase().catch((err) => {
      basePromise = null;
      throw err;
    });
  }
  return basePromise;
}