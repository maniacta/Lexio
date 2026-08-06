use crate::db::Database;
use crate::models::{new_id, QuizAttempt, QuizQuestion, SubmitQuizAnswerRequest};

pub fn create_question(db: &Database, q: &QuizQuestion) -> Result<(), String> {
    let conn = db.conn.lock().map_err(|e| e.to_string())?;
    let options = q.options.as_ref().map(|o| serde_json::to_string(o).unwrap_or_default());
    conn.execute(
        "INSERT INTO quiz_questions (id, kp_id, type, question, options, answer, explanation) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
        rusqlite::params![q.id, q.kp_id, q.question_type, q.question, options, q.answer, q.explanation],
    ).map_err(|e| e.to_string())?;
    Ok(())
}

pub fn get_questions_by_kp(db: &Database, kp_id: &str) -> Result<Vec<QuizQuestion>, String> {
    let conn = db.conn.lock().map_err(|e| e.to_string())?;
    let mut stmt = conn
        .prepare("SELECT id, kp_id, type, question, options, answer, explanation FROM quiz_questions WHERE kp_id = ?1")
        .map_err(|e| e.to_string())?;
    let questions: Vec<QuizQuestion> = stmt
        .query_map([kp_id], |row| question_from_row(row))
        .map_err(|e| e.to_string())?
        .filter_map(|r| r.ok())
        .collect();
    Ok(questions)
}

pub fn get_questions_by_ids(db: &Database, ids: &[String]) -> Result<Vec<QuizQuestion>, String> {
    if ids.is_empty() { return Ok(vec![]); }
    let conn = db.conn.lock().map_err(|e| e.to_string())?;
    let placeholders: Vec<String> = ids.iter().enumerate().map(|(i, _)| format!("?{}", i + 1)).collect();
    let sql = format!(
        "SELECT id, kp_id, type, question, options, answer, explanation FROM quiz_questions WHERE id IN ({})",
        placeholders.join(",")
    );
    let mut stmt = conn.prepare(&sql).map_err(|e| e.to_string())?;
    let params: Vec<&dyn rusqlite::types::ToSql> = ids.iter().map(|id| id as &dyn rusqlite::types::ToSql).collect();
    let questions: Vec<QuizQuestion> = stmt
        .query_map(params.as_slice(), |row| question_from_row(row))
        .map_err(|e| e.to_string())?
        .filter_map(|r| r.ok())
        .collect();
    Ok(questions)
}

pub fn record_attempt(db: &Database, req: &SubmitQuizAnswerRequest, is_correct: bool) -> Result<QuizAttempt, String> {
    let conn = db.conn.lock().map_err(|e| e.to_string())?;
    let id = new_id();
    let now = chrono::Utc::now().to_rfc3339();
    let attempt = QuizAttempt {
        id: id.clone(),
        question_id: req.question_id.clone(),
        user_answer: req.user_answer.clone(),
        is_correct,
        attempted_at: now.clone(),
    };
    conn.execute(
        "INSERT INTO quiz_attempts (id, question_id, user_answer, is_correct, attempted_at) VALUES (?1, ?2, ?3, ?4, ?5)",
        rusqlite::params![id, attempt.question_id, attempt.user_answer, is_correct as i32, now],
    ).map_err(|e| e.to_string())?;
    Ok(attempt)
}

pub fn get_attempts_by_kp(db: &Database, kp_id: &str) -> Result<Vec<QuizAttempt>, String> {
    let conn = db.conn.lock().map_err(|e| e.to_string())?;
    let mut stmt = conn
        .prepare("SELECT qa.id, qa.question_id, qa.user_answer, qa.is_correct, qa.attempted_at FROM quiz_attempts qa JOIN quiz_questions qq ON qa.question_id = qq.id WHERE qq.kp_id = ?1 ORDER BY qa.attempted_at DESC")
        .map_err(|e| e.to_string())?;
    let attempts: Vec<QuizAttempt> = stmt
        .query_map([kp_id], |row| attempt_from_row(row))
        .map_err(|e| e.to_string())?
        .filter_map(|r| r.ok())
        .collect();
    Ok(attempts)
}

/// Normalize answers for comparison: ignore case, whitespace, and common punctuation.
pub fn answers_match(user: &str, expected: &str) -> bool {
    normalize_answer(user) == normalize_answer(expected)
}

fn normalize_answer(s: &str) -> String {
    s.chars()
        .filter(|c| !c.is_whitespace())
        .filter(|c| {
            !matches!(
                *c,
                ',' | '.' | '!' | '?' | ';' | ':' | '"' | '\'' | '`'
                    | '，' | '。' | '！' | '？' | '；' | '：' | '“' | '”' | '‘' | '’'
                    | '(' | ')' | '（' | '）' | '[' | ']' | '【' | '】'
                    | '、' | '·' | '…'
            )
        })
        .flat_map(|c| c.to_lowercase())
        .collect()
}

fn question_from_row(row: &rusqlite::Row) -> rusqlite::Result<QuizQuestion> {
    let options_str: Option<String> = row.get(4)?;
    Ok(QuizQuestion {
        id: row.get(0)?,
        kp_id: row.get(1)?,
        question_type: row.get(2)?,
        question: row.get(3)?,
        options: options_str.and_then(|s| serde_json::from_str(&s).ok()),
        answer: row.get(5)?,
        explanation: row.get(6)?,
    })
}

fn attempt_from_row(row: &rusqlite::Row) -> rusqlite::Result<QuizAttempt> {
    Ok(QuizAttempt {
        id: row.get(0)?,
        question_id: row.get(1)?,
        user_answer: row.get(2)?,
        is_correct: row.get::<_, i32>(3)? != 0,
        attempted_at: row.get(4)?,
    })
}
