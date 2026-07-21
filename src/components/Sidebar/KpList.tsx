import { useKnowledge } from "../../hooks/useKnowledge";
import "./KpList.css";

interface Props {
  onSelect: (id: string) => void;
  selectedId?: string;
}

export default function KpList({ onSelect, selectedId }: Props) {
  const { kps, loading } = useKnowledge();

  return (
    <div className="kp-list">
      <h3>知识点 ({kps.length})</h3>
      {loading && <p className="list-loading">加载中...</p>}
      {kps.map((kp) => (
        <div
          key={kp.id}
          className={`kp-item ${selectedId === kp.id ? "selected" : ""}`}
          onClick={() => onSelect(kp.id)}
        >
          <span className="kp-title">{kp.title}</span>
          <span className="kp-summary">{kp.summary}</span>
        </div>
      ))}
    </div>
  );
}
