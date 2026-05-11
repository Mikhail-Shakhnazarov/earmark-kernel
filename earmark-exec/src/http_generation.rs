#[cfg(feature = "http-provider")]
use crate::error::{ProviderFailure, ProviderFailureKind};
#[cfg(feature = "http-provider")]
use crate::provider::ProviderAdapter;
#[cfg(feature = "http-provider")]
use earmark_core::{ProviderProfile, ProviderRequest, ProviderResponse, ProviderUsage};
#[cfg(feature = "http-provider")]
use std::collections::BTreeMap;
#[cfg(feature = "http-provider")]
use std::env;
#[cfg(feature = "http-provider")]
use std::time::Duration;

#[cfg(feature = "http-provider")]
pub struct HttpGenerationAdapter;

#[cfg(feature = "http-provider")]
impl ProviderAdapter for HttpGenerationAdapter {
    fn provider_key(&self) -> &'static str {
        "http_generation"
    }

    fn provide(
        &self,
        request: ProviderRequest,
        profile: &ProviderProfile,
        _transition_operation: &str,
    ) -> Result<ProviderResponse, ProviderFailure> {
        let http = profile.http.as_ref().ok_or_else(|| {
            ProviderFailure::new(
                ProviderFailureKind::ProviderUnavailable,
                "http_generation adapter requires an 'http' block in the profile",
            )
        })?;

        // 1. Prepare variables
        let input_text = if let Some(path) = &request.work_surface_manifest {
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

        let mut vars = BTreeMap::new();
        vars.insert("model".to_string(), profile.model.clone());
        vars.insert("input_text".to_string(), input_text);
        vars.insert(
            "instruction_text".to_string(),
            request.instruction_text.clone(),
        );
        vars.insert(
            "max_output_tokens".to_string(),
            profile.budget.max_output_tokens.unwrap_or(256).to_string(),
        );

        // 4. Resolve Auth
        let mut auth_header: Option<String> = None;
        let mut auth_value: Option<String> = None;

        match http.auth.kind {
            earmark_core::HttpAuthKind::None => {}
            earmark_core::HttpAuthKind::Header => {
                let header = http.auth.header_name.as_ref().ok_or_else(|| {
                    ProviderFailure::new(
                        ProviderFailureKind::AuthenticationFailed,
                        "missing header_name for header auth",
                    )
                })?;
                let env_name = http
                    .auth
                    .env
                    .as_ref()
                    .or(profile.auth_env.as_ref())
                    .ok_or_else(|| {
                        ProviderFailure::new(
                            ProviderFailureKind::AuthenticationFailed,
                            "missing auth env variable name",
                        )
                    })?;
                let val = env::var(env_name).map_err(|_| {
                    ProviderFailure::new(
                        ProviderFailureKind::AuthenticationFailed,
                        format!("auth env variable '{}' not set", env_name),
                    )
                })?;
                auth_header = Some(header.clone());
                auth_value = Some(val);
            }
            earmark_core::HttpAuthKind::Bearer => {
                let env_name = http
                    .auth
                    .env
                    .as_ref()
                    .or(profile.auth_env.as_ref())
                    .ok_or_else(|| {
                        ProviderFailure::new(
                            ProviderFailureKind::AuthenticationFailed,
                            "missing auth env variable name",
                        )
                    })?;
                let val = env::var(env_name).map_err(|_| {
                    ProviderFailure::new(
                        ProviderFailureKind::AuthenticationFailed,
                        format!("auth env variable '{}' not set", env_name),
                    )
                })?;
                auth_header = Some("Authorization".to_string());
                auth_value = Some(format!("Bearer {}", val));
            }
            earmark_core::HttpAuthKind::QueryParameter => {
                let env_name = http
                    .auth
                    .env
                    .as_ref()
                    .or(profile.auth_env.as_ref())
                    .ok_or_else(|| {
                        ProviderFailure::new(
                            ProviderFailureKind::AuthenticationFailed,
                            "missing auth env variable name",
                        )
                    })?;
                let val = env::var(env_name).map_err(|_| {
                    ProviderFailure::new(
                        ProviderFailureKind::AuthenticationFailed,
                        format!("auth env variable '{}' not set", env_name),
                    )
                })?;
                auth_value = Some(val);
            }
        }

        // 2. Build URL
        let url = render_template(&http.url_template, &vars);

        // 3. Build Body
        let body = render_json_template(&http.request.body, &vars);

        // 5. Send Request
        let client = reqwest::blocking::Client::builder()
            .timeout(Duration::from_millis(
                profile.budget.max_latency_ms.unwrap_or(30_000) as u64,
            ))
            .build()
            .map_err(|e| {
                ProviderFailure::new(ProviderFailureKind::ProviderUnavailable, e.to_string())
            })?;

        let mut rb = client.post(&url).header(
            "Content-Type",
            http.request
                .content_type
                .as_deref()
                .unwrap_or("application/json"),
        );

        if let (Some(h), Some(v)) = (auth_header, auth_value.as_ref()) {
            rb = rb.header(h, v);
        }

        if let earmark_core::HttpAuthKind::QueryParameter = http.auth.kind {
            if let Some(v) = &auth_value {
                let p = http.auth.param_name.as_deref().unwrap_or("key");
                rb = rb.query(&[(p, v)]);
            }
        }

        let response = rb.json(&body).send().map_err(|e| {
            if e.is_timeout() {
                ProviderFailure::new(ProviderFailureKind::Timeout, e.to_string())
            } else {
                ProviderFailure::new(ProviderFailureKind::ProviderUnavailable, e.to_string())
            }
        })?;

        let status = response.status();
        if !status.is_success() {
            let kind = match status.as_u16() {
                401 | 403 => ProviderFailureKind::AuthenticationFailed,
                429 => ProviderFailureKind::RateLimited,
                _ => ProviderFailureKind::ProviderUnavailable,
            };
            return Err(ProviderFailure::new(kind, format!("HTTP {}", status)));
        }

        let resp_json: serde_json::Value = response.json().map_err(|e| {
            ProviderFailure::new(ProviderFailureKind::MalformedResponse, e.to_string())
        })?;

        // 6. Extract Response
        let text = extract_path(&resp_json, &http.response.text_path).ok_or_else(|| {
            ProviderFailure::new(
                ProviderFailureKind::MalformedResponse,
                format!("could not find text at path '{}'", http.response.text_path),
            )
        })?;

        let mut usage = ProviderUsage::default();
        if let Some(path) = &http.response.input_tokens_path {
            if let Some(val) = extract_path(&resp_json, path).and_then(|v| v.parse::<u32>().ok()) {
                usage.input_tokens = Some(val);
            }
        }
        if let Some(path) = &http.response.output_tokens_path {
            if let Some(val) = extract_path(&resp_json, path).and_then(|v| v.parse::<u32>().ok()) {
                usage.output_tokens = Some(val);
            }
        }

        Ok(ProviderResponse {
            request_id: request.request_id,
            provider: "http_generation".to_string(),
            model: profile.model.clone(),
            status: "completed".to_string(),
            candidate_payload: text,
            metadata: BTreeMap::new(),
            advisory_warnings: vec![],
            usage: Some(usage),
            received_at: chrono::Utc::now(),
        })
    }
}

