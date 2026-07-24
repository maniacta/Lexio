import { useState, useEffect } from "react";
import type { SettingsData } from "../../types";
import { api } from "../../api/client";
import GeneralTab from "./GeneralTab";
import ProvidersTab from "./ProvidersTab";
import TaskModelsTab from "./TaskModelsTab";
import "./SettingsView.css";

type Tab = "general" | "providers" | "tasks";

export default function SettingsView() {
  const [tab, setTab] = useState<Tab>("general");
  const [settings, setSettings] = useState<SettingsData | null>(null);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);

  const loadSettings = async () => {
    setLoading(true);
    setError(null);
    try {
      const data = await api.settings.getAll();
      setSettings(data);
    } catch (e: any) {
      setError(e.message || "加载设置失败");
    } finally {
      setLoading(false);
    }
  };

  useEffect(() => { loadSettings(); }, []);

  if (loading) return <div className="settings-loading">加载中...</div>;
  if (error) return <div className="settings-error">{error}<button onClick={loadSettings}>重试</button></div>;
  if (!settings) return null;

  return (
    <div className="settings-view">
      <h2 className="settings-title">⚙ 设置</h2>
      <div className="settings-body">
        <nav className="settings-nav">
          {(["general", "providers", "tasks"] as Tab[]).map(t => (
            <button
              key={t}
              className={`settings-nav-btn ${tab === t ? "active" : ""}`}
              onClick={() => setTab(t)}
            >
              {{ general: "通用", providers: "模型厂商", tasks: "任务模型" }[t]}
            </button>
          ))}
        </nav>
        <div className="settings-content">
          {tab === "general" && <GeneralTab settings={settings} onSaved={loadSettings} />}
          {tab === "providers" && <ProvidersTab settings={settings} onSaved={loadSettings} />}
          {tab === "tasks" && <TaskModelsTab settings={settings} onSaved={loadSettings} />}
        </div>
      </div>
    </div>
  );
}
