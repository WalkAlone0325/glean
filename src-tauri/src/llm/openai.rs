use super::{ChatRequest, ChatResponse, LLMProvider, Message, ToolCall};
use anyhow::{anyhow, Result};
use async_trait::async_trait;
use futures::StreamExt;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use serde_json::{json, Map, Value};
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
    messages: Vec<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    temperature: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    max_tokens: Option<u32>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    tools: Vec<Value>,
    stream: bool,
}

fn message_to_openai(m: &Message) -> Value {
    let mut obj = Map::new();
    obj.insert("role".into(), Value::String(m.role.clone()));
    if !m.content.is_empty() {
        obj.insert("content".into(), Value::String(m.content.clone()));
    } else {
        obj.insert("content".into(), Value::Null);
    }
    if let Some(id) = &m.tool_call_id {
        obj.insert("tool_call_id".into(), Value::String(id.clone()));
    }
    if let Some(name) = &m.name {
        obj.insert("name".into(), Value::String(name.clone()));
    }
    if !m.tool_calls.is_empty() {
        let arr: Vec<Value> = m
            .tool_calls
            .iter()
            .map(|tc| {
                json!({
                    "id": tc.id,
                    "type": "function",
                    "function": {
                        "name": tc.name,
                        "arguments": tc.arguments,
                    }
                })
            })
            .collect();
        obj.insert("tool_calls".into(), Value::Array(arr));
    }
    Value::Object(obj)
}

fn tool_def_to_openai(td: &super::ToolDefinition) -> Value {
    json!({
        "type": "function",
        "function": {
            "name": td.name,
            "description": td.description,
            "parameters": td.parameters,
        }
    })
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
    finish_reason: Option<String>,
}

#[derive(Debug, Deserialize)]
struct OpenAIMessageResp {
    content: Option<String>,
    #[serde(default)]
    tool_calls: Vec<OpenAIToolCallResp>,
}

#[derive(Debug, Deserialize)]
struct OpenAIToolCallResp {
    id: String,
    function: OpenAIToolFunctionResp,
}

#[derive(Debug, Deserialize)]
struct OpenAIToolFunctionResp {
    name: String,
    arguments: String,
}

#[derive(Debug, Deserialize)]
struct OpenAIUsage {
    prompt_tokens: Option<u32>,
    completion_tokens: Option<u32>,
}

fn parse_tool_calls(raw: &[OpenAIToolCallResp]) -> Vec<ToolCall> {
    raw.iter()
        .map(|tc| ToolCall {
            id: tc.id.clone(),
            name: tc.function.name.clone(),
            arguments: tc.function.arguments.clone(),
        })
        .collect()
}

#[async_trait]
impl LLMProvider for OpenAIProvider {
    async fn chat(&self, req: ChatRequest) -> Result<ChatResponse> {
        let model = req.model.clone().unwrap_or_else(|| self.model.clone());
        let messages: Vec<Value> = req.messages.iter().map(message_to_openai).collect();
        let tools: Vec<Value> = req.tools.iter().map(tool_def_to_openai).collect();

        let body = OpenAIRequest {
            model: &model,
            messages,
            temperature: req.temperature,
            max_tokens: req.max_tokens,
            tools,
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

        let choice = parsed
            .choices
            .into_iter()
            .next()
            .ok_or_else(|| anyhow!("no choices in response"))?;

        let msg = choice.message.unwrap_or(OpenAIMessageResp {
            content: None,
            tool_calls: Vec::new(),
        });
        let content = msg.content.unwrap_or_default();
        let tool_calls = parse_tool_calls(&msg.tool_calls);

        let (input_tokens, output_tokens) = parsed
            .usage
            .map(|u| (u.prompt_tokens, u.completion_tokens))
            .unwrap_or((None, None));

        Ok(ChatResponse {
            content,
            model: parsed.model.unwrap_or(model),
            input_tokens,
            output_tokens,
            tool_calls,
            finish_reason: choice.finish_reason,
        })
    }

    async fn chat_stream(
        &self,
        req: ChatRequest,
        on_delta: Box<dyn Fn(String) + Send + Sync>,
    ) -> Result<ChatResponse> {
        let model = req.model.clone().unwrap_or_else(|| self.model.clone());
        let messages: Vec<Value> = req.messages.iter().map(message_to_openai).collect();
        let tools: Vec<Value> = req.tools.iter().map(tool_def_to_openai).collect();

        let body = OpenAIRequest {
            model: &model,
            messages,
            temperature: req.temperature,
            max_tokens: req.max_tokens,
            tools,
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
        let mut buf: Vec<u8> = Vec::new();
        let mut full_content = String::new();
        let mut input_tokens: Option<u32> = None;
        let mut output_tokens: Option<u32> = None;
        let mut finish_reason: Option<String> = None;
        let mut tool_call_state: Vec<ToolCallAccumulator> = Vec::new();

        while let Some(chunk) = stream.next().await {
            let chunk = chunk?;
            buf.extend_from_slice(&chunk);

            loop {
                let Some(line_end) = buf.iter().position(|b| *b == b'\n') else { break };
                let line_bytes: Vec<u8> = buf.drain(..=line_end).collect();
                let line_str = match std::str::from_utf8(&line_bytes) {
                    Ok(s) => s.trim().to_string(),
                    Err(_) => {
                        let lossy = String::from_utf8_lossy(&line_bytes);
                        lossy.trim().to_string()
                    }
                };
                if line_str.is_empty() || !line_str.starts_with("data:") {
                    continue;
                }
                let data = line_str[5..].trim();
                if data == "[DONE]" {
                    break;
                }
                let Ok(value): Result<Value, _> = serde_json::from_str(data) else {
                    continue;
                };

                let delta = &value["choices"][0]["delta"];
                let delta_text = delta["content"].as_str();
                if let Some(s) = delta_text {
                    if !s.is_empty() {
                        full_content.push_str(s);
                        on_delta(s.to_string());
                    }
                }

                if let Some(arr) = delta["tool_calls"].as_array() {
                    for tc in arr {
                        let Some(idx) = tc["index"].as_u64() else { continue };
                        let idx = idx as usize;
                        while tool_call_state.len() <= idx {
                            tool_call_state.push(ToolCallAccumulator::default());
                        }
                        let slot = &mut tool_call_state[idx];
                        if let Some(id) = tc["id"].as_str().filter(|s| !s.is_empty()) {
                            slot.id = id.to_string();
                        }
                        if let Some(name) = tc["function"]["name"].as_str().filter(|s| !s.is_empty()) {
                            slot.name = name.to_string();
                        }
                        if let Some(args) = tc["function"]["arguments"].as_str() {
                            slot.arguments.push_str(args);
                        }
                    }
                }

                if let Some(reason) = value["choices"][0]["finish_reason"]
                    .as_str()
                    .filter(|s| !s.is_empty())
                {
                    finish_reason = Some(reason.to_string());
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

        let tool_calls: Vec<ToolCall> = tool_call_state
            .into_iter()
            .filter(|t| !t.name.is_empty())
            .map(|t| ToolCall {
                id: t.id,
                name: t.name,
                arguments: t.arguments,
            })
            .collect();

        Ok(ChatResponse {
            content: full_content,
            model,
            input_tokens,
            output_tokens,
            tool_calls,
            finish_reason,
        })
    }

    fn name(&self) -> &str {
        "openai"
    }
}

#[derive(Default)]
struct ToolCallAccumulator {
    id: String,
    name: String,
    arguments: String,
}
