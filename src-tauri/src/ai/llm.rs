use reqwest::Client;
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize)]
struct ChatMessage {
    role: String,
    content: String,
}

#[derive(Debug, Serialize)]
struct ChatRequest {
    model: String,
    messages: Vec<ChatMessage>,
    stream: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    temperature: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    max_tokens: Option<i32>,
}

#[derive(Debug, Deserialize)]
struct ChatChoice {
    message: ChatMessageContent,
}

#[derive(Debug, Deserialize)]
struct ChatMessageContent {
    content: String,
}

#[derive(Debug, Deserialize)]
struct ChatResponse {
    choices: Vec<ChatChoice>,
}

#[derive(Debug, Clone)]
pub struct LlmConfig {
    pub base_url: String,
    pub api_key: String,
    pub model: String,
    pub temperature: f64,
    pub max_tokens: i32,
    pub api_format: String,
}

pub struct LlmClient {
    config: LlmConfig,
    client: Client,
}

impl LlmClient {
    pub fn new(config: LlmConfig) -> Self {
        Self { config, client: Client::new() }
    }

    /// 发送非流式请求，返回完整回复
    pub async fn chat(&self, system_prompt: &str, user_prompt: &str) -> Result<String, String> {
        let messages = vec![
            ChatMessage { role: "system".to_string(), content: system_prompt.to_string() },
            ChatMessage { role: "user".to_string(), content: user_prompt.to_string() },
        ];
        let req = ChatRequest {
            model: self.config.model.clone(),
            messages,
            stream: false,
            temperature: Some(self.config.temperature),
            max_tokens: Some(self.config.max_tokens),
        };

        let resp = self.client
            .post(format!("{}/chat/completions", self.config.base_url))
            .header("Authorization", format!("Bearer {}", self.config.api_key))
            .json(&req)
            .send()
            .await
            .map_err(|e| format!("LLM request failed: {}", e))?;

        let body: ChatResponse = resp.json().await.map_err(|e| format!("Parse error: {}", e))?;
        body.choices
            .first()
            .map(|c| c.message.content.clone())
            .ok_or_else(|| "Empty response".to_string())
    }

    /// 发送流式请求，对每个 chunk 调用 on_chunk 回调
    pub async fn chat_streaming<F>(&self, system_prompt: &str, user_prompt: &str, on_chunk: F) -> Result<String, String>
    where
        F: Fn(&str),
    {
        let messages = vec![
            ChatMessage { role: "system".to_string(), content: system_prompt.to_string() },
            ChatMessage { role: "user".to_string(), content: user_prompt.to_string() },
        ];
        let req = ChatRequest {
            model: self.config.model.clone(),
            messages,
            stream: true,
            temperature: Some(self.config.temperature),
            max_tokens: Some(self.config.max_tokens),
        };
        let mut full_response = String::new();

        let resp = self.client
            .post(format!("{}/chat/completions", self.config.base_url))
            .header("Authorization", format!("Bearer {}", self.config.api_key))
            .json(&req)
            .send()
            .await
            .map_err(|e| format!("LLM request failed: {}", e))?;

        use futures_util::StreamExt;
        let mut stream = resp.bytes_stream();
        while let Some(chunk) = stream.next().await {
            let chunk = chunk.map_err(|e| format!("Stream error: {}", e))?;
            let text = String::from_utf8_lossy(&chunk);
            for line in text.lines() {
                if let Some(data) = line.strip_prefix("data: ") {
                    if data == "[DONE]" { continue; }
                    if let Ok(event) = serde_json::from_str::<serde_json::Value>(data) {
                        if let Some(content) = event["choices"][0]["delta"]["content"].as_str() {
                            full_response.push_str(content);
                            on_chunk(content);
                        }
                    }
                }
            }
        }
        Ok(full_response)
    }
}
