use crate::ai::{create_provider, extract_json_payload, truncate_chars, LlmConfig};
use crate::models::QuizQuestion;
use serde::Deserialize;

const MAX_CONTENT_CHARS: usize = 20_000;

#[derive(Debug, Deserialize)]
struct QuizDraft {
    #[serde(rename = "type")]
    question_type: String,
    question: String,
    #[serde(default)]
    options: Option<Vec<String>>,
    answer: String,
    #[serde(default)]
    explanation: String,
}

#[derive(Debug, Deserialize)]
struct QuizEnvelope {
    items: Vec<QuizDraft>,
}

fn normalize_question_type(raw: &str) -> Option<&'static str> {
    match raw.trim().to_ascii_lowercase().as_str() {
        "multiple_choice" | "mcq" | "choice" | "选择题" => Some("multiple_choice"),
        "fill_blank" | "fill_in_blank" | "blank" | "填空" | "填空题" => Some("fill_blank"),
        _ => None,
    }
}

fn option_matches_answer(option: &str, answer: &str) -> bool {
    option.trim() == answer.trim() || crate::repo::quiz::answers_match(option, answer)
}

fn draft_to_question(d: QuizDraft) -> Option<QuizQuestion> {
    let question_type = normalize_question_type(&d.question_type)?;
    let question = d.question.trim();
    let answer = d.answer.trim();
    if question.is_empty() || answer.is_empty() {
        return None;
    }

    let (options, answer) = match question_type {
        "multiple_choice" => {
            let opts: Vec<String> = d
                .options
                .unwrap_or_default()
                .into_iter()
                .map(|s| s.trim().to_string())
                .filter(|s| !s.is_empty())
                .collect();
            if opts.len() < 2 {
                return None;
            }
            let canonical = opts
                .iter()
                .find(|o| option_matches_answer(o, answer))?
                .clone();
            (Some(opts), canonical)
        }
        "fill_blank" => (None, answer.to_string()),
        _ => return None,
    };

    Some(QuizQuestion {
        id: crate::models::new_id(),
        kp_id: String::new(),
        question_type: question_type.to_string(),
        question: question.to_string(),
        options,
        answer,
        explanation: d.explanation,
    })
}

fn questions_from_payload(json_str: &str) -> Result<Vec<QuizQuestion>, String> {
    let drafts: Vec<QuizDraft> = if let Ok(env) = serde_json::from_str::<QuizEnvelope>(json_str) {
        env.items
    } else {
        serde_json::from_str(json_str.trim()).map_err(|e| {
            crate::error::internal(format!(
                "Failed to parse quizzes: {}. Raw: {}",
                e, json_str
            ))
        })?
    };

    let questions: Vec<QuizQuestion> = drafts.into_iter().filter_map(draft_to_question).collect();
    if questions.is_empty() {
        return Err(
            "模型未返回有效题目（选择题须含选项且答案必须是其中一项，不支持分析题）".into(),
        );
    }
    Ok(questions)
}

pub async fn generate_quizzes(
    config: LlmConfig,
    kp_title: &str,
    kp_content: &str,
    count: usize,
) -> Result<Vec<QuizQuestion>, String> {
    let count = count.clamp(1, 10);
    let content = truncate_chars(kp_content, MAX_CONTENT_CHARS);
    let system_prompt = "You are a quiz generation assistant for spaced-repetition learning. Always reply with a valid JSON object only.";
    let user_prompt = format!(
        "为以下知识点生成 {count} 道测验题。\n\
Title: {kp_title}\nContent:\n{content}\n\n\
返回 JSON 对象，格式严格为：\
{{\"items\":[{{\
\"type\":\"multiple_choice或fill_blank\",\
\"question\":\"题目\",\
\"options\":[\"A\",\"B\",\"C\",\"D\"],\
\"answer\":\"正确答案（选择题为选项原文，填空为词语）\",\
\"explanation\":\"1-2句解析\"\
}}]}}\n\
选择题必须提供 options（4项），且 answer 必须是 options 中的一项；填空题不要 options。不要生成分析题。"
    );

    let llm = create_provider(config)?;
    let response = llm.chat_json(system_prompt, &user_prompt).await?;
    let json_str = extract_json_payload(&response);
    questions_from_payload(json_str)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn payload(items: &str) -> String {
        format!(r#"{{"items":[{items}]}}"#)
    }

    #[test]
    fn multiple_choice_keeps_a_matching_option_as_the_answer() {
        let json = payload(
            r#"{"type":"multiple_choice","question":"1+1?","options":["1","2","3","4"],"answer":"2","explanation":"e"}"#,
        );
        let qs = questions_from_payload(&json).unwrap();
        assert_eq!(qs.len(), 1);
        assert_eq!(qs[0].question_type, "multiple_choice");
        assert_eq!(qs[0].answer, "2");
        assert_eq!(qs[0].options.as_ref().unwrap().len(), 4);
    }

    #[test]
    fn multiple_choice_without_options_is_dropped() {
        let json = payload(
            r#"{"type":"multiple_choice","question":"1+1?","answer":"2","explanation":"e"}"#,
        );
        let err = questions_from_payload(&json).unwrap_err();
        assert!(err.contains("有效题目"), "got: {err}");
    }

    #[test]
    fn multiple_choice_answer_outside_options_is_dropped() {
        let json = payload(
            r#"{"type":"multiple_choice","question":"1+1?","options":["1","3","4","5"],"answer":"2","explanation":"e"}"#,
        );
        let err = questions_from_payload(&json).unwrap_err();
        assert!(err.contains("有效题目"), "got: {err}");
    }

    #[test]
    fn analysis_items_are_dropped() {
        let json = payload(
            r#"{"type":"analysis","question":"discuss","answer":"long essay","explanation":"e"}"#,
        );
        let err = questions_from_payload(&json).unwrap_err();
        assert!(err.contains("有效题目"), "got: {err}");
    }

    #[test]
    fn fill_blank_strips_options_and_survives() {
        let json = payload(
            r#"{"type":"fill_blank","question":"capital?","options":["Paris"],"answer":"Paris","explanation":"e"}"#,
        );
        let qs = questions_from_payload(&json).unwrap();
        assert_eq!(qs.len(), 1);
        assert_eq!(qs[0].question_type, "fill_blank");
        assert!(qs[0].options.is_none());
        assert_eq!(qs[0].answer, "Paris");
    }

    #[test]
    fn a_mixed_batch_keeps_only_valid_items() {
        let json = payload(
            r#"{"type":"analysis","question":"x","answer":"y","explanation":""},{"type":"fill_blank","question":"q","answer":"a","explanation":""},{"type":"multiple_choice","question":"q","options":["a","b"],"answer":"z","explanation":""}"#,
        );
        let qs = questions_from_payload(&json).unwrap();
        assert_eq!(qs.len(), 1);
        assert_eq!(qs[0].question_type, "fill_blank");
    }
}
