use crate::{
    ProviderAdapter, ProviderCapability, ProviderCapabilityStatus, ProviderFailure,
    ProviderFailureKind,
};
#[cfg(feature = "gemini")]
use chrono::Utc;
#[cfg(feature = "gemini")]
use earmark_core::ProviderUsage;
use earmark_core::{ProviderProfile, ProviderRequest, ProviderResponse};
#[cfg(feature = "gemini")]
use std::collections::BTreeMap;

#[cfg(feature = "gemini")]
use reqwest::blocking::Client;

pub struct GeminiAdapter {
    pub model: String,
    pub api_key_env: String,
}

impl GeminiAdapter {
    pub fn new(model: String, api_key_env: String) -> Self {
        Self { model, api_key_env }
    }
}

impl ProviderAdapter for GeminiAdapter {
    fn provider_key(&self) -> &'static str {
        "google_gemini"
    }

    fn provide(
        &self,
        request: ProviderRequest,
        profile: &ProviderProfile,
    ) -> Result<ProviderResponse, ProviderFailure> {
        #[cfg(not(feature = "gemini"))]
        {
            let _ = request;
            let _ = profile;
            Err(ProviderFailure::new(
                ProviderFailureKind::ProviderUnavailable,
                "Gemini feature not enabled",
            ))
        }

        #[cfg(feature = "gemini")]
        {
            let api_key = std::env::var(&self.api_key_env).map_err(|_| {
                ProviderFailure::new(
                    ProviderFailureKind::AuthenticationFailed,
                    format!("API key environment variable {} not set", self.api_key_env),
                )
            })?;

            let url = if let Some(endpoint_env) = &profile.endpoint_env {
                let val = std::env::var(endpoint_env).map_err(|_| {
                    ProviderFailure::new(
                        ProviderFailureKind::AuthenticationFailed,
                        format!("Endpoint environment variable {} not set", endpoint_env),
                    )
                })?;
                earmark_core::validate_endpoint_url(&val).map_err(|e| {
                    ProviderFailure::new(ProviderFailureKind::AuthenticationFailed, e.to_string())
                })?;
                val
            } else {
                format!(
                    "https://generativelanguage.googleapis.com/v1beta/models/{}:generateContent",
                    self.model
                )
            };

            let mut contents = Vec::new();

            // System instruction
            let mut system_instruction = None;
            if !request.instruction_text.is_empty() {
                system_instruction = Some(serde_json::json!({
                    "parts": [{ "text": request.instruction_text }]
                }));
            }

            // User content
            let user_text = if let Some(path) = &request.work_surface_manifest {
                format!(
                    "Work surface manifest: {}\n\nPlease process according to instructions.",
                    path
                )
            } else {
                format!(
                    "Inputs: {:?}\n\nPlease process according to instructions.",
                    request.inputs
                )
            };

            contents.push(serde_json::json!({
                "role": "user",
                "parts": [{ "text": user_text }]
            }));

            let mut body = serde_json::json!({
                "contents": contents
            });

            if let Some(si) = system_instruction {
                body["systemInstruction"] = si;
            }

            let mut generation_config = serde_json::json!({});
            if request.response_contract.format == "json" {
                generation_config["responseMimeType"] =
                    serde_json::Value::String("application/json".to_string());
            }

            // Budget enforcement
            if let Some(max_output) = profile.budget.max_output_tokens {
                generation_config["maxOutputTokens"] = serde_json::Value::Number(max_output.into());
            }

            body["generationConfig"] = generation_config;

            let mut client_builder = Client::builder();
            if let Some(max_latency) = profile.budget.max_latency_ms {
                client_builder =
                    client_builder.timeout(std::time::Duration::from_millis(max_latency as u64));
            }

            let client = client_builder.build().map_err(|e| {
                ProviderFailure::new(ProviderFailureKind::ProviderUnavailable, e.to_string())
            })?;

            let response = client
                .post(&url)
                .header("x-goog-api-key", &api_key)
                .json(&body)
                .send()
                .map_err(|e| {
                    if e.is_timeout() {
                        ProviderFailure::new(ProviderFailureKind::Timeout, e.to_string())
                    } else {
                        ProviderFailure::new(
                            ProviderFailureKind::ProviderUnavailable,
                            e.to_string(),
                        )
                    }
                })?;

            let status = response.status();
            if !status.is_success() {
                let err_text = response.text().unwrap_or_default();
                return Err(match status.as_u16() {
                    401 | 403 => {
                        ProviderFailure::new(ProviderFailureKind::AuthenticationFailed, err_text)
                    }
                    429 => ProviderFailure::new(
                        ProviderFailureKind::RateLimited,
                        "Rate limit exceeded",
                    ),
                    _ => ProviderFailure::new(
                        ProviderFailureKind::ProviderUnavailable,
                        format!("HTTP {}: {}", status, err_text),
                    ),
                });
            }

            let gemini_resp: serde_json::Value = response.json().map_err(|e| {
                ProviderFailure::new(ProviderFailureKind::MalformedResponse, e.to_string())
            })?;

            let candidate_text = gemini_resp["candidates"][0]["content"]["parts"][0]["text"]
                .as_str()
                .ok_or_else(|| {
                    ProviderFailure::new(
                        ProviderFailureKind::MalformedResponse,
                        "Could not find candidate text in response",
                    )
                })?
                .to_string();

            let mut usage = None;
            if let Some(meta) = gemini_resp.get("usageMetadata") {
                usage = Some(ProviderUsage {
                    input_tokens: meta["promptTokenCount"].as_u64().map(|v| v as u32),
                    output_tokens: meta["candidatesTokenCount"].as_u64().map(|v| v as u32),
                    estimated_cost_usd: None,
                    latency_ms: None,
                });
            }

            Ok(ProviderResponse {
                request_id: request.request_id,
                provider: "google_gemini".to_string(),
                model: self.model.clone(),
                status: "success".to_string(),
                candidate_payload: candidate_text,
                metadata: BTreeMap::new(),
                usage,
                received_at: Utc::now(),
            })
        }
    }

    fn capability(&self) -> ProviderCapability {
        let missing_env = std::env::var(&self.api_key_env)
            .ok()
            .filter(|value| !value.trim().is_empty())
            .map(|_| vec![])
            .unwrap_or_else(|| vec![self.api_key_env.clone()]);

        ProviderCapability {
            provider: self.provider_key().to_string(),
            status: if missing_env.is_empty() {
                ProviderCapabilityStatus::Available
            } else {
                ProviderCapabilityStatus::MissingConfiguration
            },
            feature: Some("gemini".to_string()),
            required_env: vec![self.api_key_env.clone()],
            missing_env,
            message: None,
        }
    }
}
