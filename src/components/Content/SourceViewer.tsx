import type { Source } from "../../types";
import "./SourceViewer.css";

interface Props {
  source: Source;
  onClose: () => void;
}

/** Minimal source-content viewer, opened from chat `view_source` actions. */
export default function SourceViewer({ source, onClose }: Props) {
  return (
    <div className="source-viewer-overlay" onClick={onClose}>
      <div className="source-viewer" onClick={(e) => e.stopPropagation()}>
        <div className="source-viewer-header">
          <h3>{source.title}</h3>
          <button className="source-viewer-close" onClick={onClose} title="关闭">
            ✕
          </button>
        </div>
        <div className="source-viewer-meta">
          <span>{source.origin === "ai_search" ? "🔍 AI 搜索" : "📎 用户添加"}</span>
          {source.source_url && (
            <a href={source.source_url} target="_blank" rel="noreferrer">
              {source.source_url}
            </a>
          )}
        </div>
        <pre className="source-viewer-content">{source.content}</pre>
      </div>
    </div>
  );
}
