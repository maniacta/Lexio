import { useState, useCallback, useRef, useEffect } from "react";
import type { QuizQuestion, QuizResult } from "../types";
import { api } from "../api/client";
import { formatApiError, isAbortError } from "../utils/errors";
import { notifyDataChanged } from "../utils/events";

/** Advance to the next index, or signal that the last card should complete. */
export function quizStep(currentIndex: number, questionCount: number): number | "complete" {
  if (questionCount <= 0 || currentIndex >= questionCount - 1) return "complete";
  return currentIndex + 1;
}

export function useQuiz(kpId: string | null) {
  const [questions, setQuestions] = useState<QuizQuestion[]>([]);
  const [currentIndex, setCurrentIndex] = useState(0);
  const [result, setResult] = useState<QuizResult | null>(null);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [finished, setFinished] = useState(false);
  const abortRef = useRef<AbortController | null>(null);

  useEffect(() => {
    return () => {
      abortRef.current?.abort();
    };
  }, [kpId]);

  const loadQuestions = useCallback(async () => {
    if (!kpId) return;
    abortRef.current?.abort();
    const ac = new AbortController();
    abortRef.current = ac;

    setLoading(true);
    setError(null);
    try {
      let qs = await api.quiz.getByKp(kpId, ac.signal);
      if (qs.length === 0) {
        qs = await api.ai.generateQuiz(kpId, 3, ac.signal);
      }
      if (ac.signal.aborted) return;
      setQuestions(qs);
      setCurrentIndex(0);
      setResult(null);
      setFinished(false);
    } catch (err) {
      if (isAbortError(err)) return;
      setError(formatApiError(err));
    } finally {
      if (!ac.signal.aborted) setLoading(false);
    }
  }, [kpId]);

  const submitAnswer = useCallback(async (answer: string) => {
    const q = questions[currentIndex];
    if (!q) return;
    abortRef.current?.abort();
    const ac = new AbortController();
    abortRef.current = ac;

    setLoading(true);
    setError(null);
    try {
      const res = await api.quiz.submit(q.id, answer, ac.signal);
      if (ac.signal.aborted) return;
      setResult(res);
      await api.ai.updateMastery(q.kp_id, res.is_correct, ac.signal);
      if (!ac.signal.aborted) notifyDataChanged();
    } catch (err) {
      if (isAbortError(err)) return;
      setError(formatApiError(err));
    } finally {
      if (!ac.signal.aborted) setLoading(false);
    }
  }, [questions, currentIndex]);

  const nextQuestion = useCallback(() => {
    const step = quizStep(currentIndex, questions.length);
    if (step === "complete") return;
    setCurrentIndex(step);
    setResult(null);
  }, [currentIndex, questions.length]);

  const finishQuiz = useCallback(() => {
    setFinished(true);
    setResult(null);
  }, []);

  const currentQuestion = questions[currentIndex] || null;
  const isLast = questions.length > 0 && currentIndex === questions.length - 1;
  const isFinished = finished;

  return {
    questions,
    currentQuestion,
    result,
    loading,
    error,
    isLast,
    isFinished,
    loadQuestions,
    submitAnswer,
    nextQuestion,
    finishQuiz,
  };
}
