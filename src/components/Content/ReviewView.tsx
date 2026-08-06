import { useState, useEffect, useCallback, useRef } from "react";
import type { ReviewItem, ReviewResult } from "../../types";
import { api } from "../../api/client";
import { isAbortError } from "../../utils/errors";
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
  const abortRef = useRef<AbortController | null>(null);

  const loadItems = useCallback(async () => {
    abortRef.current?.abort();
    const ac = new AbortController();
    abortRef.current = ac;
    setLoading(true);
    setError(null);
    try {
      const data = await api.learning.getDueReviewsWithKp(ac.signal);
      if (!ac.signal.aborted) setItems(data);
    } catch (e: unknown) {
      if (!isAbortError(e)) {
        setError(e instanceof Error ? e.message : String(e));
      }
    } finally {
      if (!ac.signal.aborted) setLoading(false);
    }
  }, []);

  useEffect(() => {
    loadItems();
    return () => abortRef.current?.abort();
  }, [loadItems]);

  const handleStart = (ids: string[]) => {
    setSelectedIds(ids);
    setPhase("session");
  };

  const handleComplete = useCallback((sessionResults: ReviewResult[]) => {
    setResults(sessionResults);
    setPhase("summary");
  }, []);

  const handleBack = () => {
    setPhase("list");
    loadItems();
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
