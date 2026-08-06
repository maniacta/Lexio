import { useState } from "react";
import Sidebar from "./Sidebar";
import Content from "./Content";
import "./Layout.css";

export type View = "chat" | "learning" | "settings" | "review";

export default function Layout() {
  const [selectedKpId, setSelectedKpId] = useState<string | null>(null);
  const [view, setView] = useState<View>("chat");

  const handleSelectKp = (id: string) => {
    setSelectedKpId(id);
    setView("learning");
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
        onOpenSettings={() => setView("settings")}
      />
    </div>
  );
}
