use crate::ai::{create_provider, extract_json_payload, LlmConfig};
use crate::models::CreateKnowledgePointRequest;

pub async fn extract_knowledge_points(
    config: LlmConfig,
    source_title: &str,
    source_content: &str,
) -> Result<Vec<CreateKnowledgePointRequest>, String> {
    let system_prompt = "You are a knowledge extraction assistant. Always reply with a valid JSON object only.";
    let user_prompt = format!(
        "从以下内容提取主要知识点。\n\
Title: {}\n\nContent:\n{}\n\n\
返回 JSON 对象，格式严格为：\
{{\"items\":[{{\"title\":\"...\",\"summary\":\"一句话\",\"content\":\"2-3段详细说明\",\"tags\":[\"...\"]}}]}}",
        source_title, source_content
    );

    let llm = create_provider(config)?;
    let response = llm.chat_json(system_prompt, &user_prompt).await?;
    let json_str = extract_json_payload(&response);

    #[derive(serde::Deserialize)]
    struct KpEnvelope {
        items: Vec<CreateKnowledgePointRequest>,
    }

    if let Ok(env) = serde_json::from_str::<KpEnvelope>(json_str) {
        return Ok(env.items);
    }

    serde_json::from_str(json_str.trim())
        .map_err(|e| format!("Failed to parse knowledge points: {}. Raw: {}", e, json_str))
}
