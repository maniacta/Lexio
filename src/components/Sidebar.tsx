import { useState, useEffect } from "react";
import SourceList from "./Sidebar/SourceList";
import KpList from "./Sidebar/KpList";
import { usePlatform, getRunMode } from "../utils/tauri";
import { api } from "../api/client";
import type { View } from "./Layout";
import "./Sidebar.css";

interface Props {
  onSelectKp: (id: string) => void;
  selectedKpId?: string;
  currentView: View;
  onNavigate: (view: View) => void;
}

export default function Sidebar({ onSelectKp, selectedKpId, currentView, onNavigate }: Props) {
  const platform = usePlatform();
  const runMode = getRunMode();
  const [tab, setTab] = useState<"sources" | "knowledge">("knowledge");
  const [dueCount, setDueCount] = useState<number>(0);

  useEffect(() => {
    api.learning.getDueReviews().then(records => setDueCount(records.length)).catch(() => {});
  }, [currentView]);

  return (
    <aside className="sidebar">
      <button
        type="button"
        className={`sidebar-chat-btn ${currentView === "chat" ? "active" : ""}`}
        onClick={() => onNavigate("chat")}
      >
        聊天
      </button>
      <div className="sidebar-tabs">
        <button
          className={`sidebar-tab ${tab === "knowledge" ? "active" : ""}`}
          onClick={() => setTab("knowledge")}
        >
          知识点
        </button>
        <button
          className={`sidebar-tab ${tab === "sources" ? "active" : ""}`}
          onClick={() => setTab("sources")}
        >
          资料
        </button>
      </div>
      {tab === "knowledge" && <KpList onSelect={onSelectKp} selectedId={selectedKpId} />}
      {tab === "sources" && <SourceList />}
      <button
        className={`sidebar-tab review ${currentView === "review" ? "active" : ""}`}
        onClick={() => onNavigate("review")}
      >
        复习
        {dueCount > 0 && <span className="review-badge">{dueCount}</span>}
      </button>
      <div className="sidebar-footer">
        <button
          className={`sidebar-settings-btn ${currentView === "settings" ? "active" : ""}`}
          onClick={() => onNavigate("settings")}
          title="设置"
        >
          ⚙
        </button>
        <span className={`sidebar-mode-badge ${runMode}`}>
          {platform}
        </span>
        <span className="sidebar-version">v0.1.0</span>
      </div>
    </aside>
  );
}
