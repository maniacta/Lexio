import type { Source, CreateSourceRequest, KnowledgePoint, QuizQuestion, QuizResult, LearningPlan, MasteryRecord, AiResearchResult, SettingsData, ProviderWithModels, ModelProvider, ProviderModel, CreateProviderRequest, UpdateProviderRequest, CreateModelRequest, UpdateModelRequest, TestConnectionResponse, TaskModelEntry, ReviewItem } from "../types";

// Vite proxy forwards /api/* to backend on localhost:3001
const API_BASE = "/api";

async function request<T>(path: string, options?: RequestInit): Promise<T> {
  let res: Response;
  try {
    res = await fetch(`${API_BASE}${path}`, {
      headers: { "Content-Type": "application/json", ...options?.headers },
      ...options,
    });
  } catch {
    throw new Error("NETWORK_ERROR: 无法连接后端服务");
  }
  if (!res.ok) {
    const err = (await res.text()).trim();
    throw new Error(err || `API error ${res.status}`);
  }
  if (res.status === 204) return undefined as T;
  return res.json();
}

export const api = {
  // Sources
  sources: {
    list: (includeHidden?: boolean, search?: string) => {
      const params = new URLSearchParams();
      if (includeHidden) params.set("include_hidden", "true");
      if (search) params.set("search", search);
      return request<Source[]>(`/sources?${params}`);
    },
    get: (id: string) => request<Source>(`/sources/${id}`),
    create: (data: CreateSourceRequest) =>
      request<Source>("/sources", { method: "POST", body: JSON.stringify(data) }),
    toggleHidden: (id: string, hidden: boolean) =>
      request<void>(`/sources/${id}/hide`, { method: "POST", body: JSON.stringify({ hidden }) }),
  },

  // Knowledge Points
  knowledge: {
    list: (search?: string, ids?: string[]) => {
      const params = new URLSearchParams();
      if (search) params.set("search", search);
      if (ids) params.set("ids", ids.join(","));
      return request<KnowledgePoint[]>(`/knowledge?${params}`);
    },
    get: (id: string) => request<KnowledgePoint>(`/knowledge/${id}`),
    create: (data: { title: string; summary: string; content: string; tags: string[]; source_ids: string[] }) =>
      request<KnowledgePoint>("/knowledge", { method: "POST", body: JSON.stringify(data) }),
    delete: (id: string) => request<void>(`/knowledge/${id}`, { method: "DELETE" }),
  },

  // Quiz
  quiz: {
    getByKp: (kpId: string) => request<QuizQuestion[]>(`/quiz/kp/${kpId}`),
    submit: (questionId: string, userAnswer: string) =>
      request<QuizResult>("/quiz/submit", {
        method: "POST",
        body: JSON.stringify({ question_id: questionId, user_answer: userAnswer }),
      }),
  },

  // Learning
  learning: {
    listPlans: () => request<LearningPlan[]>("/learning/plans"),
    createPlan: (data: { title: string; goal: string; kp_ids: string[] }) =>
      request<LearningPlan>("/learning/plans", { method: "POST", body: JSON.stringify(data) }),
    getDueReviews: () => request<MasteryRecord[]>("/learning/reviews/due"),
    getDueReviewsWithKp: () => request<ReviewItem[]>("/learning/reviews/due?with_kp=true"),
  },

  // AI
  ai: {
    startResearch: (topic: string) =>
      request<AiResearchResult>("/ai/research", { method: "POST", body: JSON.stringify({ topic }) }),
    generateQuiz: (kpId: string, count: number = 3) =>
      request<QuizQuestion[]>("/ai/generate-quiz", {
        method: "POST",
        body: JSON.stringify({ kp_id: kpId, count }),
      }),
    updateMastery: (kpId: string, isCorrect: boolean) =>
      request<MasteryRecord>("/ai/update-mastery", {
        method: "POST",
        body: JSON.stringify({ kp_id: kpId, is_correct: isCorrect }),
      }),
  },

  // Settings
  settings: {
    getAll: () => request<SettingsData>("/settings"),
    listProviders: () => request<ProviderWithModels[]>("/settings/providers"),
    createProvider: (data: CreateProviderRequest) =>
      request<ModelProvider>("/settings/providers", { method: "POST", body: JSON.stringify(data) }),
    updateProvider: (id: string, data: UpdateProviderRequest) =>
      request<void>(`/settings/providers/${id}`, { method: "PUT", body: JSON.stringify(data) }),
    deleteProvider: (id: string) =>
      request<void>(`/settings/providers/${id}`, { method: "DELETE" }),
    createModel: (providerId: string, data: CreateModelRequest) =>
      request<ProviderModel>(`/settings/providers/${providerId}/models`, {
        method: "POST", body: JSON.stringify(data),
      }),
    updateModel: (providerId: string, modelId: string, data: UpdateModelRequest) =>
      request<void>(`/settings/providers/${providerId}/models/${modelId}`, {
        method: "PUT", body: JSON.stringify(data),
      }),
    deleteModel: (providerId: string, modelId: string) =>
      request<void>(`/settings/providers/${providerId}/models/${modelId}`, { method: "DELETE" }),
    getTaskModels: () => request<Record<string, TaskModelEntry>>("/settings/tasks"),
    setTaskModel: (taskName: string, modelId: string | null) =>
      request<void>(`/settings/tasks/${taskName}`, {
        method: "PUT", body: JSON.stringify({ model_id: modelId }),
      }),
    updateGeneral: (data: Record<string, string | boolean>) =>
      request<void>("/settings/general", { method: "PUT", body: JSON.stringify(data) }),
    testConnection: (providerId: string, modelName: string) =>
      request<TestConnectionResponse>("/settings/test-connection", {
        method: "POST",
        body: JSON.stringify({ provider_id: providerId, model_name: modelName }),
      }),
  },
};
