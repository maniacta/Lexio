import { useState } from "react";
import Sidebar from "./Sidebar";
import Content from "./Content";
import "./Layout.css";

export default function Layout() {
  const [selectedKpId, setSelectedKpId] = useState<string | null>(null);
  const [view, setView] = useState<"chat" | "learning">("chat");

  const handleSelectKp = (id: string) => {
    setSelectedKpId(id);
    setView("learning");
  };

  return (
    <div className="layout">
      <Sidebar onSelectKp={handleSelectKp} selectedKpId={selectedKpId ?? undefined} />
      <Content view={view} selectedKpId={selectedKpId} />
    </div>
  );
}
