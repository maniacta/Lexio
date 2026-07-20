import { getRunMode } from "../utils/tauri";
import "./Sidebar.css";

type NavItem = {
  id: string;
  label: string;
  icon: string;
  active: boolean;
};

const navItems: NavItem[] = [
  { id: "home", label: "Home", icon: "\u{1F3E0}", active: true },
  { id: "library", label: "Library", icon: "\u{1F4DA}", active: false },
  { id: "settings", label: "Settings", icon: "\u2699\uFE0F", active: false },
];

export default function Sidebar() {
  const runMode = getRunMode();

  return (
    <aside className="sidebar">
      <div className="sidebar-header">
        <span className="sidebar-logo">{"\u{1F4D6}"}</span>
        <h1 className="sidebar-title">Lexio</h1>
      </div>

      <nav className="sidebar-nav">
        <ul className="sidebar-nav-list">
          {navItems.map((item) => (
            <li key={item.id}>
              <button
                className={`sidebar-nav-item${item.active ? " active" : ""}`}
                type="button"
              >
                <span className="sidebar-nav-icon">{item.icon}</span>
                <span className="sidebar-nav-label">{item.label}</span>
              </button>
            </li>
          ))}
        </ul>
      </nav>

      <div className="sidebar-footer">
        <span className={`sidebar-mode-badge ${runMode}`}>
          {runMode === "desktop" ? "Desktop" : "Web"}
        </span>
        <span className="sidebar-version">v0.1.0</span>
      </div>
    </aside>
  );
}
