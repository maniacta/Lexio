import ChatPanel from "./Chat/ChatPanel";
import LearningView from "./Content/LearningView";
import "./Content.css";

interface Props {
  view: "chat" | "learning";
  selectedKpId: string | null;
}

export default function Content({ view, selectedKpId }: Props) {
  return (
    <main className="content">
      {view === "chat" ? <ChatPanel /> : <LearningView kpId={selectedKpId} />}
    </main>
  );
}
