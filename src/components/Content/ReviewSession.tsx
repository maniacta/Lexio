import { useState, useEffect, useCallback, useRef } from "react";
import type { QuizQuestion, ReviewResult } from "../../types";
import { api } from "../../api/client";
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

  const fetchQuestion = useCallback(async (kpId: string) => {
    setLoading(true);
    setError(null);
    try {
      // Get KP title for display
      const kp = await api.knowledge.get(kpId);
      setKpTitle(kp.title);

      // Try to find an unused question from existing bank
      const existingQuestions = await api.quiz.getByKp(kpId);
      const unused = existingQuestions.filter((q) => !usedQuestionIds.current.has(q.id));
      let q: QuizQuestion;

      if (unused.length > 0) {
        q = unused[Math.floor(Math.random() * unused.length)];
      } else {
        // Generate new questions via AI
        const generated = await api.ai.generateQuiz(kpId, 1);
        if (generated.length === 0) throw new Error("无法生成题目");
        q = generated[0];
      }

      usedQuestionIds.current.add(q.id);
      setQuestion(q);
      setResult(null);
    } catch (e: any) {
      // If can't generate, skip this KP
      const reviewResult: ReviewResult = {
        kp_id: kpId,
        kp_title: kpTitle || "(加载失败)",
        is_correct: false,
        next_review_at: "",
      };
      resultsRef.current.push(reviewResult);
      setError(e.message);
      setQuestion(null);
      setIndex((i) => i + 1);
    } finally {
      setLoading(false);
    }
  }, []);

  useEffect(() => {
    if (index < kpIds.length) {
      fetchQuestion(kpIds[index]);
    } else {
      onComplete(resultsRef.current);
    }
  }, [index, kpIds, fetchQuestion, onComplete]);

  const submitAnswer = async (answer: string) => {
    if (!question) return;
    setLoading(true);
    try {
      const res = await api.quiz.submit(question.id, answer);
      const mastery = await api.ai.updateMastery(question.kp_id, res.is_correct);
      setResult({
        user_answer: answer,
        is_correct: res.is_correct,
        explanation: res.explanation,
        next_review_at: mastery.next_review_at,
      });
    } catch (e: any) {
      setError(e.message);
    } finally {
      setLoading(false);
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
        <p>⚠️ 出题失败：{error}</p>
        <button className="btn-primary" onClick={nextQuestion}>
          跳过，继续下一个
        </button>
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
