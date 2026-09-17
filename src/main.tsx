import React from "react";
import ReactDOM from "react-dom/client";
import App from "./App";
import { logger } from "./utils/logger";

// Start the audit logger (buffered, flushed to the backend in batches).
logger.start();

// Global error capture → audit log.
window.addEventListener("unhandledrejection", (e) => {
  logger.log({
    level: "error",
    category: "ui",
    action: "unhandled_rejection",
    error_message: e.reason instanceof Error ? e.reason.message : String(e.reason),
  });
});
window.addEventListener("error", (e) => {
  logger.log({
    level: "error",
    category: "ui",
    action: "window_error",
    error_message: e.message,
  });
});

ReactDOM.createRoot(document.getElementById("root") as HTMLElement).render(
  <React.StrictMode>
    <App />
  </React.StrictMode>,
);
