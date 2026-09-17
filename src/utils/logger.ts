import { resolveApiToken } from "../api/client";

interface LogEntry {
  level: "info" | "warn" | "error";
  category: string;
  action: string;
  user_action?: string;
  timestamp?: string;
  params_summary?: Record<string, unknown>;
  result_summary?: Record<string, unknown>;
  duration_ms?: number;
  error_message?: string;
}

const BATCH_SIZE = 20;
const FLUSH_INTERVAL = 5000;
const MAX_BUFFER = 100;
// Vite proxy forwards /api/* to backend (same base as the api client).
const API_BASE = "/api";

class Logger {
  private buffer: LogEntry[] = [];
  private flushTimer: ReturnType<typeof setInterval> | null = null;
  private started = false;

  /** Start the logger (buffering + periodic flush). Call once on app init. */
  start() {
    if (this.started) return;
    this.started = true;
    this.flushTimer = setInterval(() => this.flush(), FLUSH_INTERVAL);
    window.addEventListener("beforeunload", () => this.flush());
  }

  /** Stop the logger. */
  stop() {
    if (this.flushTimer) {
      clearInterval(this.flushTimer);
      this.flushTimer = null;
    }
    this.started = false;
  }

  /** Log an event. */
  log(entry: LogEntry) {
    // Attach a per-entry timestamp so batched events keep their own time.
    this.buffer.push({ timestamp: new Date().toISOString(), ...entry });
    if (this.buffer.length > MAX_BUFFER) {
      this.buffer = this.buffer.slice(-MAX_BUFFER);
    }
    if (this.buffer.length >= BATCH_SIZE) {
      void this.flush();
    }
  }

  /** Force-flush the buffer to backend. */
  async flush() {
    if (this.buffer.length === 0) return;
    const batch = this.buffer.splice(0);
    try {
      const token = await resolveApiToken();
      const res = await fetch(`${API_BASE}/logs/batch`, {
        method: "POST",
        headers: {
          "Content-Type": "application/json",
          "X-Lexio-Token": token,
        },
        body: JSON.stringify({ logs: batch }),
      });
      if (res.status >= 500) {
        // Backend-side failure: re-queue and try again later.
        this.buffer = [...batch, ...this.buffer].slice(-MAX_BUFFER);
        console.error("[Logger] flush rejected (" + res.status + "), will retry");
      } else if (!res.ok) {
        // 4xx: the batch itself is invalid — dropping it avoids a retry loop.
        console.error("[Logger] flush rejected (" + res.status + "), dropping batch");
      }
    } catch {
      // Network error (e.g. backend still starting up): re-queue (bounded).
      this.buffer = [...batch, ...this.buffer].slice(-MAX_BUFFER);
      console.error("[Logger] failed to flush logs, will retry");
    }
  }
}

export const logger = new Logger();