#[cfg(feature = "http-provider")]
fn render_template(template: &str, vars: &BTreeMap<String, String>) -> String {
    let mut result = template.to_string();
    for (k, v) in vars {
        result = result.replace(&format!("{{{{{}}}}}", k), v);
    }
    result
}

#[cfg(feature = "http-provider")]
fn render_json_template(
    value: &serde_json::Value,
    vars: &BTreeMap<String, String>,
) -> serde_json::Value {
    match value {
        serde_json::Value::String(s) => serde_json::Value::String(render_template(s, vars)),
        serde_json::Value::Array(arr) => {
            serde_json::Value::Array(arr.iter().map(|v| render_json_template(v, vars)).collect())
        }
        serde_json::Value::Object(obj) => {
            let mut new_obj = serde_json::Map::new();
            for (k, v) in obj {
                new_obj.insert(k.clone(), render_json_template(v, vars));
            }
            serde_json::Value::Object(new_obj)
        }
        _ => value.clone(),
    }
}

#[cfg(feature = "http-provider")]
fn extract_path(value: &serde_json::Value, path: &str) -> Option<String> {
    if !path.starts_with("$.") {
        return None;
    }
    let parts = path[2..].split('.');
    let mut current = value;
    for part in parts {
        if part.contains('[') && part.ends_with(']') {
            let bracket_start = part.find('[')?;
            let key = &part[..bracket_start];
            let index_str = &part[bracket_start + 1..part.len() - 1];
            let index: usize = index_str.parse().ok()?;

            if !key.is_empty() {
                current = current.get(key)?;
            }
            current = current.get(index)?;
        } else {
            current = current.get(part)?;
        }
    }

    match current {
        serde_json::Value::String(s) => Some(s.clone()),
        serde_json::Value::Number(n) => Some(n.to_string()),
        serde_json::Value::Bool(b) => Some(b.to_string()),
        _ => None,
    }
}

