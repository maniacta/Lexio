import { useState } from "react";
import type { QuizQuestion, QuizResult } from "../../types";
import "./QuizCard.css";

interface Props {
  question: QuizQuestion;
  result: QuizResult | null;
  loading: boolean;
  onSubmit: (answer: string) => void;
  onNext: () => void;
  isLast: boolean;
}

export default function QuizCard({ question, result, loading, onSubmit, onNext, isLast }: Props) {
  const [selected, setSelected] = useState("");
  const [textAnswer, setTextAnswer] = useState("");

  const handleSubmit = () => {
    const answer = question.type === "multiple_choice" ? selected : textAnswer;
    if (answer.trim()) onSubmit(answer.trim());
  };

  return (
    <div className="quiz-card">
      <div className="quiz-question">
        <span className="quiz-type-badge">
          {question.type === "multiple_choice" ? "选择题" : "填空题"}
        </span>
        <p>{question.question}</p>
      </div>

      {!result && (
        <div className="quiz-answer-area">
          {question.type === "multiple_choice" && question.options ? (
            <div className="quiz-options">
              {question.options.map((opt, i) => (
                <label
                  key={i}
                  className={`quiz-option ${selected === opt ? "selected" : ""}`}
                >
                  <input
                    type="radio"
                    name="quiz-answer"
                    value={opt}
                    checked={selected === opt}
                    onChange={() => setSelected(opt)}
                  />
                  {opt}
                </label>
              ))}
            </div>
          ) : (
            <input
              type="text"
              className="quiz-text-input"
              value={textAnswer}
              onChange={(e) => setTextAnswer(e.target.value)}
              placeholder="输入你的答案..."
            />
          )}
          <button
            className="quiz-submit-btn"
            onClick={handleSubmit}
            disabled={loading || (!selected && !textAnswer)}
          >
            {loading ? "提交中..." : "提交"}
          </button>
        </div>
      )}

      {result && (
        <div className={`quiz-result ${result.is_correct ? "correct" : "incorrect"}`}>
          <div className="quiz-result-header">
            {result.is_correct ? "✅ 正确！" : "❌ 不正确"}
          </div>
          <div className="quiz-result-explanation">{result.explanation}</div>
          {!isLast && (
            <button className="quiz-next-btn" onClick={onNext}>
              下一题
            </button>
          )}
        </div>
      )}
    </div>
  );
}
