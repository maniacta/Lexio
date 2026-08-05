import { useState } from "react";
import type { ReviewItem } from "../../types";

interface Props {
  items: ReviewItem[];
  onStart: (ids: string[]) => void;
}

export default function ReviewList({ items, onStart }: Props) {
  const [selected, setSelected] = useState<Set<string>>(
    () => new Set(items.map((i) => i.mastery.kp_id))
  );

  const toggle = (id: string) => {
    setSelected((prev) => {
      const next = new Set(prev);
      if (next.has(id)) next.delete(id);
      else next.add(id);
      return next;
    });
  };

  const toggleAll = () => {
    if (selected.size === items.length) {
      setSelected(new Set());
    } else {
      setSelected(new Set(items.map((i) => i.mastery.kp_id)));
    }
  };

  const formatTimeAgo = (dateStr: string | null): string => {
    if (!dateStr) return "首次复习";
    const diff = Date.now() - new Date(dateStr).getTime();
    const days = Math.floor(diff / 86400000);
    if (days === 0) return "今天";
    if (days === 1) return "1 天前";
    return `${days} 天前`;
  };

  if (items.length === 0) {
    return (
      <div className="review-empty">
        <h3>暂无待复习知识点 🎉</h3>
        <p>完成测验后，系统会根据你的掌握程度自动安排复习。</p>
      </div>
    );
  }

  return (
    <div className="review-list">
      <div className="review-list-header">
        <h2>📋 待复习 ({items.length})</h2>
        <button className="btn-link" onClick={toggleAll}>
          {selected.size === items.length ? "取消全选" : "全选"}
        </button>
      </div>

      <div className="review-items">
        {items.map((item) => (
          <label
            key={item.mastery.kp_id}
            className={`review-item-card ${selected.has(item.mastery.kp_id) ? "selected" : ""}`}
          >
            <input
              type="checkbox"
              checked={selected.has(item.mastery.kp_id)}
              onChange={() => toggle(item.mastery.kp_id)}
            />
            <div className="review-item-info">
              <span className="review-item-title">📌 {item.knowledge_point.title}</span>
              <span className="review-item-meta">
                上次复习：{formatTimeAgo(item.mastery.last_reviewed_at)}
                &nbsp;·&nbsp;间隔：{item.mastery.interval_days} 天
                &nbsp;·&nbsp;连续正确：{item.mastery.repetitions} 次
              </span>
            </div>
          </label>
        ))}
      </div>

      <button
        className="btn-primary btn-start-review"
        disabled={selected.size === 0}
        onClick={() => onStart(Array.from(selected))}
      >
        开始复习 ({selected.size})
      </button>
    </div>
  );
}