#[cfg(all(test, feature = "http-provider"))]
mod tests {
    use super::*;
    use earmark_core::{
        HttpAuthConfig, HttpAuthKind, HttpGenerationProfile, HttpRequestTemplate,
        HttpResponseExtraction, ProviderBudget, ProviderResponseContract,
    };
    use httpmock::prelude::*;
    use serde_json::json;

    #[test]
    fn test_render_template() {
        let mut vars = BTreeMap::new();
        vars.insert("model".to_string(), "gpt-4".to_string());
        vars.insert("input_text".to_string(), "hello".to_string());

        let rendered = render_template("https://api.com/{{model}}?q={{input_text}}", &vars);
        assert_eq!(rendered, "https://api.com/gpt-4?q=hello");
    }

    #[test]
    fn test_extract_path() {
        let val = json!({
            "choices": [
                {
                    "message": {
                        "content": "hello world"
                    }
                }
            ],
            "usage": {
                "total": 100
            }
        });

        assert_eq!(
            extract_path(&val, "$.choices[0].message.content"),
            Some("hello world".to_string())
        );
        assert_eq!(extract_path(&val, "$.usage.total"), Some("100".to_string()));
        assert_eq!(extract_path(&val, "$.nonexistent"), None);
    }

    #[test]
    #[cfg(feature = "http-provider")]
    fn test_adapter_provide_success() {
        let server = MockServer::start();
        let m = server.mock(|when, then| {
            when.method(POST)
                .path("/v1/chat")
                .header("x-api-key", "secret")
                .json_body(json!({
                    "model": "test-model",
                    "prompt": "Inputs: []\n\nPlease process according to instructions."
                }));
            then.status(200).json_body(json!({
                "output": "EARMARK_OK",
                "usage": { "tokens": 42 }
            }));
        });

        let profile = ProviderProfile {
            name: "test".to_string(),
            version: "1".to_string(),
            description: None,
            provider: "http_generation".to_string(),
            model: "test-model".to_string(),
            endpoint_env: None,
            auth_env: Some("TEST_API_KEY".to_string()),
            budget: ProviderBudget {
                max_input_tokens: None,
                max_output_tokens: Some(128),
                max_cost_usd: None,
                max_latency_ms: None,
            },
            allowed_operations: vec!["transform".to_string()],
            exposure: earmark_core::ProviderExposure {
                allow_prose_objects: true,
                allow_structured_declarations: true,
                allow_work_surface_only: false,
                allow_export_requests: false,
            },
            response_contract: ProviderResponseContract {
                format: "markdown".to_string(),
                must_return_candidate_only: true,
                must_include_lineage: false,
            },
            http: Some(HttpGenerationProfile {
                method: Some("POST".to_string()),
                url_template: format!("{}/v1/chat", server.base_url()),
                auth: HttpAuthConfig {
                    kind: HttpAuthKind::Header,
                    header_name: Some("x-api-key".to_string()),
                    param_name: None,
                    env: Some("TEST_API_KEY".to_string()),
                },
                request: HttpRequestTemplate {
                    content_type: Some("application/json".to_string()),
                    body: json!({
                        "model": "{{model}}",
                        "prompt": "{{input_text}}"
                    }),
                },
                response: HttpResponseExtraction {
                    text_path: "$.output".to_string(),
                    finish_reason_path: None,
                    input_tokens_path: Some("$.usage.tokens".to_string()),
                    output_tokens_path: None,
                },
            }),
        };

        std::env::set_var("TEST_API_KEY", "secret");

        let adapter = HttpGenerationAdapter;
        let request = ProviderRequest {
            request_id: "req1".to_string(),
            run_id: "run1".to_string(),
            work_packet: earmark_core::ObjectRef::new(
                earmark_core::ObjectId::new(),
                earmark_core::VersionId::new(),
                earmark_core::Kind::WorkPacket,
                None,
            ),
            provider_profile: earmark_core::VersionRef::new(
                earmark_core::ObjectId::new(),
                earmark_core::VersionId::new(),
            ),
            instruction_text: "hi".to_string(),
            work_surface_manifest: None,
            inputs: vec![],
            response_contract: profile.response_contract.clone(),
            issued_at: chrono::Utc::now(),
        };

        let result = adapter.provide(request, &profile, "transform").unwrap();
        assert_eq!(result.candidate_payload, "EARMARK_OK");
        assert_eq!(result.usage.unwrap().input_tokens, Some(42));
        m.assert();
    }

