import { useState, useCallback, useRef, useEffect } from "react";
import type { QuizQuestion, QuizResult } from "../types";
import { api } from "../api/client";
import { formatApiError, isAbortError } from "../utils/errors";
import { notifyDataChanged } from "../utils/events";

export function useQuiz(kpId: string | null) {
  const [questions, setQuestions] = useState<QuizQuestion[]>([]);
  const [currentIndex, setCurrentIndex] = useState(0);
  const [result, setResult] = useState<QuizResult | null>(null);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);
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
    if (currentIndex < questions.length - 1) {
      setCurrentIndex((i) => i + 1);
      setResult(null);
    }
  }, [currentIndex, questions.length]);

  const currentQuestion = questions[currentIndex] || null;
  const isFinished = currentIndex >= questions.length - 1 && result !== null;

  return {
    questions,
    currentQuestion,
    result,
    loading,
    error,
    isFinished,
    loadQuestions,
    submitAnswer,
    nextQuestion,
  };
}
