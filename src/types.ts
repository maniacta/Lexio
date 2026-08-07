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

export interface Relation {
  id: string;
  from_kp_id: string;
  to_kp_id: string;
  relation_type: "prerequisite" | "related" | "extension" | string;
  created_at: string;
}

export interface CreateRelationRequest {
  to_kp_id: string;
  relation_type: "prerequisite" | "related" | "extension";
}

export interface QuizQuestion {
  id: string;
  kp_id: string;
  type: "multiple_choice" | "fill_blank" | "analysis";
  question: string;
  options: string[] | null;
  /** Omitted on list/generate; only present after some legacy responses. */
  answer?: string;
  explanation?: string;
}

export interface QuizResult {
  question: QuizQuestion;
  user_answer: string;
  is_correct: boolean;
  explanation: string;
  correct_answer?: string;
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

export interface ChatAction {
  type: "navigate_learning" | "start_quiz" | "view_source" | "start_research";
  label: string;
  payload: { kpId?: string; kpTitle?: string; sourceId?: string; topic?: string };
}

export interface ChatMessage {
  role: "user" | "assistant" | "system";
  content: string;
  actions?: ChatAction[];
  context?: {
    plan?: LearningPlan;
    kps?: KnowledgePoint[];
  };
}

export interface ChatRequest {
  messages: { role: string; content: string }[];
  context?: {
    plan_id?: string;
    current_kp_id?: string;
  };
}

export interface ChatResponse {
  content: string;
  actions: ChatAction[];
}

export interface ChatSession {
  id: string;
  title: string;
  plan_id: string | null;
  message_count: number;
  updated_at: string;
}

export interface ModelProvider {
  id: string;
  name: string;
  base_url: string;
  api_key: string;
  /** Vendor kind: deepseek | openai | anthropic */
  api_format: string;
  is_preset: boolean;
  is_default: boolean;
  created_at: string;
}

export interface ProviderModel {
  id: string;
  provider_id: string;
  model_name: string;
  /** Kept for API compat; not shown or edited in settings. */
  temperature: number;
  /** Kept for API compat; not shown or edited in settings. */
  max_tokens: number;
  is_default: boolean;
}

export interface ProviderWithModels extends ModelProvider {
  models: ProviderModel[];
}

export interface ProviderKindInfo {
  kind: string;
  display_name: string;
  default_base_url: string;
  implemented: boolean;
  models: {
    model_name: string;
    is_default: boolean;
  }[];
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
  kind: string;
  api_key: string;
  base_url?: string;
  set_default?: boolean;
}

export interface UpdateProviderRequest {
  name?: string;
  base_url: string;
  api_key?: string;
  is_default?: boolean;
}

export interface CreateModelRequest {
  model_name: string;
  is_default?: boolean;
}

export interface UpdateModelRequest {
  model_name: string;
  is_default?: boolean;
}

export interface TestConnectionResponse {
  ok: boolean;
  message: string;
}
