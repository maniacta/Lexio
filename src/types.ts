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

export interface AiResearchResult {
  sources: Source[];
  knowledge_points: KnowledgePoint[];
  plan: LearningPlan;
}

export interface ChatMessage {
  role: "user" | "assistant" | "system";
  content: string;
}
