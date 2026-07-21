import { useState, useCallback } from "react";
import type { ChatMessage, AiResearchResult, LearningPlan } from "../types";
import { api } from "../api/client";

export function useChat() {
  const [messages, setMessages] = useState<ChatMessage[]>([]);
  const [loading, setLoading] = useState(false);
  const [currentPlan, setCurrentPlan] = useState<LearningPlan | null>(null);

  const sendMessage = useCallback(async (content: string) => {
    const userMsg: ChatMessage = { role: "user", content };
    setMessages((prev) => [...prev, userMsg]);
    setLoading(true);

    try {
      // Check if this looks like a new topic request
      const topicTriggers = ["想学", "学习", "教我", "了解", "learn", "study"];
      const isLearnRequest = topicTriggers.some((t) => content.includes(t));

      if (isLearnRequest && !currentPlan) {
        // Extract topic (simple heuristic: take the content as topic)
        const topic = content;
        const result: AiResearchResult = await api.ai.startResearch(topic);

        const botMsg: ChatMessage = {
          role: "assistant",
          content: `我找到了 ${result.sources.length} 份资料，提取了 ${result.knowledge_points.length} 个知识点。

**学习计划：${result.plan.title}**

知识点列表：
${result.knowledge_points.map((kp, i) => `${i + 1}. ${kp.title} — ${kp.summary}`).join("\n")}

准备好学习第一个知识点了吗？`,
        };
        setMessages((prev) => [...prev, botMsg]);
        setCurrentPlan(result.plan);
      } else {
        // General chat (stub for now)
        const botMsg: ChatMessage = {
          role: "assistant",
          content: "收到。你可以告诉我你想学什么，或者粘贴文章链接/内容给我。",
        };
        setMessages((prev) => [...prev, botMsg]);
      }
    } catch (err) {
      const errMsg: ChatMessage = {
        role: "assistant",
        content: `出错了：${err instanceof Error ? err.message : "未知错误"}`,
      };
      setMessages((prev) => [...prev, errMsg]);
    } finally {
      setLoading(false);
    }
  }, [currentPlan]);

  return { messages, loading, currentPlan, sendMessage };
}
