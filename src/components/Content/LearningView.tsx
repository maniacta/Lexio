import { useEffect, useState } from "react";
import ReactMarkdown from "react-markdown";
import rehypeSanitize from "rehype-sanitize";
import { api } from "../../api/client";
import type { KnowledgePoint, Relation } from "../../types";
import { useQuiz } from "../../hooks/useQuiz";
import { formatApiError, isAbortError } from "../../utils/errors";
import QuizCard from "./QuizCard";
import "./LearningView.css";

interface Props {
  kpId: string | null;
  autoStartQuiz?: string | null;
}

const RELATION_LABEL: Record<string, string> = {
  prerequisite: "前置",
  related: "相关",
  extension: "延伸",
};

export default function LearningView({ kpId, autoStartQuiz }: Props) {
  const [kp, setKp] = useState<KnowledgePoint | null>(null);
  const [relations, setRelations] = useState<Relation[]>([]);
  const [relatedTitles, setRelatedTitles] = useState<Record<string, string>>({});
  const [loadError, setLoadError] = useState<string | null>(null);
  const [showQuiz, setShowQuiz] = useState(false);
  const quiz = useQuiz(kpId);

  useEffect(() => {
    if (!kpId) {
      setKp(null);
      setRelations([]);
      setRelatedTitles({});
      setLoadError(null);
      return;
    }
    setKp(null);
    setRelations([]);
    setRelatedTitles({});
    setLoadError(null);
    if (autoStartQuiz === kpId) {
      // Chat asked to start the quiz for this KP immediately.
      setShowQuiz(true);
      quiz.loadQuestions();
    } else {
      setShowQuiz(false);
    }

    const ac = new AbortController();
    (async () => {
      try {
        const data = await api.knowledge.get(kpId, ac.signal);
        if (ac.signal.aborted) return;
        setKp(data);

        const rels = await api.knowledge.listRelations(kpId, ac.signal);
        if (ac.signal.aborted) return;
        setRelations(rels);

        const otherIds = [
          ...new Set(
            rels.map((r) => (r.from_kp_id === kpId ? r.to_kp_id : r.from_kp_id))
          ),
        ];
        if (otherIds.length > 0) {
          const others = await api.knowledge.list(undefined, otherIds, ac.signal);
          if (ac.signal.aborted) return;
          const map: Record<string, string> = {};
          for (const o of others) map[o.id] = o.title;
          setRelatedTitles(map);
        }
      } catch (e: unknown) {
        if (!isAbortError(e) && !ac.signal.aborted) {
          setLoadError(formatApiError(e));
        }
      }
    })();

    return () => ac.abort();
  }, [kpId, autoStartQuiz]);

  if (!kpId) {
    return (
      <div className="learning-empty">
        <p>选择一个知识点开始学习</p>
      </div>
    );
  }

  if (loadError) {
    return (
      <div className="learning-empty">
        <p>加载失败：{loadError}</p>
      </div>
    );
  }

  if (!kp) {
    return (
      <div className="learning-empty">
        <p>加载中…</p>
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
        <ReactMarkdown rehypePlugins={[rehypeSanitize]}>{kp.content}</ReactMarkdown>
      </div>

      {relations.length > 0 && (
        <div className="learning-relations">
          <h3>关联知识点</h3>
          <ul>
            {relations.map((r) => {
              const otherId = r.from_kp_id === kpId ? r.to_kp_id : r.from_kp_id;
              const label = RELATION_LABEL[r.relation_type] ?? r.relation_type;
              return (
                <li key={r.id}>
                  <span className="relation-type">{label}</span>
                  {relatedTitles[otherId] ?? otherId}
                </li>
              );
            })}
          </ul>
        </div>
      )}

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
            {quiz.error && <p className="quiz-error">{quiz.error}</p>}
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
