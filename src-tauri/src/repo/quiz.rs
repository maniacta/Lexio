use crate::db::Database;
use crate::models::{new_id, QuizAttempt, QuizQuestion, SubmitQuizAnswerRequest};

pub fn create_question(db: &Database, q: &QuizQuestion) -> Result<(), String> {
    let conn = db.conn.lock().map_err(crate::error::internal)?;
    let options = q.options.as_ref().map(|o| serde_json::to_string(o).unwrap_or_default());
    conn.execute(
        "INSERT INTO quiz_questions (id, kp_id, type, question, options, answer, explanation) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
        rusqlite::params![q.id, q.kp_id, q.question_type, q.question, options, q.answer, q.explanation],
    ).map_err(crate::error::internal)?;
    Ok(())
}

pub fn get_questions_by_kp(db: &Database, kp_id: &str) -> Result<Vec<QuizQuestion>, String> {
    let conn = db.conn.lock().map_err(crate::error::internal)?;
    let mut stmt = conn
        .prepare("SELECT id, kp_id, type, question, options, answer, explanation FROM quiz_questions WHERE kp_id = ?1")
        .map_err(crate::error::internal)?;
    let questions = crate::repo::rows(
        stmt.query_map([kp_id], |row| question_from_row(row)),
    )?;
    Ok(questions)
}

pub fn get_questions_by_ids(db: &Database, ids: &[String]) -> Result<Vec<QuizQuestion>, String> {
    if ids.is_empty() { return Ok(vec![]); }
    let conn = db.conn.lock().map_err(crate::error::internal)?;
    let placeholders: Vec<String> = ids.iter().enumerate().map(|(i, _)| format!("?{}", i + 1)).collect();
    let sql = format!(
        "SELECT id, kp_id, type, question, options, answer, explanation FROM quiz_questions WHERE id IN ({})",
        placeholders.join(",")
    );
    let mut stmt = conn.prepare(&sql).map_err(crate::error::internal)?;
    let params: Vec<&dyn rusqlite::types::ToSql> = ids.iter().map(|id| id as &dyn rusqlite::types::ToSql).collect();
    let questions = crate::repo::rows(
        stmt.query_map(params.as_slice(), |row| question_from_row(row)),
    )?;
    Ok(questions)
}

pub fn record_attempt(db: &Database, req: &SubmitQuizAnswerRequest, is_correct: bool) -> Result<QuizAttempt, String> {
    let conn = db.conn.lock().map_err(crate::error::internal)?;
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
    ).map_err(crate::error::internal)?;
    Ok(attempt)
}

pub fn latest_attempt(db: &Database, question_id: &str) -> Result<Option<QuizAttempt>, String> {
    let conn = db.conn.lock().map_err(crate::error::internal)?;
    let mut stmt = conn
        .prepare(
            "SELECT id, question_id, user_answer, is_correct, attempted_at
             FROM quiz_attempts WHERE question_id = ?1
             ORDER BY attempted_at DESC LIMIT 1",
        )
        .map_err(crate::error::internal)?;
    let rows = stmt
        .query_map([question_id], |row| attempt_from_row(row))
        .map_err(crate::error::internal)?;
    crate::repo::one(rows)
}

pub fn get_attempts_by_kp(db: &Database, kp_id: &str) -> Result<Vec<QuizAttempt>, String> {
    let conn = db.conn.lock().map_err(crate::error::internal)?;
    let mut stmt = conn
        .prepare("SELECT qa.id, qa.question_id, qa.user_answer, qa.is_correct, qa.attempted_at FROM quiz_attempts qa JOIN quiz_questions qq ON qa.question_id = qq.id WHERE qq.kp_id = ?1 ORDER BY qa.attempted_at DESC")
        .map_err(crate::error::internal)?;
    let attempts = crate::repo::rows(
        stmt.query_map([kp_id], |row| attempt_from_row(row)),
    )?;
    Ok(attempts)
}

/// Compare a user's answer with the expected one.
///
/// Two passes, and the order matters. Anything that is plainly a number on both
/// sides is compared numerically, so `1.5` and `15` are no longer the same answer
/// (the old single text pass stripped `.` and `,` unconditionally, which made
/// `1.5` normalize to `15` and `3,14` to `314` — a wrong answer could be marked
/// correct, and a correct one written differently marked wrong).
///
/// Everything else falls back to a text comparison. Only edge punctuation and
/// whitespace are ignored there: stripping characters from the middle of a token
/// is what destroyed the decimal point, and it also makes distracting matches
/// like `A: 细胞壁` → `细胞壁`, which is not a distinction a quiz should blur.
pub fn answers_match(user: &str, expected: &str) -> bool {
    if let (Some(a), Some(b)) = (parse_number(user), parse_number(expected)) {
        return numbers_equal(a, b);
    }
    normalize_text(user) == normalize_text(expected)
}

/// A parsed answer number, keeping integers exact.
///
/// `1.5` and `15` must differ but `1.5` and `1.50` must not, so integers are
/// compared as integers rather than being widened to `f64` (where a long digit
/// string could lose the last digits and wrongly match).
#[derive(Debug, Clone, Copy)]
enum Num {
    Int(i128),
    Float(f64),
}

fn numbers_equal(a: Num, b: Num) -> bool {
    match (a, b) {
        (Num::Int(x), Num::Int(y)) => x == y,
        (Num::Int(x), Num::Float(y)) => x as f64 == y,
        (Num::Float(x), Num::Int(y)) => x == y as f64,
        (Num::Float(x), Num::Float(y)) => x == y,
    }
}

