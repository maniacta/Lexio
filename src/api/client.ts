import type { Source, CreateSourceRequest, KnowledgePoint, QuizQuestion, QuizResult, LearningPlan, MasteryRecord, AiResearchResult, SettingsData, ProviderWithModels, ModelProvider, ProviderModel, CreateProviderRequest, UpdateProviderRequest, CreateModelRequest, UpdateModelRequest, TestConnectionResponse, TaskModelEntry, ReviewItem, ProviderKindInfo, Relation, CreateRelationRequest, ChatRequest, ChatResponse, ChatSession, ChatMessage, ChatAction } from "../types";
import { isTauri } from "../utils/tauri";
import { isAbortError } from "../utils/errors";

// Vite proxy forwards /api/* to backend on localhost:3001
const API_BASE = "/api";

let cachedToken: string | null = null;
let tokenPromise: Promise<string> | null = null;

async function resolveApiToken(): Promise<string> {
  if (cachedToken) return cachedToken;
  if (!tokenPromise) {
    tokenPromise = (async () => {
      if (isTauri()) {
        const { invoke } = await import("@tauri-apps/api/core");
        return invoke<string>("get_api_token");
      }
      const res = await fetch(`${API_BASE}/auth/token`);
      if (!res.ok) {
        throw new Error("UNAUTHORIZED: 无法获取本地 API Token");
      }
      const data = (await res.json()) as { token: string };
      return data.token;
    })().then((token) => {
      cachedToken = token;
      return token;
    }).catch((err) => {
      tokenPromise = null;
      throw err;
    });
  }
  return tokenPromise;
}

export type RequestOptions = RequestInit & { signal?: AbortSignal };

async function request<T>(path: string, options?: RequestOptions): Promise<T> {
  const token = await resolveApiToken();
  let res: Response;
  try {
    const headers = new Headers(options?.headers);
    if (!headers.has("Content-Type") && options?.body) {
      headers.set("Content-Type", "application/json");
    }
    headers.set("X-Lexio-Token", token);
    res = await fetch(`${API_BASE}${path}`, {
      ...options,
      headers,
      signal: options?.signal,
    });
  } catch (e) {
    if (isAbortError(e)) throw e;
    throw new Error("NETWORK_ERROR: 无法连接后端服务");
  }
  if (!res.ok) {
    const err = (await res.text()).trim();
    throw new Error(err || `API error ${res.status}`);
  }
  // 204 / empty 200 bodies (e.g. updateProvider) must not call res.json()
  if (res.status === 204) return undefined as T;
  const text = await res.text();
  if (!text.trim()) return undefined as T;
  return JSON.parse(text) as T;
}

