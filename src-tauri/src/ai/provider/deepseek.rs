//! DeepSeek Chat Completions adapter
//! Docs: https://api-docs.deepseek.com/zh-cn/api/create-chat-completion

use async_trait::async_trait;
use reqwest::Client;
use serde::{Deserialize, Serialize};

use super::{ChatOptions, LlmConfig, LlmProvider, ThinkingMode};

#[derive(Debug, Serialize)]
struct ChatMessage {
    role: String,
    content: String,
}

#[derive(Debug, Serialize)]
struct ThinkingParam {
    #[serde(rename = "type")]
    thinking_type: String,
}

#[derive(Debug, Serialize)]
struct ResponseFormat {
    #[serde(rename = "type")]
    format_type: String,
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
    #[serde(skip_serializing_if = "Option::is_none")]
    thinking: Option<ThinkingParam>,
    #[serde(skip_serializing_if = "Option::is_none")]
    reasoning_effort: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    response_format: Option<ResponseFormat>,
}

#[derive(Debug, Deserialize)]
struct ChatChoice {
    message: ChatMessageContent,
}

#[derive(Debug, Deserialize)]
struct ChatMessageContent {
    content: Option<String>,
    #[serde(default)]
    reasoning_content: Option<String>,
}

#[derive(Debug, Deserialize)]
struct ChatResponse {
    choices: Vec<ChatChoice>,
}

pub struct DeepSeekClient {
    config: LlmConfig,
    http: Client,
}

impl DeepSeekClient {
    pub fn new(config: LlmConfig) -> Self {
        let http = Client::builder()
            .timeout(std::time::Duration::from_secs(90))
            .connect_timeout(std::time::Duration::from_secs(15))
            .build()
            .unwrap_or_else(|_| Client::new());
        Self { config, http }
    }

    fn completions_url(&self) -> String {
        let base = self.config.base_url.trim_end_matches('/');
        if base.ends_with("/chat/completions") {
            base.to_string()
        } else if base.ends_with("/v1") {
            format!("{}/chat/completions", base)
        } else {
            format!("{}/chat/completions", base)
        }
    }

    fn friendly_http_error(status: u16, body: &str) -> String {
        let lower = body.to_lowercase();
        if status == 401 || status == 403 || lower.contains("invalid api key") || lower.contains("unauthorized") {
            return "AUTH_ERROR: API Key 无效或权限不足，请在设置中检查 DeepSeek 密钥".into();
        }
        if status == 429 || lower.contains("rate limit") || lower.contains("quota") {
            return "QUOTA_ERROR: DeepSeek 请求过于频繁或额度不足，请稍后再试".into();
        }
        if status == 400 && (lower.contains("model") || lower.contains("not found")) {
            return "MODEL_ERROR: 模型不可用。请使用 deepseek-v4-flash 或 deepseek-v4-pro".into();
        }
        if status >= 500 {
            return format!("PROVIDER_ERROR: DeepSeek 服务暂时不可用（HTTP {}）", status);
        }
        let snippet: String = body.chars().take(160).collect();
        format!(
            "LLM_ERROR: DeepSeek 请求失败（HTTP {}）{}",
            status,
            if snippet.is_empty() {
                String::new()
            } else {
                format!(": {}", snippet)
            }
        )
    }

    fn build_request(&self, messages: Vec<ChatMessage>, stream: bool, options: &ChatOptions) -> ChatRequest {
        let thinking = Some(ThinkingParam {
            thinking_type: match options.thinking {
                ThinkingMode::Enabled => "enabled".into(),
                ThinkingMode::Disabled | ThinkingMode::Auto => "disabled".into(),
            },
        });
        let reasoning_effort = if options.thinking == ThinkingMode::Enabled {
            Some(options.reasoning_effort.unwrap_or("high").to_string())
        } else {
            None
        };

        ChatRequest {
            model: self.config.model.clone(),
            messages,
            stream,
            temperature: Some(self.config.temperature),
            max_tokens: Some(self.config.max_tokens),
            thinking,
            reasoning_effort,
            response_format: if options.json_object {
                Some(ResponseFormat {
                    format_type: "json_object".into(),
                })
            } else {
                None
            },
        }
    }

    fn extract_text(choice: &ChatChoice) -> Result<String, String> {
        if let Some(content) = choice
            .message
            .content
            .as_ref()
            .map(|s| s.trim())
            .filter(|s| !s.is_empty())
        {
            return Ok(content.to_string());
        }
        if let Some(reasoning) = choice
            .message
            .reasoning_content
            .as_ref()
            .map(|s| s.trim())
            .filter(|s| !s.is_empty())
        {
            return Ok(reasoning.to_string());
        }
        Err("Empty response from DeepSeek".into())
    }
}

#[async_trait]
impl LlmProvider for DeepSeekClient {
    async fn chat_with_options(
        &self,
        system_prompt: &str,
        user_prompt: &str,
        options: ChatOptions,
    ) -> Result<String, String> {
        let messages = vec![
            ChatMessage {
                role: "system".into(),
                content: system_prompt.to_string(),
            },
            ChatMessage {
                role: "user".into(),
                content: user_prompt.to_string(),
            },
        ];
        let req = self.build_request(messages, false, &options);

        let resp = self
            .http
            .post(self.completions_url())
            .header("Content-Type", "application/json")
            .header("Authorization", format!("Bearer {}", self.config.api_key))
            .json(&req)
            .send()
            .await
            .map_err(|e| format!("NETWORK_ERROR: 无法连接 DeepSeek（{}）", e))?;

        let status = resp.status();
        let text = resp.text().await.map_err(|e| format!("Read error: {}", e))?;
        if !status.is_success() {
            return Err(Self::friendly_http_error(status.as_u16(), &text));
        }

        let body: ChatResponse = serde_json::from_str(&text).map_err(|e| {
            format!(
                "Parse error: {} | body: {}",
                e,
                text.chars().take(200).collect::<String>()
            )
        })?;
        let choice = body
            .choices
            .first()
            .ok_or_else(|| "Empty response from DeepSeek".to_string())?;
        Self::extract_text(choice)
    }
}
