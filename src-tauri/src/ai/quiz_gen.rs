use crate::ai::{create_provider, extract_json_payload, LlmConfig};
use crate::models::QuizQuestion;
use serde::Deserialize;

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

pub async fn generate_quizzes(
    config: LlmConfig,
    kp_title: &str,
    kp_content: &str,
    count: usize,
) -> Result<Vec<QuizQuestion>, String> {
    let system_prompt = "You are a quiz generation assistant for spaced-repetition learning. Always reply with a valid JSON object only.";
    let user_prompt = format!(
        "为以下知识点生成 {count} 道测验题。\n\
Title: {kp_title}\nContent:\n{kp_content}\n\n\
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

    let questions = drafts
        .into_iter()
        .map(|d| QuizQuestion {
            id: crate::models::new_id(),
            kp_id: String::new(),
            question_type: d.question_type,
            question: d.question,
            options: d.options,
            answer: d.answer,
            explanation: d.explanation,
        })
        .collect();
    Ok(questions)
}
