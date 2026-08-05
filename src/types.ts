export interface Source {
  id: string;
  title: string;
  type: "url" | "text" | "file";
  content: string;
  tags: string[];
  origin: "user" | "ai_search";
  source_url: string | null;
  hidden: boolean;
  created_at: string;
}

export interface CreateSourceRequest {
  title: string;
  type: string;
  content: string;
  tags: string[];
  origin: string;
  source_url?: string;
}

export interface KnowledgePoint {
  id: string;
  title: string;
  summary: string;
  content: string;
  tags: string[];
  source_ids: string[];
  created_at: string;
}

export interface QuizQuestion {
  id: string;
  kp_id: string;
  type: "multiple_choice" | "fill_blank" | "analysis";
  question: string;
  options: string[] | null;
  answer: string;
  explanation: string;
}

export interface QuizResult {
  question: QuizQuestion;
  user_answer: string;
  is_correct: boolean;
  explanation: string;
}

export interface LearningPlan {
  id: string;
  title: string;
  goal: string;
  kp_ids: string[];
  status: "active" | "completed" | "paused";
  created_at: string;
}

export interface MasteryRecord {
  id: string;
  kp_id: string;
  ease_factor: number;
  interval_days: number;
  repetitions: number;
  next_review_at: string;
  last_reviewed_at: string | null;
}

export interface ReviewItem {
  mastery: MasteryRecord;
  knowledge_point: KnowledgePoint;
}

export interface ReviewResult {
  kp_id: string;
  kp_title: string;
  is_correct: boolean;
  next_review_at: string;
}

export interface AiResearchResult {
  sources: Source[];
  knowledge_points: KnowledgePoint[];
  plan: LearningPlan;
}

export interface ChatMessage {
  role: "user" | "assistant" | "system";
  content: string;
}

export interface ModelProvider {
  id: string;
  name: string;
  base_url: string;
  api_key: string;
  api_format: string;
  is_preset: boolean;
  is_default: boolean;
  created_at: string;
}

export interface ProviderModel {
  id: string;
  provider_id: string;
  model_name: string;
  temperature: number;
  max_tokens: number;
  is_default: boolean;
}

export interface ProviderWithModels extends ModelProvider {
  models: ProviderModel[];
}

export interface TaskModelEntry {
  model_id: string | null;
  resolved: string | null;
}

export interface SettingsData {
  general: Record<string, string>;
  providers: ProviderWithModels[];
  task_models: Record<string, TaskModelEntry>;
}

export interface CreateProviderRequest {
  name: string;
  base_url: string;
  api_key: string;
  api_format?: string;
}

export interface UpdateProviderRequest {
  name: string;
  base_url: string;
  api_key?: string;
  api_format?: string;
  is_default?: boolean;
}

export interface CreateModelRequest {
  model_name: string;
  temperature?: number;
  max_tokens?: number;
  is_default?: boolean;
}

export interface UpdateModelRequest {
  model_name: string;
  temperature?: number;
  max_tokens?: number;
  is_default?: boolean;
}

export interface TestConnectionResponse {
  ok: boolean;
  message: string;
}
