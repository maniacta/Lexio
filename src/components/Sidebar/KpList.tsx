import { useKnowledge } from "../../hooks/useKnowledge";
import { useEffect } from "react";
import { DATA_CHANGED } from "../../utils/events";
import "./KpList.css";

interface Props {
  onSelect: (id: string) => void;
  selectedId?: string;
}

export default function KpList({ onSelect, selectedId }: Props) {
  const { kps, loading, refresh } = useKnowledge();

  useEffect(() => {
    const onChange = () => { refresh(); };
    window.addEventListener(DATA_CHANGED, onChange);
    return () => window.removeEventListener(DATA_CHANGED, onChange);
  }, [refresh]);

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
      {!loading && kps.length === 0 && (
        <p className="list-empty">暂无知识点，去聊天里开一个主题吧</p>
      )}
    </div>
  );
}
