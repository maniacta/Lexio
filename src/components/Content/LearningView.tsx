import { useEffect, useState } from "react";
import ReactMarkdown from "react-markdown";
import { api } from "../../api/client";
import type { KnowledgePoint } from "../../types";
import { useQuiz } from "../../hooks/useQuiz";
import QuizCard from "./QuizCard";
import "./LearningView.css";

interface Props {
  kpId: string | null;
}

export default function LearningView({ kpId }: Props) {
  const [kp, setKp] = useState<KnowledgePoint | null>(null);
  const [showQuiz, setShowQuiz] = useState(false);
  const quiz = useQuiz(kpId);

  useEffect(() => {
    if (!kpId) return;
    api.knowledge.get(kpId).then(setKp).catch(console.error);
    setShowQuiz(false);
  }, [kpId]);

  if (!kpId || !kp) {
    return (
      <div className="learning-empty">
        <p>选择一个知识点开始学习</p>
      </div>
    );
  }

  return (
    <div className="learning-view">
      <div className="learning-header">
        <h2>{kp.title}</h2>
        <div className="learning-tags">
          {kp.tags.map((t) => (
            <span key={t} className="tag">{t}</span>
          ))}
        </div>
      </div>

      <div className="learning-content">
        <ReactMarkdown>{kp.content}</ReactMarkdown>
      </div>

      <div className="learning-actions">
        {!showQuiz ? (
          <button
            className="btn-start-quiz"
            onClick={() => { setShowQuiz(true); quiz.loadQuestions(); }}
          >
            开始测验
          </button>
        ) : (
          <div className="quiz-section">
            <h3>测验</h3>
            {quiz.loading && !quiz.currentQuestion && <p>加载题目中...</p>}
            {quiz.currentQuestion && (
              <QuizCard
                question={quiz.currentQuestion}
                result={quiz.result}
                loading={quiz.loading}
                onSubmit={quiz.submitAnswer}
                onNext={quiz.nextQuestion}
                isLast={quiz.isFinished}
              />
            )}
            {quiz.isFinished && (
              <p className="quiz-done">测验完成！</p>
            )}
          </div>
        )}
      </div>
    </div>
  );
}
