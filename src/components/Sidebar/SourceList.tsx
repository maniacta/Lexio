import { useEffect } from "react";
import { useSources } from "../../hooks/useSources";
import { DATA_CHANGED } from "../../utils/events";
import type { Source } from "../../types";
import "./SourceList.css";

interface Props {
  onOpen?: (source: Source) => void;
}

export default function SourceList({ onOpen }: Props) {
  const { sources, loading, refresh, toggleHidden } = useSources();
  const visible = sources.filter((s) => !s.hidden);

  useEffect(() => {
    const onChange = () => { refresh(); };
    window.addEventListener(DATA_CHANGED, onChange);
    return () => window.removeEventListener(DATA_CHANGED, onChange);
  }, [refresh]);

  return (
    <div className="source-list">
      <h3 id="source-list-heading">资料来源 ({visible.length})</h3>
      {loading && <p className="list-loading">加载中...</p>}
      {visible.length > 0 && (
        <ul className="sidebar-item-list" aria-labelledby="source-list-heading">
          {visible.map((s) => (
            <li key={s.id} className="source-item">
              <button
                type="button"
                className="source-item-open"
                onClick={() => onOpen?.(s)}
              >
                <span className="source-item-header">
                  <span className={`source-origin ${s.origin}`}>
                    {s.origin === "ai_search" ? "AI 搜索" : "用户添加"}
                  </span>
                  <span className="source-title">{s.title}</span>
                </span>
              </button>
              <div className="source-item-actions">
                <button
                  type="button"
                  onClick={() => toggleHidden(s.id, true)}
                  aria-label={`隐藏「${s.title}」`}
                >
                  隐藏
                </button>
              </div>
            </li>
          ))}
        </ul>
      )}
      {!loading && visible.length === 0 && (
        <p className="list-empty">暂无资料来源</p>
      )}
    </div>
  );
}
