import { useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { getRunMode, type RunMode } from "../utils/tauri";
import "./Content.css";

export default function Content() {
  const [greetMsg, setGreetMsg] = useState("");
  const [name, setName] = useState("");
  const runMode: RunMode = getRunMode();

  async function greet() {
    if (runMode === "desktop") {
      setGreetMsg(await invoke("greet", { name }));
    } else {
      // Web fallback — no Tauri backend available
      setGreetMsg(`Hello, ${name}! (web mode — no Tauri backend)`);
    }
  }

  const statusContent =
    runMode === "desktop"
      ? {
          dotClass: "status-dot-online",
          label: "Tauri Backend Connected",
          description:
            "The Tauri desktop backend is running and ready. Try the greet command below to verify IPC communication.",
        }
      : {
          dotClass: "status-dot-web",
          label: "Web Mode",
          description:
            "Running in browser via Vite dev server. Tauri IPC is unavailable — greet uses local fallback. Use npm run tauri dev for full desktop experience.",
        };

  return (
    <main className="content">
      <header className="content-header">
        <div className="content-breadcrumb">
          <span className="content-breadcrumb-item">Home</span>
          <span className="content-breadcrumb-separator">/</span>
          <span className="content-breadcrumb-item active">Dashboard</span>
        </div>
      </header>

      <div className="content-body">
        <div className="welcome-section">
          <div className="welcome-icon">{"\u{1F4D6}"}</div>
          <h2 className="welcome-title">Welcome to Lexio</h2>
          <p className="welcome-description">
            Your personal reading and learning companion. Organize your library,
            track your progress, and build lasting knowledge.
          </p>
        </div>

        <div className="status-section">
          <h3 className="status-title">Connection Status</h3>
          <div className="status-card">
            <div className="status-indicator">
              <span className={`status-dot ${statusContent.dotClass}`} />
              <span className="status-label">{statusContent.label}</span>
            </div>
            <p className="status-description">{statusContent.description}</p>
          </div>
        </div>

        <div className="demo-section">
          <h3 className="demo-title">Try it out</h3>
          <p className="demo-description">
            Enter a name and click Greet to test the bridge.
          </p>
          <form
            className="greet-form"
            onSubmit={(e) => {
              e.preventDefault();
              greet();
            }}
          >
            <input
              id="greet-input"
              className="greet-input"
              onChange={(e) => setName(e.currentTarget.value)}
              placeholder="Enter a name..."
              value={name}
            />
            <button className="greet-button" type="submit">
              Greet
            </button>
          </form>
          {greetMsg && <p className="greet-response">{greetMsg}</p>}
        </div>
      </div>
    </main>
  );
}