export const api = {
  // Sources
  sources: {
    list: (includeHidden?: boolean, search?: string, signal?: AbortSignal) => {
      const params = new URLSearchParams();
      if (includeHidden) params.set("include_hidden", "true");
      if (search) params.set("search", search);
      return request<Source[]>(`/sources?${params}`, { signal });
    },
    get: (id: string, signal?: AbortSignal) =>
      request<Source>(`/sources/${id}`, { signal }),
    create: (data: CreateSourceRequest, signal?: AbortSignal) =>
      request<Source>("/sources", { method: "POST", body: JSON.stringify(data), signal }),
    toggleHidden: (id: string, hidden: boolean, signal?: AbortSignal) =>
      request<void>(`/sources/${id}/hide`, {
        method: "POST",
        body: JSON.stringify({ hidden }),
        signal,
      }),
  },

  // Knowledge Points
  knowledge: {
    list: (search?: string, ids?: string[], signal?: AbortSignal) => {
      const params = new URLSearchParams();
      if (search) params.set("search", search);
      if (ids) params.set("ids", ids.join(","));
      return request<KnowledgePoint[]>(`/knowledge?${params}`, { signal });
    },
    get: (id: string, signal?: AbortSignal) =>
      request<KnowledgePoint>(`/knowledge/${id}`, { signal }),
    create: (
      data: { title: string; summary: string; content: string; tags: string[]; source_ids: string[] },
      signal?: AbortSignal
    ) =>
      request<KnowledgePoint>("/knowledge", {
        method: "POST",
        body: JSON.stringify(data),
        signal,
      }),
    delete: (id: string, signal?: AbortSignal) =>
      request<void>(`/knowledge/${id}`, { method: "DELETE", signal }),
    listRelations: (kpId: string, signal?: AbortSignal) =>
      request<Relation[]>(`/knowledge/${kpId}/relations`, { signal }),
    createRelation: (kpId: string, data: CreateRelationRequest, signal?: AbortSignal) =>
      request<Relation>(`/knowledge/${kpId}/relations`, {
        method: "POST",
        body: JSON.stringify(data),
        signal,
      }),
    deleteRelation: (relationId: string, signal?: AbortSignal) =>
      request<void>(`/relations/${relationId}`, { method: "DELETE", signal }),
  },

  // Quiz
  quiz: {
    getByKp: (kpId: string, signal?: AbortSignal) =>
      request<QuizQuestion[]>(`/quiz/kp/${kpId}`, { signal }),
    submit: (questionId: string, userAnswer: string, signal?: AbortSignal) =>
      request<QuizResult>("/quiz/submit", {
        method: "POST",
        body: JSON.stringify({ question_id: questionId, user_answer: userAnswer }),
        signal,
      }),
  },

  // Learning
  learning: {
    listPlans: (signal?: AbortSignal) =>
      request<LearningPlan[]>("/learning/plans", { signal }),
    createPlan: (
      data: { title: string; goal: string; kp_ids: string[] },
      signal?: AbortSignal
    ) =>
      request<LearningPlan>("/learning/plans", {
        method: "POST",
        body: JSON.stringify(data),
        signal,
      }),
    getDueReviews: (signal?: AbortSignal) =>
      request<MasteryRecord[]>("/learning/reviews/due", { signal }),
    getDueReviewsWithKp: (signal?: AbortSignal) =>
      request<ReviewItem[]>("/learning/reviews/due?with_kp=true", { signal }),
  },

  // AI
  ai: {
    startResearch: (topic: string, signal?: AbortSignal) =>
      request<AiResearchResult>("/ai/research", {
        method: "POST",
        body: JSON.stringify({ topic }),
        signal,
      }),
    generateQuiz: (kpId: string, count: number = 3, signal?: AbortSignal) =>
      request<QuizQuestion[]>("/ai/generate-quiz", {
        method: "POST",
        body: JSON.stringify({ kp_id: kpId, count }),
        signal,
      }),
    updateMastery: (kpId: string, isCorrect: boolean, signal?: AbortSignal) =>
      request<MasteryRecord>("/ai/update-mastery", {
        method: "POST",
        body: JSON.stringify({ kp_id: kpId, is_correct: isCorrect }),
        signal,
      }),
    chat: (data: ChatRequest, signal?: AbortSignal) =>
      request<ChatResponse>("/ai/chat", {
        method: "POST",
        body: JSON.stringify(data),
        signal,
      }),
  },

  // Chat sessions
  chatApi: {
    listSessions: (signal?: AbortSignal) =>
      request<ChatSession[]>("/chat/sessions", { signal }),
    createSession: (title: string, signal?: AbortSignal) =>
      request<ChatSession>("/chat/sessions", {
        method: "POST",
        body: JSON.stringify({ title }),
        signal,
      }),
    getMessages: (sessionId: string, signal?: AbortSignal) =>
      request<ChatMessage[]>(`/chat/sessions/${sessionId}/messages`, { signal }),
    appendMessage: (
      data: {
        session_id: string;
        role: string;
        content: string;
        actions?: ChatAction[];
        context?: unknown;
      },
      signal?: AbortSignal
    ) =>
      request<ChatMessage>("/chat/messages", {
        method: "POST",
        body: JSON.stringify({
          ...data,
          actions: data.actions ? JSON.stringify(data.actions) : undefined,
          context: data.context ? JSON.stringify(data.context) : undefined,
        }),
        signal,
      }),
    deleteSession: (sessionId: string, signal?: AbortSignal) =>
      request<void>(`/chat/sessions/${sessionId}`, { method: "DELETE", signal }),
    setSessionPlan: (sessionId: string, planId: string, signal?: AbortSignal) =>
      request<void>(`/chat/sessions/${sessionId}/plan`, {
        method: "POST",
        body: JSON.stringify({ plan_id: planId }),
        signal,
      }),
  },

  // Settings
  settings: {
    getAll: (signal?: AbortSignal) => request<SettingsData>("/settings", { signal }),
    listProviders: (signal?: AbortSignal) =>
      request<ProviderWithModels[]>("/settings/providers", { signal }),
    listProviderKinds: (signal?: AbortSignal) =>
      request<ProviderKindInfo[]>("/settings/provider-kinds", { signal }),
    createProvider: (data: CreateProviderRequest, signal?: AbortSignal) =>
      request<ModelProvider>("/settings/providers", {
        method: "POST",
        body: JSON.stringify(data),
        signal,
      }),
    updateProvider: (id: string, data: UpdateProviderRequest, signal?: AbortSignal) =>
      request<void>(`/settings/providers/${id}`, {
        method: "PUT",
        body: JSON.stringify(data),
        signal,
      }),
    deleteProvider: (id: string, signal?: AbortSignal) =>
      request<void>(`/settings/providers/${id}`, { method: "DELETE", signal }),
    createModel: (providerId: string, data: CreateModelRequest, signal?: AbortSignal) =>
      request<ProviderModel>(`/settings/providers/${providerId}/models`, {
        method: "POST",
        body: JSON.stringify(data),
        signal,
      }),
    updateModel: (
      providerId: string,
      modelId: string,
      data: UpdateModelRequest,
      signal?: AbortSignal
    ) =>
      request<void>(`/settings/providers/${providerId}/models/${modelId}`, {
        method: "PUT",
        body: JSON.stringify(data),
        signal,
      }),
    deleteModel: (providerId: string, modelId: string, signal?: AbortSignal) =>
      request<void>(`/settings/providers/${providerId}/models/${modelId}`, {
        method: "DELETE",
        signal,
      }),
    setModelDefault: (providerId: string, modelId: string, signal?: AbortSignal) =>
      request<void>(`/settings/providers/${providerId}/models/${modelId}/default`, {
        method: "POST",
        signal,
      }),
    getTaskModels: (signal?: AbortSignal) =>
      request<Record<string, TaskModelEntry>>("/settings/tasks", { signal }),
    setTaskModel: (taskName: string, modelId: string | null, signal?: AbortSignal) =>
      request<void>(`/settings/tasks/${taskName}`, {
        method: "PUT",
        body: JSON.stringify({ model_id: modelId }),
        signal,
      }),
    updateGeneral: (data: Record<string, string | boolean>, signal?: AbortSignal) =>
      request<void>("/settings/general", {
        method: "PUT",
        body: JSON.stringify(data),
        signal,
      }),
    testConnection: (providerId: string, modelName: string, signal?: AbortSignal) =>
      request<TestConnectionResponse>("/settings/test-connection", {
        method: "POST",
        body: JSON.stringify({ provider_id: providerId, model_name: modelName }),
        signal,
      }),
  },
};
