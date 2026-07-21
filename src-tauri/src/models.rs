use chrono::NaiveDateTime;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Source {
    pub id: String,
    pub title: String,
    #[serde(rename = "type")]
    pub source_type: String,
    pub content: String,
    pub tags: Vec<String>,
    pub origin: String,
    pub source_url: Option<String>,
    pub hidden: bool,
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateSourceRequest {
    pub title: String,
    #[serde(rename = "type")]
    pub source_type: String,
    pub content: String,
    pub tags: Vec<String>,
    pub origin: String,
    pub source_url: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KnowledgePoint {
    pub id: String,
    pub title: String,
    pub summary: String,
    pub content: String,
    pub tags: Vec<String>,
    pub source_ids: Vec<String>,
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateKnowledgePointRequest {
    pub title: String,
    pub summary: String,
    pub content: String,
    pub tags: Vec<String>,
    pub source_ids: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Relation {
    pub id: String,
    pub from_kp_id: String,
    pub to_kp_id: String,
    pub relation_type: String,
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QuizQuestion {
    pub id: String,
    pub kp_id: String,
    #[serde(rename = "type")]
    pub question_type: String,
    pub question: String,
    pub options: Option<Vec<String>>,
    pub answer: String,
    pub explanation: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QuizAttempt {
    pub id: String,
    pub question_id: String,
    pub user_answer: String,
    pub is_correct: bool,
    pub attempted_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MasteryRecord {
    pub id: String,
    pub kp_id: String,
    pub ease_factor: f64,
    pub interval_days: i32,
    pub repetitions: i32,
    pub next_review_at: String,
    pub last_reviewed_at: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LearningPlan {
    pub id: String,
    pub title: String,
    pub goal: String,
    pub kp_ids: Vec<String>,
    pub status: String,
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateLearningPlanRequest {
    pub title: String,
    pub goal: String,
    pub kp_ids: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SubmitQuizAnswerRequest {
    pub question_id: String,
    pub user_answer: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QuizResult {
    pub question: QuizQuestion,
    pub user_answer: String,
    pub is_correct: bool,
    pub explanation: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AiStartResearchRequest {
    pub topic: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AiResearchResult {
    pub sources: Vec<Source>,
    pub knowledge_points: Vec<KnowledgePoint>,
    pub plan: LearningPlan,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AiGenerateQuizRequest {
    pub kp_ids: Vec<String>,
    pub count: usize,
}

pub fn new_id() -> String {
    Uuid::new_v4().to_string()
}
