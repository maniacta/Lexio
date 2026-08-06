use async_trait::async_trait;
use serde::Serialize;

pub mod deepseek;

/// Known vendor kinds — each has (or will have) its own adapter.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProviderKind {
    DeepSeek,
    OpenAi,
    Anthropic,
}

impl ProviderKind {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::DeepSeek => "deepseek",
            Self::OpenAi => "openai",
            Self::Anthropic => "anthropic",
        }
    }

    pub fn parse(s: &str) -> Option<Self> {
        match s.trim().to_ascii_lowercase().as_str() {
            "deepseek" => Some(Self::DeepSeek),
            "openai" | "openai_compatible" => Some(Self::OpenAi),
            "anthropic" => Some(Self::Anthropic),
            _ => None,
        }
    }

    pub fn display_name(self) -> &'static str {
        match self {
            Self::DeepSeek => "DeepSeek",
            Self::OpenAi => "OpenAI",
            Self::Anthropic => "Anthropic",
        }
    }

    pub fn default_base_url(self) -> &'static str {
        match self {
            Self::DeepSeek => "https://api.deepseek.com",
            Self::OpenAi => "https://api.openai.com/v1",
            Self::Anthropic => "https://api.anthropic.com",
        }
    }

    /// Hosts allowed for this vendor — blocks SSRF via arbitrary base_url.
    pub fn allowed_hosts(self) -> &'static [&'static str] {
        match self {
            Self::DeepSeek => &["api.deepseek.com"],
            Self::OpenAi => &["api.openai.com"],
            Self::Anthropic => &["api.anthropic.com"],
        }
    }

    /// Validate and normalize base_url. Empty input → official default.
    /// Rejects non-HTTPS and hosts outside the vendor allowlist.
    pub fn normalize_base_url(self, input: &str) -> Result<String, String> {
        let trimmed = input.trim();
        if trimmed.is_empty() {
            return Ok(self.default_base_url().to_string());
        }

        let parsed = reqwest::Url::parse(trimmed)
            .map_err(|_| "Base URL 格式无效，请使用官方 HTTPS 地址".to_string())?;
        if parsed.scheme() != "https" {
            return Err("Base URL 必须使用 HTTPS".into());
        }
        let host = parsed
            .host_str()
            .ok_or_else(|| "Base URL 缺少主机名".to_string())?;
        if !self.allowed_hosts().iter().any(|h| *h == host) {
            return Err(format!(
                "「{}」仅允许官方地址（{}），当前主机「{}」不被接受",
                self.display_name(),
                self.default_base_url(),
                host
            ));
        }
        // Canonicalize to the known official endpoint (drops path/query tricks).
        Ok(self.default_base_url().to_string())
    }

    pub fn is_implemented(self) -> bool {
        matches!(self, Self::DeepSeek)
    }

    pub fn default_models(self) -> &'static [ModelPreset] {
        match self {
            Self::DeepSeek => &[
                ModelPreset {
                    model_name: "deepseek-v4-flash",
                    temperature: 0.7,
                    max_tokens: 4096,
                    is_default: true,
                },
                ModelPreset {
                    model_name: "deepseek-v4-pro",
                    temperature: 0.7,
                    max_tokens: 8192,
                    is_default: false,
                },
            ],
            Self::OpenAi => &[ModelPreset {
                model_name: "gpt-4o",
                temperature: 0.7,
                max_tokens: 4096,
                is_default: true,
            }],
            Self::Anthropic => &[ModelPreset {
                model_name: "claude-sonnet-4-20250514",
                temperature: 0.7,
                max_tokens: 4096,
                is_default: true,
            }],
        }
    }

    pub fn all() -> &'static [ProviderKind] {
        &[Self::DeepSeek, Self::OpenAi, Self::Anthropic]
    }
}

#[derive(Debug, Clone, Copy)]
pub struct ModelPreset {
    pub model_name: &'static str,
    pub temperature: f64,
    pub max_tokens: i32,
    pub is_default: bool,
}

#[derive(Debug, Clone, Serialize)]
pub struct ProviderKindInfo {
    pub kind: String,
    pub display_name: String,
    pub default_base_url: String,
    pub implemented: bool,
    pub models: Vec<ProviderKindModelInfo>,
}

