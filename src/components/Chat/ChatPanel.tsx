import { useEffect, useRef } from "react";
import { useChat } from "../../hooks/useChat";
import MessageBubble from "./MessageBubble";
import ChatInput from "./ChatInput";
import "./ChatPanel.css";

export default function ChatPanel() {
  const { messages, loading, sendMessage } = useChat();
  const bottomRef = useRef<HTMLDivElement>(null);

  useEffect(() => {
    bottomRef.current?.scrollIntoView({ behavior: "smooth" });
  }, [messages]);

  return (
    <div className="chat-panel">
      <div className="chat-header">
        <h2>Lexio 学习教练</h2>
      </div>
      <div className="chat-messages">
        {messages.length === 0 && (
          <div className="chat-empty">
            <h3>你想学什么？</h3>
            <p>告诉我一个主题，我帮你搜集资料、整理知识点，用测验帮你掌握它。</p>
          </div>
        )}
        {messages.map((msg, i) => (
          <MessageBubble key={i} message={msg} />
        ))}
        {loading && <div className="chat-loading">思考中...</div>}
        <div ref={bottomRef} />
      </div>
      <ChatInput onSend={sendMessage} disabled={loading} />
    </div>
  );
}
