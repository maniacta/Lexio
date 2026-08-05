import { useState, useEffect } from "react";
import type { ReviewItem, ReviewResult } from "../../types";
import { api } from "../../api/client";
import ReviewList from "./ReviewList";
import ReviewSession from "./ReviewSession";
import ReviewSummary from "./ReviewSummary";
import "./ReviewView.css";

type Phase = "list" | "session" | "summary";

export default function ReviewView() {
  const [phase, setPhase] = useState<Phase>("list");
  const [items, setItems] = useState<ReviewItem[]>([]);
  const [selectedIds, setSelectedIds] = useState<string[]>([]);
  const [results, setResults] = useState<ReviewResult[]>([]);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);

  const loadItems = async () => {
    setLoading(true);
    setError(null);
    try {
      const data = await api.learning.getDueReviewsWithKp();
      setItems(data);
    } catch (e: any) {
      setError(e.message);
    } finally {
      setLoading(false);
    }
  };

  useEffect(() => {
    loadItems();
  }, []);

  const handleStart = (ids: string[]) => {
    setSelectedIds(ids);
    setPhase("session");
  };

  const handleComplete = (sessionResults: ReviewResult[]) => {
    setResults(sessionResults);
    setPhase("summary");
  };

  const handleBack = () => {
    setPhase("list");
    loadItems(); // refresh due list
  };

  if (loading) {
    return (
      <div className="review-view">
        <div className="review-loading">加载中...</div>
      </div>
    );
  }

  if (error) {
    return (
      <div className="review-view">
        <div className="review-error">
          <p>加载失败：{error}</p>
          <button className="btn-primary" onClick={loadItems}>重试</button>
        </div>
      </div>
    );
  }

  return (
    <div className="review-view">
      {phase === "list" && (
        <ReviewList items={items} onStart={handleStart} />
      )}
      {phase === "session" && (
        <ReviewSession kpIds={selectedIds} onComplete={handleComplete} />
      )}
      {phase === "summary" && (
        <ReviewSummary results={results} onBack={handleBack} />
      )}
    </div>
  );
}