#[derive(Debug, Clone, Serialize)]
pub struct ProviderKindModelInfo {
    pub model_name: String,
    pub is_default: bool,
}

pub fn list_provider_kinds() -> Vec<ProviderKindInfo> {
    ProviderKind::all()
        .iter()
        .copied()
        .filter(|k| k.is_implemented())
        .map(|k| ProviderKindInfo {
            kind: k.as_str().to_string(),
            display_name: k.display_name().to_string(),
            default_base_url: k.default_base_url().to_string(),
            implemented: k.is_implemented(),
            models: k
                .default_models()
                .iter()
                .map(|m| ProviderKindModelInfo {
                    model_name: m.model_name.to_string(),
                    is_default: m.is_default,
                })
                .collect(),
        })
        .collect()
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ThinkingMode {
    Auto,
    Enabled,
    Disabled,
}

#[derive(Debug, Clone)]
pub struct ChatOptions {
    pub thinking: ThinkingMode,
    pub reasoning_effort: Option<&'static str>,
    pub json_object: bool,
}

impl Default for ChatOptions {
    fn default() -> Self {
        Self {
            thinking: ThinkingMode::Auto,
            reasoning_effort: None,
            json_object: false,
        }
    }
}

impl ChatOptions {
    pub fn for_chat() -> Self {
        Self {
            thinking: ThinkingMode::Disabled,
            reasoning_effort: None,
            json_object: false,
        }
    }

    pub fn for_json() -> Self {
        Self {
            thinking: ThinkingMode::Disabled,
            reasoning_effort: None,
            json_object: true,
        }
    }
}

#[derive(Debug, Clone)]
pub struct LlmConfig {
    pub kind: ProviderKind,
    pub base_url: String,
    pub api_key: String,
    pub model: String,
    pub temperature: f64,
    pub max_tokens: i32,
}

#[async_trait]
pub trait LlmProvider: Send + Sync {
    async fn chat(&self, system_prompt: &str, user_prompt: &str) -> Result<String, String> {
        self.chat_with_options(system_prompt, user_prompt, ChatOptions::for_chat())
            .await
    }

    async fn chat_json(&self, system_prompt: &str, user_prompt: &str) -> Result<String, String> {
        self.chat_with_options(system_prompt, user_prompt, ChatOptions::for_json())
            .await
    }

    async fn chat_with_options(
        &self,
        system_prompt: &str,
        user_prompt: &str,
        options: ChatOptions,
    ) -> Result<String, String>;
}

/// Build the vendor-specific adapter. Unimplemented kinds return a clear error.
pub fn create_provider(config: LlmConfig) -> Result<Box<dyn LlmProvider>, String> {
    match config.kind {
        ProviderKind::DeepSeek => Ok(Box::new(deepseek::DeepSeekClient::new(config))),
        ProviderKind::OpenAi => Err(
            "PROVIDER_NOT_IMPLEMENTED: OpenAI 适配器尚未接入，请改用已适配的 DeepSeek".into(),
        ),
        ProviderKind::Anthropic => Err(
            "PROVIDER_NOT_IMPLEMENTED: Anthropic 适配器尚未接入，请改用已适配的 DeepSeek".into(),
        ),
    }
}

pub fn extract_json_payload(response: &str) -> &str {
    let trimmed = response.trim();
    if let Some(start) = trimmed.find("```json") {
        let after = &trimmed[start + 7..];
        if let Some(end) = after.find("```") {
            return after[..end].trim();
        }
        return after.trim();
    }
    if let Some(start) = trimmed.find("```") {
        let after = &trimmed[start + 3..];
        if let Some(end) = after.find("```") {
            return after[..end].trim();
        }
    }
    if let Some(start) = trimmed.find('[') {
        return &trimmed[start..];
    }
    if let Some(start) = trimmed.find('{') {
        return &trimmed[start..];
    }
    trimmed
}

/// Truncate by Unicode scalar values to keep LLM prompts bounded.
pub fn truncate_chars(input: &str, max_chars: usize) -> String {
    if input.chars().count() <= max_chars {
        return input.to_string();
    }
    let truncated: String = input.chars().take(max_chars).collect();
    format!("{}\n…(truncated)", truncated)
}
