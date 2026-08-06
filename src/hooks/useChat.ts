import { useState, useCallback, useEffect } from "react";
import type { ChatMessage, AiResearchResult, LearningPlan } from "../types";
import { api } from "../api/client";
import { formatApiError } from "../utils/errors";
import { notifyDataChanged } from "../utils/events";

export function useChat() {
  const [messages, setMessages] = useState<ChatMessage[]>([]);
  const [loading, setLoading] = useState(false);
  const [currentPlan, setCurrentPlan] = useState<LearningPlan | null>(null);
  const [hydrated, setHydrated] = useState(false);

  useEffect(() => {
    let cancelled = false;
    (async () => {
      try {
        const plans = await api.learning.listPlans();
        if (cancelled) return;
        if (plans.length > 0) {
          // Prefer the most recently created plan
          const latest = [...plans].sort((a, b) =>
            (b.created_at ?? "").localeCompare(a.created_at ?? "")
          )[0];
          setCurrentPlan(latest);
        }
      } catch {
        // Ignore hydrate errors — chat still works for new topics
      } finally {
        if (!cancelled) setHydrated(true);
      }
    })();
    return () => {
      cancelled = true;
    };
  }, []);

  const sendMessage = useCallback(
    async (content: string) => {
      const userMsg: ChatMessage = { role: "user", content };
      setMessages((prev) => [...prev, userMsg]);
      setLoading(true);

      try {
        const trimmed = content.trim();
        const wantsNewTopic =
          /^(新主题|换个主题|重新研究|开新主题)[:：\s]*/i.test(trimmed) ||
          trimmed.toLowerCase().startsWith("new topic:");

        // First message without an active plan, or explicit new-topic request → research
        if (!currentPlan || wantsNewTopic) {
          const topic = wantsNewTopic
            ? trimmed
                .replace(/^(新主题|换个主题|重新研究|开新主题)[:：\s]*/i, "")
                .replace(/^new topic:\s*/i, "")
                .trim() || trimmed
            : trimmed;

          const result: AiResearchResult = await api.ai.startResearch(topic);
          notifyDataChanged();

          const botMsg: ChatMessage = {
            role: "assistant",
            content: `我找到了 ${result.sources.length} 份资料，整理了 ${result.knowledge_points.length} 个知识点。

**学习计划：${result.plan.title}**

知识点列表：
${result.knowledge_points.map((kp, i) => `${i + 1}. ${kp.title} — ${kp.summary}`).join("\n")}

左侧「知识点」已更新，点任意一项即可开始学习与测验。若要开新主题，发送「新主题：xxx」。`,
          };
          setMessages((prev) => [...prev, botMsg]);
          setCurrentPlan(result.plan);
        } else {
          const botMsg: ChatMessage = {
            role: "assistant",
            content: `当前学习计划是「${currentPlan.title}」。请在左侧选择知识点继续学习，或到「复习」巩固已学内容。

若要开新主题，发送「新主题：你想学的内容」。`,
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
    },
    [currentPlan]
  );

  return { messages, loading, currentPlan, sendMessage, hydrated };
}
