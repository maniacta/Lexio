import { useEffect, useRef, useState } from "react";
import { useChat } from "../../hooks/useChat";
import MessageBubble from "./MessageBubble";
import ChatInput from "./ChatInput";
import { api } from "../../api/client";
import { needsApiKeySetup } from "../../utils/errors";
import "./ChatPanel.css";

const EXAMPLE_TOPICS = ["Rust 所有权", "HTTP 缓存", "概率论基础"];

interface Props {
  onOpenSettings?: () => void;
}

export default function ChatPanel({ onOpenSettings }: Props) {
  const { messages, loading, sendMessage } = useChat();
  const bottomRef = useRef<HTMLDivElement>(null);
  const [needsKey, setNeedsKey] = useState(false);

  useEffect(() => {
    bottomRef.current?.scrollIntoView({ behavior: "smooth" });
  }, [messages]);

  useEffect(() => {
    api.settings
      .getAll()
      .then((data) => setNeedsKey(needsApiKeySetup(data.providers)))
      .catch(() => setNeedsKey(false));
  }, [messages.length]);

  return (
    <div className="chat-panel">
      <div className="chat-header">
        <h2>Lexio 学习教练</h2>
      </div>
      {needsKey && (
        <div className="chat-setup-banner">
          <div>
            <strong>先配置 API Key</strong>
            <p>到「设置 → 模型厂商」填写密钥后，即可开始学习。</p>
          </div>
          {onOpenSettings && (
            <button type="button" className="chat-setup-btn" onClick={onOpenSettings}>
              前往设置
            </button>
          )}
        </div>
      )}
      <div className="chat-messages">
        {messages.length === 0 && (
          <div className="chat-empty">
            <h3>你想学什么？</h3>
            <p>直接输入一个主题，我帮你整理知识点，并用测验帮你掌握它。</p>
            <div className="chat-examples">
              {EXAMPLE_TOPICS.map((topic) => (
                <button
                  key={topic}
                  type="button"
                  className="chat-example-chip"
                  disabled={loading || needsKey}
                  onClick={() => sendMessage(topic)}
                >
                  {topic}
                </button>
              ))}
            </div>
          </div>
        )}
        {messages.map((msg, i) => (
          <MessageBubble key={i} message={msg} />
        ))}
        {loading && <div className="chat-loading">正在研究主题，稍候…</div>}
        <div ref={bottomRef} />
      </div>
      <ChatInput onSend={sendMessage} disabled={loading} />
    </div>
  );
}
