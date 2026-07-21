use crate::ai::llm::LlmClient;
use crate::models::QuizQuestion;

pub async fn generate_quizzes(
    llm: &LlmClient,
    kp_title: &str,
    kp_content: &str,
    count: usize,
) -> Result<Vec<QuizQuestion>, String> {
    let id_prefix = crate::models::new_id(); // placeholder, will be replaced by caller

    let system_prompt = "You are a quiz generation assistant for spaced-repetition learning. Generate challenging multiple-choice and fill-in-the-blank questions that test true understanding, not memorization. Return JSON only.";
    let user_prompt = format!(
        "Generate {} quiz questions for this knowledge point.\n\
        Title: {}\nContent:\n{}\n\n\
        Return ONLY a JSON array of objects with fields:\n\
        - type: 'multiple_choice' or 'fill_blank'\n\
        - question: the question text\n\
        - options: (array of 4 strings, only for multiple_choice)\n\
        - answer: the correct answer text (single letter A/B/C/D for multiple_choice, the word for fill_blank)\n\
        - explanation: why this answer is correct, 1-2 sentences",
        count, kp_title, kp_content
    );

    let response = llm.chat(system_prompt, &user_prompt).await?;
    let json_str = if let Some(start) = response.find("```json") {
        let after = &response[start + 7..];
        if let Some(end) = after.find("```") { &after[..end] } else { after }
    } else if let Some(start) = response.find('[') {
        &response[start..]
    } else {
        &response
    };

    let mut questions: Vec<QuizQuestion> = serde_json::from_str(json_str.trim())
        .map_err(|e| format!("Failed to parse quizzes: {}. Raw: {}", e, json_str))?;

    // Set proper IDs
    for q in &mut questions {
        q.id = crate::models::new_id();
    }
    Ok(questions)
}
