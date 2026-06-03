use super::{ChatRequest, ChatResponse, LLMProvider};
use anyhow::{anyhow, Result};
use async_trait::async_trait;
use futures::StreamExt;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::time::Duration;

pub struct OpenAIProvider {
    base_url: String,
    api_key: String,
    model: String,
    client: Client,
}

impl OpenAIProvider {
    pub fn new(base_url: impl Into<String>, api_key: impl Into<String>, model: impl Into<String>) -> Self {
        Self {
            base_url: base_url.into().trim_end_matches('/').to_string(),
            api_key: api_key.into(),
            model: model.into(),
            client: Client::builder()
                .timeout(Duration::from_secs(120))
                .build()
                .unwrap_or_else(|_| Client::new()),
        }
    }
}

#[derive(Debug, Serialize)]
struct OpenAIRequest<'a> {
    model: &'a str,
    messages: Vec<OpenAIMessage<'a>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    temperature: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    max_tokens: Option<u32>,
    stream: bool,
}

#[derive(Debug, Serialize)]
struct OpenAIMessage<'a> {
    role: &'a str,
    content: &'a str,
}

#[derive(Debug, Deserialize)]
struct OpenAIResponse {
    choices: Vec<OpenAIChoice>,
    usage: Option<OpenAIUsage>,
    model: Option<String>,
}

#[derive(Debug, Deserialize)]
struct OpenAIChoice {
    message: Option<OpenAIMessageResp>,
}

#[derive(Debug, Deserialize)]
struct OpenAIMessageResp {
    content: Option<String>,
}

#[derive(Debug, Deserialize)]
struct OpenAIUsage {
    prompt_tokens: Option<u32>,
    completion_tokens: Option<u32>,
}

#[async_trait]
impl LLMProvider for OpenAIProvider {
    async fn chat(&self, req: ChatRequest) -> Result<ChatResponse> {
        let model = req.model.unwrap_or_else(|| self.model.clone());
        let messages: Vec<OpenAIMessage> = req
            .messages
            .iter()
            .map(|m| OpenAIMessage {
                role: m.role.as_str(),
                content: m.content.as_str(),
            })
            .collect();

        let body = OpenAIRequest {
            model: &model,
            messages,
            temperature: req.temperature,
            max_tokens: req.max_tokens,
            stream: false,
        };

        let url = format!("{}/chat/completions", self.base_url);
        let resp = self
            .client
            .post(&url)
            .bearer_auth(&self.api_key)
            .json(&body)
            .send()
            .await?;

        let status = resp.status();
        let text = resp.text().await?;
        if !status.is_success() {
            return Err(anyhow!("OpenAI error {}: {}", status, text));
        }

        let parsed: OpenAIResponse = serde_json::from_str(&text)
            .map_err(|e| anyhow!("parse response failed: {} (raw: {})", e, &text[..text.len().min(500)]))?;

        let content = parsed
            .choices
            .into_iter()
            .next()
            .and_then(|c| c.message)
            .and_then(|m| m.content)
            .unwrap_or_default();

        let (input_tokens, output_tokens) = parsed
            .usage
            .map(|u| (u.prompt_tokens, u.completion_tokens))
            .unwrap_or((None, None));

        Ok(ChatResponse {
            content,
            model: parsed.model.unwrap_or(model),
            input_tokens,
            output_tokens,
        })
    }

    async fn chat_stream(
        &self,
        req: ChatRequest,
        on_delta: Box<dyn Fn(String) + Send + Sync>,
    ) -> Result<ChatResponse> {
        let model = req.model.unwrap_or_else(|| self.model.clone());
        let messages: Vec<OpenAIMessage> = req
            .messages
            .iter()
            .map(|m| OpenAIMessage {
                role: m.role.as_str(),
                content: m.content.as_str(),
            })
            .collect();

        let body = OpenAIRequest {
            model: &model,
            messages,
            temperature: req.temperature,
            max_tokens: req.max_tokens,
            stream: true,
        };

        let url = format!("{}/chat/completions", self.base_url);
        let resp = self
            .client
            .post(&url)
            .bearer_auth(&self.api_key)
            .json(&body)
            .send()
            .await?;

        let status = resp.status();
        if !status.is_success() {
            let text = resp.text().await?;
            return Err(anyhow!("OpenAI error {}: {}", status, text));
        }

        let mut stream = resp.bytes_stream();
        let mut buf = String::new();
        let mut full_content = String::new();
        let mut input_tokens: Option<u32> = None;
        let mut output_tokens: Option<u32> = None;

        while let Some(chunk) = stream.next().await {
            let chunk = chunk?;
            buf.push_str(&String::from_utf8_lossy(&chunk));

            loop {
                let Some(line_end) = buf.find('\n') else { break };
                let line = buf[..line_end].trim().to_string();
                buf.drain(..=line_end);

                if line.is_empty() || !line.starts_with("data:") {
                    continue;
                }
                let data = line[5..].trim();
                if data == "[DONE]" {
                    continue;
                }
                let Ok(value): Result<Value, _> = serde_json::from_str(data) else {
                    continue;
                };
                let delta_owned: Option<String> = value["choices"][0]["delta"]["content"]
                    .as_str()
                    .map(|s| s.to_string());
                if let Some(delta) = delta_owned {
                    if !delta.is_empty() {
                        full_content.push_str(&delta);
                        on_delta(delta);
                    }
                }
                if let Some(usage) = value.get("usage") {
                    if let Some(n) = usage["prompt_tokens"].as_u64() {
                        input_tokens = Some(n as u32);
                    }
                    if let Some(n) = usage["completion_tokens"].as_u64() {
                        output_tokens = Some(n as u32);
                    }
                }
            }
        }

        Ok(ChatResponse {
            content: full_content,
            model,
            input_tokens,
            output_tokens,
        })
    }

    fn name(&self) -> &str {
        "openai"
    }
}