    #[test]
    #[ignore]
    fn test_live_gemini_flash() {
        let adapter = HttpGenerationAdapter {};
        let profile = ProviderProfile {
            name: "gemini_flash_http".to_string(),
            version: "0.1.0".to_string(),
            description: None,
            provider: "http_generation".to_string(),
            model: "gemini-3.1-flash-lite".to_string(),
            endpoint_env: None,
            auth_env: Some("GEMINI_API_KEY".to_string()),
            budget: earmark_core::ProviderBudget::default(),
            allowed_operations: vec!["transform".to_string()],
            exposure: earmark_core::ProviderExposure {
                allow_prose_objects: true,
                allow_structured_declarations: false,
                allow_work_surface_only: false,
                allow_export_requests: false,
            },
            response_contract: earmark_core::ProviderResponseContract {
                format: "markdown".to_string(),
                must_return_candidate_only: true,
                must_include_lineage: false,
            },
            http: Some(HttpGenerationProfile {
                method: Some("POST".to_string()),
                url_template: "https://generativelanguage.googleapis.com/v1beta/models/{{model}}:generateContent".to_string(),
                auth: HttpAuthConfig {
                    kind: HttpAuthKind::QueryParameter,
                    param_name: Some("key".to_string()),
                    header_name: None,
                    env: None,
                },
                request: HttpRequestTemplate {
                    content_type: Some("application/json".to_string()),
                    body: serde_json::json!({
                        "contents": [{
                            "parts": [{ "text": "{{input_text}}" }]
                        }]
                    }),
                },
                response: HttpResponseExtraction {
                    text_path: "$.candidates[0].content.parts[0].text".to_string(),
                    input_tokens_path: Some("$.usageMetadata.promptTokenCount".to_string()),
                    output_tokens_path: Some("$.usageMetadata.candidatesTokenCount".to_string()),
                    finish_reason_path: None,
                },
            }),
        };

        let request = ProviderRequest {
            request_id: "test".to_string(),
            run_id: "test".to_string(),
            work_packet: earmark_core::ObjectRef::new(
                earmark_core::ObjectId::new(),
                earmark_core::VersionId::new(),
                earmark_core::Kind::WorkPacket,
                None,
            ),
            provider_profile: earmark_core::VersionRef::new(
                earmark_core::ObjectId::new(),
                earmark_core::VersionId::new(),
            ),
            instruction_text: "Return exactly: EARMARK_GEMINI_SMOKE_OK".to_string(),
            work_surface_manifest: None,
            inputs: vec![],
            response_contract: profile.response_contract.clone(),
            issued_at: chrono::Utc::now(),
        };

        let api_key = std::env::var("GEMINI_API_KEY").unwrap_or_default();
        let response = adapter.provide(request, &profile, "transform").unwrap();
        let sanitized = response.candidate_payload.replace(&api_key, "***");
        println!("Sanitized Response: {}", sanitized);
        assert!(response
            .candidate_payload
            .contains("EARMARK_GEMINI_SMOKE_OK"));
    }

