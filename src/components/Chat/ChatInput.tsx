import { useState, KeyboardEvent } from "react";
import "./ChatInput.css";

interface Props {
  onSend: (msg: string) => void;
  disabled: boolean;
}

export default function ChatInput({ onSend, disabled }: Props) {
  const [input, setInput] = useState("");

  const handleSend = () => {
    const trimmed = input.trim();
    if (trimmed && !disabled) {
      onSend(trimmed);
      setInput("");
    }
  };

  const handleKeyDown = (e: KeyboardEvent) => {
    if (e.key === "Enter" && !e.shiftKey) {
      e.preventDefault();
      handleSend();
    }
  };

  return (
    <div className="chat-input">
      <input
        type="text"
        value={input}
        onChange={(e) => setInput(e.target.value)}
        onKeyDown={handleKeyDown}
        placeholder={disabled ? "处理中..." : "输入你想学的内容，或者粘贴资料..."}
        disabled={disabled}
      />
      <button onClick={handleSend} disabled={disabled || !input.trim()}>
        发送
      </button>
    </div>
  );
}
