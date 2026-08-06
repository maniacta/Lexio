import { useState, useCallback } from "react";
import type { QuizQuestion, QuizResult } from "../types";
import { api } from "../api/client";
import { formatApiError } from "../utils/errors";
import { notifyDataChanged } from "../utils/events";

export function useQuiz(kpId: string | null) {
  const [questions, setQuestions] = useState<QuizQuestion[]>([]);
  const [currentIndex, setCurrentIndex] = useState(0);
  const [result, setResult] = useState<QuizResult | null>(null);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const loadQuestions = useCallback(async () => {
    if (!kpId) return;
    setLoading(true);
    setError(null);
    try {
      let qs = await api.quiz.getByKp(kpId);
      if (qs.length === 0) {
        qs = await api.ai.generateQuiz(kpId, 3);
      }
      setQuestions(qs);
      setCurrentIndex(0);
      setResult(null);
    } catch (err) {
      setError(formatApiError(err));
    } finally {
      setLoading(false);
    }
  }, [kpId]);

  const submitAnswer = useCallback(async (answer: string) => {
    const q = questions[currentIndex];
    if (!q) return;
    setLoading(true);
    setError(null);
    try {
      const res = await api.quiz.submit(q.id, answer);
      setResult(res);
      await api.ai.updateMastery(q.kp_id, res.is_correct);
      notifyDataChanged();
    } catch (err) {
      setError(formatApiError(err));
    } finally {
      setLoading(false);
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
