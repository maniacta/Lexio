import ReactMarkdown from "react-markdown";
import type { ChatMessage } from "../../types";
import "./MessageBubble.css";

interface Props {
  message: ChatMessage;
}

export default function MessageBubble({ message }: Props) {
  const isUser = message.role === "user";

  return (
    <div className={`message-bubble ${isUser ? "user" : "assistant"}`}>
      <div className="message-avatar">{isUser ? "👤" : "🤖"}</div>
      <div className="message-content">
        {isUser ? (
          <p>{message.content}</p>
        ) : (
          <ReactMarkdown>{message.content}</ReactMarkdown>
        )}
      </div>
    </div>
  );
}
