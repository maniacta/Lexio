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
      <h3 id="kp-list-heading">知识点 ({kps.length})</h3>
      {loading && <p className="list-loading">加载中...</p>}
      {kps.length > 0 && (
        <ul className="sidebar-item-list" aria-labelledby="kp-list-heading">
          {kps.map((kp) => {
            const selected = selectedId === kp.id;
            return (
              <li key={kp.id}>
                <button
                  type="button"
                  className={`kp-item ${selected ? "selected" : ""}`}
                  aria-current={selected ? "true" : undefined}
                  onClick={() => onSelect(kp.id)}
                >
                  <span className="kp-title">{kp.title}</span>
                  <span className="kp-summary">{kp.summary}</span>
                </button>
              </li>
            );
          })}
        </ul>
      )}
      {!loading && kps.length === 0 && (
        <p className="list-empty">暂无知识点，去聊天里开一个主题吧</p>
      )}
    </div>
  );
}
