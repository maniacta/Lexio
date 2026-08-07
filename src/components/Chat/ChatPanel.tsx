import { useEffect, useRef, useState } from "react";
import { useChat } from "../../hooks/useChat";
import MessageBubble from "./MessageBubble";
import ChatInput from "./ChatInput";
import { api } from "../../api/client";
import { needsApiKeySetup } from "../../utils/errors";
import type { ChatAction } from "../../types";
import "./ChatPanel.css";

const EXAMPLE_TOPICS = ["Rust 所有权", "HTTP 缓存", "概率论基础"];

interface Props {
  onOpenSettings?: () => void;
  onNavigate: (action: ChatAction) => void;
}

export default function ChatPanel({ onOpenSettings, onNavigate }: Props) {
  const {
    messages,
    loading,
    sessions,
    activeSessionId,
    sendMessage,
    newSession,
    switchSession,
    deleteSession,
  } = useChat();
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
      <div className="chat-sidebar">
        <div className="chat-sidebar-header">
          <span>会话</span>
          <button
            type="button"
            className="chat-new-btn"
            onClick={() => void newSession()}
            title="新对话"
          >
            ＋
          </button>
        </div>
        <ul className="chat-session-list">
          {sessions.map((s) => (
            <li
              key={s.id}
              className={`chat-session-item ${s.id === activeSessionId ? "active" : ""}`}
              onClick={() => void switchSession(s.id)}
            >
              <span className="chat-session-title">{s.title || "新对话"}</span>
              <button
                type="button"
                className="chat-session-del"
                title="删除会话"
                onClick={(e) => {
                  e.stopPropagation();
                  void deleteSession(s.id);
                }}
              >
                ✕
              </button>
            </li>
          ))}
        </ul>
      </div>
      <div className="chat-main">
        <div className="chat-header">
          <h2>Lexio</h2>
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
            <MessageBubble key={i} message={msg} onAction={onNavigate} />
          ))}
          {loading && <div className="chat-loading">正在处理，稍候…</div>}
          <div ref={bottomRef} />
        </div>
        <ChatInput onSend={sendMessage} disabled={loading} />
      </div>
    </div>
  );
}
