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
        "analysis" | "分析题" => Some("analysis"),
        _ => None,
    }
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
选择题必须提供 options（4项）；填空题 options 可为 null 或省略。"
    );

    let llm = create_provider(config)?;
    let response = llm.chat_json(system_prompt, &user_prompt).await?;
    let json_str = extract_json_payload(&response);

    let drafts: Vec<QuizDraft> = if let Ok(env) = serde_json::from_str::<QuizEnvelope>(json_str) {
        env.items
    } else {
        serde_json::from_str(json_str.trim())
            .map_err(|e| format!("Failed to parse quizzes: {}. Raw: {}", e, json_str))?
    };

    let questions: Vec<QuizQuestion> = drafts
        .into_iter()
        .filter_map(|d| {
            let question_type = normalize_question_type(&d.question_type)?.to_string();
            Some(QuizQuestion {
                id: crate::models::new_id(),
                kp_id: String::new(),
                question_type,
                question: d.question,
                options: d.options,
                answer: d.answer,
                explanation: d.explanation,
            })
        })
        .collect();

    if questions.is_empty() {
        return Err("模型未返回有效题型（仅支持 multiple_choice / fill_blank / analysis）".into());
    }
    Ok(questions)
}
