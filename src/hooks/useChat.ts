import { useState, useCallback, useEffect, useRef } from "react";
import type { ChatMessage, AiResearchResult, ChatSession, ChatAction } from "../types";
import { api } from "../api/client";
import { formatApiError, isAbortError } from "../utils/errors";
import { notifyDataChanged } from "../utils/events";
import { logger } from "../utils/logger";

interface PlanContext {
  plan_id: string;
  title: string;
}

const NEW_SESSION_TITLE = "新对话";

export function useChat() {
  const [sessions, setSessions] = useState<ChatSession[]>([]);
  const [activeSessionId, setActiveSessionId] = useState<string | null>(null);
  const [messages, setMessages] = useState<ChatMessage[]>([]);
  const [loading, setLoading] = useState(false);
  const [hydrated, setHydrated] = useState(false);
  const [planContext, setPlanContext] = useState<PlanContext | null>(null);
  const researchAbort = useRef<AbortController | null>(null);

  /** Load messages for a session and restore its plan context. */
  const loadSession = useCallback(async (sessionId: string, signal?: AbortSignal) => {
    const msgs = await api.chatApi.getMessages(sessionId, signal);
    if (signal?.aborted) return;
    setMessages(msgs);
    // Restore plan context from the session's plan_id (title looked up below).
    const s = (await api.chatApi.listSessions(signal)).find((x) => x.id === sessionId);
    if (signal?.aborted) return;
    if (s?.plan_id) {
      try {
        const plans = await api.learning.listPlans(signal);
        const plan = plans.find((p) => p.id === s.plan_id);
        setPlanContext({ plan_id: s.plan_id, title: plan?.title ?? "" });
      } catch {
        setPlanContext({ plan_id: s.plan_id, title: "" });
      }
    } else {
      setPlanContext(null);
    }
  }, []);

  // Initial hydration: load sessions, open the most recent one (or create).
  useEffect(() => {
    const ac = new AbortController();
    (async () => {
      try {
        const list = await api.chatApi.listSessions(ac.signal);
        if (ac.signal.aborted) return;
        if (list.length > 0) {
          const latest = list[0];
          setSessions(list);
          setActiveSessionId(latest.id);
          await loadSession(latest.id, ac.signal);
        } else {
          const s = await api.chatApi.createSession(NEW_SESSION_TITLE, ac.signal);
          if (ac.signal.aborted) return;
          setSessions([s]);
          setActiveSessionId(s.id);
        }
      } catch (e) {
        if (!isAbortError(e)) {
          // Ignore hydrate errors — chat still works for new topics.
        }
      } finally {
        if (!ac.signal.aborted) setHydrated(true);
      }
    })();
    return () => {
      ac.abort();
      researchAbort.current?.abort();
    };
  }, [loadSession]);

  /** Create a fresh session and switch to it. */
  const newSession = useCallback(async () => {
    try {
      const s = await api.chatApi.createSession(NEW_SESSION_TITLE);
      setSessions((prev) => [s, ...prev]);
      setActiveSessionId(s.id);
      setMessages([]);
      setPlanContext(null);
    } catch (err) {
      logger.log({
        level: "error",
        category: "chat",
        action: "new_session_error",
        error_message: err instanceof Error ? err.message : String(err),
      });
    }
  }, []);

  /** Switch to an existing session. */
  const switchSession = useCallback(
    async (sessionId: string) => {
      if (sessionId === activeSessionId) return;
      setActiveSessionId(sessionId);
      setLoading(true);
      try {
        await loadSession(sessionId);
      } catch (e) {
        if (!isAbortError(e)) {
          setMessages([]);
          setPlanContext(null);
        }
      } finally {
        setLoading(false);
      }
    },
    [activeSessionId, loadSession]
  );

  /** Delete a session; if it was active, open the next one (or create). */
  const deleteSession = useCallback(
    async (sessionId: string) => {
      try {
        await api.chatApi.deleteSession(sessionId);
      } catch (err) {
        logger.log({
          level: "error",
          category: "chat",
          action: "delete_session_error",
          error_message: err instanceof Error ? err.message : String(err),
        });
        return;
      }
      const remaining = sessions.filter((s) => s.id !== sessionId);
      setSessions(remaining);
      if (sessionId !== activeSessionId) return;
      if (remaining.length > 0) {
        const next = remaining[0];
        setActiveSessionId(next.id);
        setLoading(true);
        try {
          await loadSession(next.id);
        } finally {
          setLoading(false);
        }
      } else {
        try {
          const s = await api.chatApi.createSession(NEW_SESSION_TITLE);
          setSessions([s]);
          setActiveSessionId(s.id);
          setMessages([]);
          setPlanContext(null);
        } catch (err) {
          logger.log({
            level: "error",
            category: "chat",
            action: "new_session_error",
            error_message: err instanceof Error ? err.message : String(err),
          });
        }
      }
    },
    [sessions, activeSessionId, loadSession]
  );

  /** Format research result as a user-visible message + set plan context. */
  const handleResearchResult = useCallback(
    async (result: AiResearchResult) => {
      setPlanContext({ plan_id: result.plan.id, title: result.plan.title });

      const kpList = result.knowledge_points
        .map((kp, i) => `${i + 1}. **${kp.title}** — ${kp.summary}`)
        .join("\n");

      const botMsg: ChatMessage = {
        role: "assistant",
        content: `我找到了 ${result.sources.length} 份资料，提取了 ${result.knowledge_points.length} 个知识点。\n\n**学习计划：${result.plan.title}**\n\n${kpList}`,
        actions: result.knowledge_points.map((kp) => ({
          type: "navigate_learning" as const,
          label: `📌 ${kp.title}`,
          payload: { kpId: kp.id, kpTitle: kp.title },
        })),
        context: { plan: result.plan, kps: result.knowledge_points },
      };
      setMessages((prev) => [...prev, botMsg]);

      // Bind the plan to the active session so it survives reloads.
      if (activeSessionId) {
        try {
          await api.chatApi.setSessionPlan(activeSessionId, result.plan.id);
        } catch {
          // non-fatal
        }
      }
    },
    [activeSessionId]
  );

  /** Run a research for the given topic (shared by trigger + start_research). */
  const runResearch = useCallback(
    async (topic: string, signal?: AbortSignal) => {
      logger.log({
        level: "info",
        category: "ai",
        action: "start_research",
        user_action: "发起了主题研究",
        params_summary: { topic },
      });
      const result: AiResearchResult = await api.ai.startResearch(topic, signal);
      if (signal?.aborted) return;
      notifyDataChanged();
      await handleResearchResult(result);
      logger.log({
        level: "info",
        category: "ai",
        action: "start_research",
        result_summary: {
          sources: result.sources.length,
          knowledge_points: result.knowledge_points.length,
        },
      });
    },
    [handleResearchResult]
  );

  const sendMessage = useCallback(
    async (content: string) => {
      // Ensure a session exists before appending.
      let sessionId = activeSessionId;
      if (!sessionId) {
        try {
          const s = await api.chatApi.createSession(NEW_SESSION_TITLE);
          setSessions((prev) => [s, ...prev]);
          setActiveSessionId(s.id);
          sessionId = s.id;
        } catch (err) {
          logger.log({
            level: "error",
            category: "chat",
            action: "new_session_error",
            error_message: err instanceof Error ? err.message : String(err),
          });
          return;
        }
      }

      const userMsg: ChatMessage = { role: "user", content };
      setMessages((prev) => [...prev, userMsg]);
      setLoading(true);
      const start = Date.now();

      // Persist the user message (non-blocking on failure).
      try {
        await api.chatApi.appendMessage({ session_id: sessionId, role: "user", content });
      } catch (err) {
        logger.log({
          level: "error",
          category: "chat",
          action: "persist_message_error",
          error_message: err instanceof Error ? err.message : String(err),
        });
      }
      // Auto-title from the first user message.
      if (sessions.length === 0 || (activeSessionId && messages.length === 0)) {
        const t = content.slice(0, 20);
        setSessions((prev) =>
          prev.map((s) => (s.id === sessionId ? { ...s, title: t } : s))
        );
      }

      researchAbort.current?.abort();
      const ac = new AbortController();
      researchAbort.current = ac;

      try {
        const trimmed = content.trim();
        const wantsNewTopic =
          /^(新主题|换个主题|重新研究|开新主题)[:：\s]*/i.test(trimmed) ||
          trimmed.toLowerCase().startsWith("new topic:");

        // A learning request: message STARTS with a trigger word and has a
        // topic after it — "我不学习" no longer matches via substring.
        const researchTriggers = ["我想学", "我要学", "想学", "教我学", "学习", "学"];
        const isLearnRequest = researchTriggers.some(
          (t) => trimmed.startsWith(t) && trimmed.length > t.length
        );
        // With an active plan, only research again when the topic is new;
        // same-topic messages are chat follow-ups about the current plan.
        const sameTopic =
          !!planContext && !!planContext.title && trimmed.includes(planContext.title);

        if (wantsNewTopic) {
          const topic = trimmed
            .replace(/^(新主题|换个主题|重新研究|开新主题)[:：\s]*/i, "")
            .replace(/^new topic:\s*/i, "")
            .trim() || trimmed;
          await runResearch(topic, ac.signal);
          if (ac.signal.aborted) return;
        } else if (isLearnRequest && (!planContext || !sameTopic)) {
          await runResearch(trimmed, ac.signal);
          if (ac.signal.aborted) return;
        } else {
          // Otherwise → LLM chat
          logger.log({
            level: "info",
            category: "ai",
            action: "send_chat",
            user_action: "发送了聊天消息",
            params_summary: { chars: content.length },
          });
          const allMessages = [...messages, userMsg];
          const resp = await api.ai.chat(
            {
              messages: allMessages.map((m) => ({ role: m.role, content: m.content })),
              context: planContext ?? undefined,
            },
            ac.signal
          );
          if (ac.signal.aborted) return;

          const botMsg: ChatMessage = {
            role: "assistant",
            content: resp.content,
            actions: resp.actions,
          };
          setMessages((prev) => [...prev, botMsg]);
          try {
            await api.chatApi.appendMessage({
              session_id: sessionId,
              role: "assistant",
              content: resp.content,
              actions: resp.actions,
            });
          } catch (err) {
            logger.log({
              level: "error",
              category: "chat",
              action: "persist_message_error",
              error_message: err instanceof Error ? err.message : String(err),
            });
          }
          logger.log({
            level: "info",
            category: "ai",
            action: "send_chat",
            result_summary: { chars: resp.content.length },
            duration_ms: Date.now() - start,
          });

          // start_research action → run research as a one-click correction.
          const researchAction = resp.actions?.find((a: ChatAction) => a.type === "start_research");
          if (researchAction?.payload?.topic) {
            await runResearch(researchAction.payload.topic, ac.signal);
            if (ac.signal.aborted) return;
          }
        }
      } catch (err) {
        if (isAbortError(err)) return;
        logger.log({
          level: "error",
          category: "ai",
          action: "chat_error",
          user_action: "AI 交互出错",
          error_message: err instanceof Error ? err.message : String(err),
        });
        const errMsg: ChatMessage = {
          role: "assistant",
          content: formatApiError(err),
        };
        setMessages((prev) => [...prev, errMsg]);
      } finally {
        if (!ac.signal.aborted) setLoading(false);
      }
    },
    [activeSessionId, sessions, messages, planContext, runResearch]
  );

  const clearPlan = useCallback(() => setPlanContext(null), []);

  return {
    messages,
    loading,
    hydrated,
    sessions,
    activeSessionId,
    sendMessage,
    newSession,
    switchSession,
    deleteSession,
    clearPlan,
  };
}
