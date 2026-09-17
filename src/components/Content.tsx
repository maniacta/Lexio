import ChatPanel from "./Chat/ChatPanel";
import LearningView from "./Content/LearningView";
import SettingsView from "./Content/SettingsView";
import ReviewView from "./Content/ReviewView";
import type { View } from "./Layout";
import type { ChatAction } from "../types";
import "./Content.css";

interface Props {
  view: View;
  selectedKpId: string | null;
  autoStartQuiz?: string | null;
  onOpenSettings?: () => void;
  onChatNavigate: (action: ChatAction) => void;
}

export default function Content({
  view,
  selectedKpId,
  autoStartQuiz,
  onOpenSettings,
  onChatNavigate,
}: Props) {
  // Keep ChatPanel mounted so conversation + plan state survive view switches.
  return (
    <main className="content">
      <div className="content-pane" hidden={view !== "chat"}>
        <ChatPanel onOpenSettings={onOpenSettings} onNavigate={onChatNavigate} />
      </div>
      {view === "settings" && <SettingsView />}
      {view === "learning" && (
        <LearningView kpId={selectedKpId} autoStartQuiz={autoStartQuiz} />
      )}
      {view === "review" && <ReviewView />}
    </main>
  );
}
