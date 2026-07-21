use crate::ai::llm::LlmClient;
use crate::models::{CreateKnowledgePointRequest, KnowledgePoint};

pub async fn extract_knowledge_points(
    llm: &LlmClient,
    source_title: &str,
    source_content: &str,
) -> Result<Vec<CreateKnowledgePointRequest>, String> {
    let system_prompt = "You are a knowledge extraction assistant. Extract key concepts as structured knowledge points from the given content. Return JSON only.";
    let user_prompt = format!(
        "Extract the main knowledge points from this content.\n\
        Title: {}\n\nContent:\n{}\n\n\
        Return ONLY a JSON array of objects with fields: title, summary (one sentence), content (detailed explanation 2-3 paragraphs), tags (array of strings).",
        source_title, source_content
    );

    let response = llm.chat(system_prompt, &user_prompt).await?;
    // Extract JSON from response (may be wrapped in markdown code block)
    let json_str = if let Some(start) = response.find("```json") {
        let after = &response[start + 7..];
        if let Some(end) = after.find("```") {
            &after[..end]
        } else {
            after
        }
    } else if let Some(start) = response.find('[') {
        &response[start..]
    } else {
        &response
    };

    let kps: Vec<CreateKnowledgePointRequest> = serde_json::from_str(json_str.trim())
        .map_err(|e| format!("Failed to parse knowledge points: {}. Raw: {}", e, json_str))?;
    Ok(kps)
}