/// Parse an answer that is plainly a number, or `None`.
///
/// Deliberately strict: `str::parse::<f64>` also accepts `inf`, `NaN` and
/// exponent notation, none of which anyone means as a quiz answer. Anything
/// containing another character (a unit, a `%`, a word) returns `None` so the
/// text comparison sees it.
fn parse_number(input: &str) -> Option<Num> {
    // A Chinese IME can produce full-width digits and symbols.
    let mut ascii = String::with_capacity(input.len());
    for c in input.chars() {
        match c {
            '０'..='９' => ascii.push(char::from(b'0' + (c as u32 - '０' as u32) as u8)),
            '．' => ascii.push('.'),
            '，' => ascii.push(','),
            '－' => ascii.push('-'),
            '＋' => ascii.push('+'),
            '0'..='9' | '.' | ',' | '-' | '+' => ascii.push(c),
            c if c.is_whitespace() => {}
            _ => return None,
        }
    }
    if ascii.is_empty() {
        return None;
    }

    let (negative, rest) = match ascii.strip_prefix('-') {
        Some(r) => (true, r),
        None => (false, ascii.strip_prefix('+').unwrap_or(&ascii)),
    };
    let (int_part, frac_part) = match rest.split_once('.') {
        Some((i, f)) => (i, Some(f)),
        None => (rest, None),
    };

    // Commas are only a thousands separator in `1,000` / `12,345,678` form.
    // `3,14` does not qualify, so it falls through to the text comparison
    // instead of silently becoming 314.
    let digits = if int_part.contains(',') {
        let mut groups = int_part.split(',');
        let first = groups.next()?;
        if first.is_empty() || first.len() > 3 || !first.chars().all(|c| c.is_ascii_digit()) {
            return None;
        }
        let mut joined = first.to_string();
        for group in groups {
            if group.len() != 3 || !group.chars().all(|c| c.is_ascii_digit()) {
                return None;
            }
            joined.push_str(group);
        }
        joined
    } else {
        int_part.to_string()
    };

    if let Some(frac) = frac_part {
        if frac.contains('.') || !frac.chars().all(|c| c.is_ascii_digit()) {
            return None;
        }
    }

    let has_digits = digits.chars().any(|c| c.is_ascii_digit())
        || frac_part.is_some_and(|f| f.chars().any(|c| c.is_ascii_digit()));
    if !has_digits {
        // `-`, `+`, `.`, `,` alone are punctuation, not a number.
        return None;
    }

    let sign = if negative { "-" } else { "" };
    match frac_part {
        // Integer text is kept exact.
        None => format!("{sign}{digits}").parse::<i128>().ok().map(Num::Int),
        Some(frac) => format!("{sign}{digits}.{frac}").parse::<f64>().ok().map(Num::Float),
    }
}

/// Punctuation that carries no meaning at the edge of a free-text answer.
fn is_edge_punctuation(c: char) -> bool {
    matches!(
        c,
        ',' | '.' | '!' | '?' | ';' | ':' | '"' | '\'' | '`'
            | '，' | '。' | '！' | '？' | '；' | '：' | '“' | '”' | '‘' | '’'
            | '(' | ')' | '（' | '）' | '[' | ']' | '【' | '】'
            | '、' | '·' | '…'
    )
}

/// Normalize a free-text answer: ignore case, whitespace, and *edge* punctuation.
fn normalize_text(s: &str) -> String {
    let lowered: String = s
        .chars()
        .filter(|c| !c.is_whitespace())
        .flat_map(|c| c.to_lowercase())
        .collect();
    lowered.trim_matches(is_edge_punctuation).to_string()
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn decimals_are_not_stripped_into_integers() {
        // The regression: `.` used to be removed, so both of these normalized to "15".
        assert!(!answers_match("15", "1.5"));
        assert!(!answers_match("1.5", "15"));
    }

    #[test]
    fn equivalent_decimal_spellings_match() {
        assert!(answers_match("1.5", "1.50"));
        assert!(answers_match("0.5", ".5"));
        assert!(answers_match("-2.5", "-2.50"));
    }

    #[test]
    fn integers_compare_exactly() {
        assert!(answers_match("1,000", "1000"));
        assert!(answers_match("12,345,678", "12345678"));
        assert!(answers_match("1 000", "1000"));
        assert!(!answers_match("1000", "1001"));
    }

    #[test]
    fn comma_decimal_is_not_a_thousands_separator() {
        // `3,14` (European decimal) must not be read as 314.
        assert!(!answers_match("3,14", "314"));
        assert!(!answers_match("3,14", "3.14"));
    }

    #[test]
    fn mixed_text_and_numbers_use_text_comparison() {
        assert!(answers_match("约1.5", "约1.5"));
        assert!(!answers_match("约15", "约1.5"));
        assert!(answers_match("50%", "50%"));
    }

    #[test]
    fn edge_punctuation_does_not_break_text_answers() {
        assert!(answers_match("\"细胞壁\"", "细胞壁"));
        assert!(answers_match("细胞壁。", "细胞壁"));
        assert!(answers_match("Cell Wall!", "cell wall"));
        assert!(!answers_match("A 细胞壁", "细胞壁"));
    }

    #[test]
    fn full_width_digits_are_recognized() {
        assert!(answers_match("１．５", "1.5"));
        assert!(!answers_match("１５", "1.5"));
    }

    #[test]
    fn non_finite_text_is_not_a_number() {
        assert!(answers_match("inf", "inf"));
        assert!(!answers_match("inf", "Infinity"));
        assert!(answers_match("nan", "NaN"));
        assert!(!answers_match("1e3", "1000"));
    }
}
