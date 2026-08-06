import ChatPanel from "./Chat/ChatPanel";
import LearningView from "./Content/LearningView";
import SettingsView from "./Content/SettingsView";
import ReviewView from "./Content/ReviewView";
import type { View } from "./Layout";
import "./Content.css";

interface Props {
  view: View;
  selectedKpId: string | null;
  onOpenSettings?: () => void;
}

export default function Content({ view, selectedKpId, onOpenSettings }: Props) {
  return (
    <main className="content">
      {view === "settings" ? <SettingsView /> :
       view === "learning" ? <LearningView kpId={selectedKpId} /> :
       view === "review" ? <ReviewView /> :
       <ChatPanel onOpenSettings={onOpenSettings} />}
    </main>
  );
}
