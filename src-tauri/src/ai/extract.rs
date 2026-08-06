use crate::ai::{create_provider, extract_json_payload, truncate_chars, LlmConfig};
use crate::models::CreateKnowledgePointRequest;
use serde::Deserialize;

const MAX_CONTENT_CHARS: usize = 20_000;

#[derive(Debug, Deserialize)]
pub struct SourceDraft {
    pub title: String,
    pub description: String,
}

#[derive(Debug, Deserialize)]
struct SourceEnvelope {
    items: Vec<SourceDraft>,
}

#[derive(Debug, Deserialize)]
struct KpEnvelope {
    items: Vec<CreateKnowledgePointRequest>,
}

/// Ask the model to propose learning sources for a topic.
pub async fn propose_sources(config: LlmConfig, topic: &str) -> Result<Vec<SourceDraft>, String> {
    let system_prompt = "You are a research assistant. Always reply with a valid JSON object only.";
    let user_prompt = truncate_chars(
        &format!(
            "用户想学习主题：{}。\n\
请列出 3 份高质量学习资料。\n\
返回 JSON 对象，格式严格为：{{\"items\":[{{\"title\":\"...\",\"description\":\"...\"}}]}}",
            topic
        ),
        MAX_CONTENT_CHARS,
    );

    let llm = create_provider(config)?;
    let response = llm.chat_json(system_prompt, &user_prompt).await?;
    let json_str = extract_json_payload(&response);

    if let Ok(env) = serde_json::from_str::<SourceEnvelope>(json_str) {
        return Ok(env.items);
    }
    serde_json::from_str(json_str.trim()).map_err(|e| {
        format!(
            "Failed to parse sources: {}. Raw: {}",
            e,
            json_str.chars().take(200).collect::<String>()
        )
    })
}

/// Extract knowledge points from source material about a topic.
pub async fn extract_knowledge_points(
    config: LlmConfig,
    topic: &str,
    content: &str,
) -> Result<Vec<CreateKnowledgePointRequest>, String> {
    let system_prompt =
        "You are a knowledge extraction assistant. Always reply with a valid JSON object only.";
    let body = truncate_chars(content, MAX_CONTENT_CHARS);
    let user_prompt = format!(
        "从以下关于「{topic}」的内容中提取主要知识点。\n\n{body}\n\n\
返回 JSON 对象，格式严格为：\
{{\"items\":[{{\"title\":\"...\",\"summary\":\"一句话\",\"content\":\"2-3段详细说明\",\"tags\":[\"...\"]}}]}}"
    );

    let llm = create_provider(config)?;
    let response = llm.chat_json(system_prompt, &user_prompt).await?;
    let json_str = extract_json_payload(&response);

    if let Ok(env) = serde_json::from_str::<KpEnvelope>(json_str) {
        return Ok(env.items);
    }

    serde_json::from_str(json_str.trim())
        .map_err(|e| format!("Failed to parse knowledge points: {}. Raw: {}", e, json_str))
}
