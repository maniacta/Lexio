import ReactMarkdown from "react-markdown";
import rehypeSanitize from "rehype-sanitize";
import type { ChatMessage, ChatAction } from "../../types";
import "./MessageBubble.css";

interface Props {
  message: ChatMessage;
  onAction?: (action: ChatAction) => void;
}

export default function MessageBubble({ message, onAction }: Props) {
  const isUser = message.role === "user";

  return (
    <div className={`message-bubble ${isUser ? "user" : "assistant"}`}>
      <div className="message-avatar" aria-hidden="true">{isUser ? "你" : "L"}</div>
      <div className="message-content">
        {isUser ? (
          <p>{message.content}</p>
        ) : (
          <ReactMarkdown rehypePlugins={[rehypeSanitize]}>{message.content}</ReactMarkdown>
        )}
        {message.actions && message.actions.length > 0 && (
          <div className="message-actions">
            {message.actions.map((a, i) => (
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