    #[test]
    #[cfg(feature = "http-provider")]
    fn test_provider_service_integration() {
        use crate::provider::{provide_with_registry, ProviderRegistry};
        use earmark_core::{ProviderExposure, ProviderResponseContract};
        use std::sync::Arc;

        let server = httpmock::MockServer::start();
        let m = server.mock(|when, then| {
            when.method(httpmock::Method::POST)
                .path("/v1/test")
                .json_body(serde_json::json!({ "prompt": "hi" }));
            then.status(200).json_body(serde_json::json!({
                "result": "ok",
                "usage": { "tokens": 10 }
            }));
        });

        let profile = ProviderProfile {
            name: "service_test".to_string(),
            version: "1".to_string(),
            description: None,
            provider: "http_generation".to_string(),
            model: "service-model".to_string(),
            endpoint_env: None,
            auth_env: None,
            budget: earmark_core::ProviderBudget::default(),
            allowed_operations: vec!["transform".to_string()],
            exposure: ProviderExposure {
                allow_prose_objects: true,
                allow_structured_declarations: true,
                allow_work_surface_only: false,
                allow_export_requests: false,
            },
            response_contract: ProviderResponseContract {
                format: "markdown".to_string(),
                must_return_candidate_only: true,
                must_include_lineage: false,
            },
            http: Some(HttpGenerationProfile {
                method: Some("POST".to_string()),
                url_template: format!("{}/v1/test", server.base_url()),
                auth: HttpAuthConfig {
                    kind: earmark_core::HttpAuthKind::None,
                    ..Default::default()
                },
                request: HttpRequestTemplate {
                    content_type: Some("application/json".to_string()),
                    body: serde_json::json!({ "prompt": "{{instruction_text}}" }),
                },
                response: HttpResponseExtraction {
                    text_path: "$.result".to_string(),
                    input_tokens_path: Some("$.usage.tokens".to_string()),
                    ..Default::default()
                },
            }),
        };

        let mut registry = ProviderRegistry::new();
        registry.register(Arc::new(HttpGenerationAdapter));

        let request = ProviderRequest {
            request_id: "req_test".to_string(),
            run_id: "run_test".to_string(),
            work_packet: earmark_core::ObjectRef::new(
                earmark_core::ObjectId::new(),
                earmark_core::VersionId::new(),
                earmark_core::Kind::WorkPacket,
                None,
            ),
            provider_profile: earmark_core::VersionRef::new(
                earmark_core::ObjectId::new(),
                earmark_core::VersionId::new(),
            ),
            instruction_text: "hi".to_string(),
            work_surface_manifest: None,
            inputs: vec![],
            response_contract: profile.response_contract.clone(),
            issued_at: chrono::Utc::now(),
        };

        // 1. Success case
        let outcome =
            provide_with_registry(&registry, &profile, request.clone(), "transform").unwrap();
        assert_eq!(outcome.record.provider, "http_generation");
        assert_eq!(outcome.record.model, "service-model");
        assert_eq!(
            outcome.record.usage.as_ref().unwrap().input_tokens,
            Some(10)
        );
        assert!(!outcome.record.metadata.contains_key("synthetic"));

        // 2. Policy gate test (forbidden operation)
        let forbidden_res = provide_with_registry(&registry, &profile, request.clone(), "export");
        assert!(forbidden_res.is_err());
        assert_eq!(
            forbidden_res.unwrap_err().kind,
            crate::error::ProviderFailureKind::ForbiddenOperation
        );

        m.assert();
    }
}
