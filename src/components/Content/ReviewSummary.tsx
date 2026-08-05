import type { ReviewResult } from "../../types";

interface Props {
  results: ReviewResult[];
  onBack: () => void;
}

export default function ReviewSummary({ results, onBack }: Props) {
  const correctCount = results.filter((r) => r.is_correct).length;
  const totalCount = results.length;

  const formatNextReview = (dateStr: string): string => {
    if (!dateStr) return "未更新";
    try {
      const date = new Date(dateStr);
      const diffDays = Math.round(
        (date.getTime() - Date.now()) / 86400000
      );
      if (diffDays <= 0) return "今天";
      if (diffDays === 1) return "明天";
      return `${diffDays} 天后`;
    } catch {
      return dateStr;
    }
  };

  return (
    <div className="review-summary">
      <div className="review-summary-header">
        <h2>✅ 复习完成！</h2>
        <p className="review-summary-score">
          正确：{correctCount} / {totalCount}
        </p>
      </div>

      <div className="review-summary-items">
        {results.map((r, i) => (
          <div key={i} className={`review-summary-item ${r.is_correct ? "correct" : "incorrect"}`}>
            <span className="review-summary-icon">{r.is_correct ? "✅" : "❌"}</span>
            <span className="review-summary-title">{r.kp_title}</span>
            <span className="review-summary-next">
              下次复习：{formatNextReview(r.next_review_at)}
            </span>
          </div>
        ))}
      </div>

      <button className="btn-primary" onClick={onBack}>
        返回待复习列表
      </button>
    </div>
  );
}
