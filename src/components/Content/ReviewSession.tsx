import { useState, useEffect, useCallback, useRef } from "react";
import type { QuizQuestion, ReviewResult } from "../../types";
import { api } from "../../api/client";
import { notifyDataChanged } from "../../utils/events";
import { isAbortError } from "../../utils/errors";
import QuizCard from "./QuizCard";

interface Props {
  kpIds: string[];
  onComplete: (results: ReviewResult[]) => void;
}

export default function ReviewSession({ kpIds, onComplete }: Props) {
  const [index, setIndex] = useState(0);
  const [question, setQuestion] = useState<QuizQuestion | null>(null);
  const [result, setResult] = useState<{
    user_answer: string;
    is_correct: boolean;
    explanation: string;
    next_review_at: string;
  } | null>(null);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [kpTitle, setKpTitle] = useState("");
  const resultsRef = useRef<ReviewResult[]>([]);
  const usedQuestionIds = useRef<Set<string>>(new Set());
  const onCompleteRef = useRef(onComplete);
  onCompleteRef.current = onComplete;
  const kpIdsKey = kpIds.join(",");
  const completedRef = useRef(false);
  const abortRef = useRef<AbortController | null>(null);

  useEffect(() => {
    return () => abortRef.current?.abort();
  }, []);

  const fetchQuestion = useCallback(async (kpId: string) => {
    abortRef.current?.abort();
    const ac = new AbortController();
    abortRef.current = ac;

    setLoading(true);
    setError(null);
    setQuestion(null);
    setResult(null);
    try {
      const kp = await api.knowledge.get(kpId, ac.signal);
      if (ac.signal.aborted) return;
      setKpTitle(kp.title);

      const existingQuestions = await api.quiz.getByKp(kpId, ac.signal);
      if (ac.signal.aborted) return;
      const unused = existingQuestions.filter((q) => !usedQuestionIds.current.has(q.id));
      let q: QuizQuestion;

      if (unused.length > 0) {
        q = unused[Math.floor(Math.random() * unused.length)];
      } else {
        const generated = await api.ai.generateQuiz(kpId, 1, ac.signal);
        if (ac.signal.aborted) return;
        if (generated.length === 0) throw new Error("无法生成题目");
        q = generated[0];
      }

      usedQuestionIds.current.add(q.id);
      setQuestion(q);
    } catch (e: unknown) {
      if (isAbortError(e)) return;
      const message = e instanceof Error ? e.message : String(e);
      setError(message);
      setQuestion(null);
    } finally {
      if (!ac.signal.aborted) setLoading(false);
    }
  }, []);

  useEffect(() => {
    completedRef.current = false;
  }, [kpIdsKey]);

  useEffect(() => {
    if (index < kpIds.length) {
      fetchQuestion(kpIds[index]);
      return;
    }
    if (!completedRef.current) {
      completedRef.current = true;
      onCompleteRef.current(resultsRef.current);
    }
  }, [index, kpIdsKey, kpIds, fetchQuestion]);

  const submitAnswer = async (answer: string) => {
    if (!question) return;
    abortRef.current?.abort();
    const ac = new AbortController();
    abortRef.current = ac;

    setLoading(true);
    setError(null);
    try {
      const res = await api.quiz.submit(question.id, answer, ac.signal);
      if (ac.signal.aborted) return;
      const mastery = await api.ai.updateMastery(question.kp_id, res.is_correct, ac.signal);
      if (ac.signal.aborted) return;
      notifyDataChanged();
      setResult({
        user_answer: answer,
        is_correct: res.is_correct,
        explanation: res.explanation,
        next_review_at: mastery.next_review_at,
      });
    } catch (e: unknown) {
      if (isAbortError(e)) return;
      setError(e instanceof Error ? e.message : String(e));
    } finally {
      if (!ac.signal.aborted) setLoading(false);
    }
  };

  const nextQuestion = () => {
    if (question && result) {
      resultsRef.current.push({
        kp_id: question.kp_id,
        kp_title: kpTitle,
        is_correct: result.is_correct,
        next_review_at: result.next_review_at,
      });
    }
    setIndex((i) => i + 1);
  };

  const skipCurrent = () => {
    setError(null);
    setQuestion(null);
    setResult(null);
    setIndex((i) => i + 1);
  };

  const retryCurrent = () => {
    if (index < kpIds.length) {
      fetchQuestion(kpIds[index]);
    }
  };

  if (index >= kpIds.length) {
    return (
      <div className="review-session-loading">
        <p>完成中...</p>
      </div>
    );
  }

  if (loading && !question) {
    return (
      <div className="review-session-loading">
        <p>准备题目中...</p>
      </div>
    );
  }

  if (error && !question) {
    return (
      <div className="review-session-error">
        <p>出题失败：{error}</p>
        <div className="form-actions">
          <button type="button" className="btn-primary" onClick={retryCurrent}>
            重试
          </button>
          <button type="button" className="btn-secondary" onClick={skipCurrent}>
            跳过（不记错）
          </button>
        </div>
      </div>
    );
  }

  if (!question) {
    return (
      <div className="review-session-loading">
        <p>加载中...</p>
      </div>
    );
  }

  return (
    <div className="review-session">
      <div className="review-session-header">
        <span className="review-progress">
          第 {index + 1}/{kpIds.length} 题 — {kpTitle}
        </span>
      </div>

      {error && <p className="review-session-inline-error">{error}</p>}

      <QuizCard
        question={question}
        result={
          result
            ? {
                question,
                user_answer: result.user_answer,
                is_correct: result.is_correct,
                explanation: result.explanation,
              }
            : null
        }
        loading={loading}
        onSubmit={submitAnswer}
        onNext={nextQuestion}
        isLast={index === kpIds.length - 1}
      />
    </div>
  );
}
