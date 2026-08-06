import { useState, useCallback } from "react";
import type { ChatMessage, AiResearchResult, LearningPlan } from "../types";
import { api } from "../api/client";
import { formatApiError } from "../utils/errors";
import { notifyDataChanged } from "../utils/events";

export function useChat() {
  const [messages, setMessages] = useState<ChatMessage[]>([]);
  const [loading, setLoading] = useState(false);
  const [currentPlan, setCurrentPlan] = useState<LearningPlan | null>(null);

  const sendMessage = useCallback(async (content: string) => {
    const userMsg: ChatMessage = { role: "user", content };
    setMessages((prev) => [...prev, userMsg]);
    setLoading(true);

    try {
      // First message without an active plan → start research on any topic
      if (!currentPlan) {
        const topic = content.trim();
        const result: AiResearchResult = await api.ai.startResearch(topic);
        notifyDataChanged();

        const botMsg: ChatMessage = {
          role: "assistant",
          content: `我找到了 ${result.sources.length} 份资料，整理了 ${result.knowledge_points.length} 个知识点。

**学习计划：${result.plan.title}**

知识点列表：
${result.knowledge_points.map((kp, i) => `${i + 1}. ${kp.title} — ${kp.summary}`).join("\n")}

左侧「知识点」已更新，点任意一项即可开始学习与测验。`,
        };
        setMessages((prev) => [...prev, botMsg]);
        setCurrentPlan(result.plan);
      } else {
        const botMsg: ChatMessage = {
          role: "assistant",
          content:
            "当前已有学习计划。请在左侧选择知识点继续学习，或点击「复习」巩固已学内容。若要开新主题，可先完成当前计划后再来。",
        };
        setMessages((prev) => [...prev, botMsg]);
      }
    } catch (err) {
      const errMsg: ChatMessage = {
        role: "assistant",
        content: formatApiError(err),
      };
      setMessages((prev) => [...prev, errMsg]);
    } finally {
      setLoading(false);
    }
  }, [currentPlan]);

  return { messages, loading, currentPlan, sendMessage };
}
