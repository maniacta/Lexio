import { useState } from "react";
import Sidebar from "./Sidebar";
import Content from "./Content";
import SourceViewer from "./Content/SourceViewer";
import type { ChatAction, Source } from "../types";
import { api } from "../api/client";
import "./Layout.css";

export type View = "chat" | "learning" | "settings" | "review";

export default function Layout() {
  const [selectedKpId, setSelectedKpId] = useState<string | null>(null);
  const [autoStartQuiz, setAutoStartQuiz] = useState<string | null>(null);
  const [view, setView] = useState<View>("chat");
  const [viewSource, setViewSource] = useState<Source | null>(null);

  const handleSelectKp = (id: string) => {
    setSelectedKpId(id);
    setAutoStartQuiz(null);
    setView("learning");
  };

  const handleChatNavigate = (action: ChatAction) => {
    if (action.type === "start_quiz") {
      // Jump to the KP and open the quiz immediately.
      if (action.payload.kpId) {
        setSelectedKpId(action.payload.kpId);
        setAutoStartQuiz(action.payload.kpId);
        setView("learning");
      }
    } else if (action.type === "navigate_learning") {
      if (action.payload.kpId) {
        setSelectedKpId(action.payload.kpId);
        setAutoStartQuiz(null);
        setView("learning");
      }
    } else if (action.type === "view_source" && action.payload.sourceId) {
      api.sources
        .get(action.payload.sourceId)
        .then(setViewSource)
        .catch((err) => console.error("Failed to load source:", err));
    }
  };

  return (
    <div className="layout">
      <Sidebar
        onSelectKp={handleSelectKp}
        selectedKpId={selectedKpId ?? undefined}
        currentView={view}
        onNavigate={setView}
      />
      <Content
        view={view}
        selectedKpId={selectedKpId}
        autoStartQuiz={autoStartQuiz}
        onOpenSettings={() => setView("settings")}
        onChatNavigate={handleChatNavigate}
      />
      {viewSource && <SourceViewer source={viewSource} onClose={() => setViewSource(null)} />}
    </div>
  );
}
