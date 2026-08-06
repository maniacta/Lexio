import { useState, useEffect, useCallback } from "react";
import SourceList from "./Sidebar/SourceList";
import KpList from "./Sidebar/KpList";
import { usePlatform, getRunMode } from "../utils/tauri";
import { api } from "../api/client";
import { DATA_CHANGED } from "../utils/events";
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

  const refreshDue = useCallback(() => {
    api.learning
      .getDueReviews()
      .then((records) => setDueCount(records.length))
      .catch(() => {});
  }, []);

  useEffect(() => {
    refreshDue();
  }, [currentView, refreshDue]);

  useEffect(() => {
    window.addEventListener(DATA_CHANGED, refreshDue);
    return () => window.removeEventListener(DATA_CHANGED, refreshDue);
  }, [refreshDue]);

  return (
    <aside className="sidebar">
      <div className="sidebar-brand">
        <h1 className="sidebar-brand-title">Lexio</h1>
        <p className="sidebar-brand-sub">学习教练</p>
      </div>

      <button
        type="button"
        className={`sidebar-chat-btn ${currentView === "chat" ? "active" : ""}`}
        onClick={() => onNavigate("chat")}
      >
        聊天
      </button>

      <div className="sidebar-tabs">
        <button
          type="button"
          className={`sidebar-tab ${tab === "knowledge" ? "active" : ""}`}
          onClick={() => setTab("knowledge")}
        >
          知识点
        </button>
        <button
          type="button"
          className={`sidebar-tab ${tab === "sources" ? "active" : ""}`}
          onClick={() => setTab("sources")}
        >
          资料
        </button>
      </div>

      <div className="sidebar-scroll">
        {tab === "knowledge" && <KpList onSelect={onSelectKp} selectedId={selectedKpId} />}
        {tab === "sources" && <SourceList />}
      </div>

      <button
        type="button"
        className={`sidebar-review-btn ${currentView === "review" ? "active" : ""}`}
        onClick={() => onNavigate("review")}
      >
        <span>复习</span>
        {dueCount > 0 && <span className="review-badge">{dueCount}</span>}
      </button>

      <div className="sidebar-footer">
        <button
          type="button"
          className={`sidebar-settings-btn ${currentView === "settings" ? "active" : ""}`}
          onClick={() => onNavigate("settings")}
          title="设置"
          aria-label="设置"
        >
          ⚙
        </button>
        <span className={`sidebar-mode-badge ${runMode}`}>{platform}</span>
        <span className="sidebar-version">v0.1.0</span>
      </div>
    </aside>
  );
}
