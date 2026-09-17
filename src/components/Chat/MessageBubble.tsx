import ReactMarkdown from "react-markdown";
import type { ChatMessage, ChatAction } from "../../types";
import { markdownSanitize } from "../../utils/markdownSanitize";
import "./MessageBubble.css";

interface Props {
  message: ChatMessage;
  onAction?: (action: ChatAction) => void;
}

export default function MessageBubble({ message, onAction }: Props) {
  const isUser = message.role === "user";
  // Defensive: the API layer normalizes this, but never call `.map()` on a
  // non-array — a single bad payload must not unmount the render tree.
  const actions = Array.isArray(message.actions) ? message.actions : [];

  return (
    <div className={`message-bubble ${isUser ? "user" : "assistant"}`}>
      <div className="message-avatar" aria-hidden="true">{isUser ? "你" : "L"}</div>
      <div className="message-content">
        {isUser ? (
          <p>{message.content}</p>
        ) : (
          <ReactMarkdown rehypePlugins={[markdownSanitize]}>{message.content}</ReactMarkdown>
        )}
        {actions.length > 0 && (
          <div className="message-actions">
            {actions.map((a, i) => (
              <button key={i} className="btn-action" onClick={() => onAction?.(a)}>
                {a.label}
              </button>
            ))}
          </div>
        )}
      </div>
    </div>
  );
}
