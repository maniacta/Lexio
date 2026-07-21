import { useSources } from "../../hooks/useSources";
import "./SourceList.css";

export default function SourceList() {
  const { sources, loading, toggleHidden } = useSources();
  const visible = sources.filter((s) => !s.hidden);

  return (
    <div className="source-list">
      <h3>资料来源 ({visible.length})</h3>
      {loading && <p className="list-loading">加载中...</p>}
      {visible.map((s) => (
        <div key={s.id} className="source-item">
          <div className="source-item-header">
            <span className={`source-origin ${s.origin}`}>
              {s.origin === "ai_search" ? "🔍" : "📎"}
            </span>
            <span className="source-title">{s.title}</span>
          </div>
          <div className="source-item-actions">
            <button onClick={() => toggleHidden(s.id, true)} title="暂时不看">
              👁️
            </button>
          </div>
        </div>
      ))}
      {!loading && visible.length === 0 && (
        <p className="list-empty">暂无资料来源</p>
      )}
    </div>
  );
}
