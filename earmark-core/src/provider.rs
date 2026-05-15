//! Provider-profile and provider-runtime types.

use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

use crate::ids::{ObjectRef, VersionRef};
use crate::values::{ScalarValue, Timestamp};

#[derive(Debug, Default, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum HttpAuthKind {
    #[default]
    None,
    Header,
    Bearer,
    QueryParameter,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct HttpAuthConfig {
    pub kind: HttpAuthKind,
    pub header_name: Option<String>,
    pub param_name: Option<String>,
    pub env: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct HttpRequestTemplate {
    pub content_type: Option<String>,
    pub body: serde_json::Value,
}

impl Default for HttpRequestTemplate {
    fn default() -> Self {
        Self {
            content_type: None,
            body: serde_json::Value::Null,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct HttpResponseExtraction {
    pub text_path: String,
    pub finish_reason_path: Option<String>,
    pub input_tokens_path: Option<String>,
    pub output_tokens_path: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct HttpGenerationProfile {
    pub method: Option<String>,
    pub url_template: String,
    pub auth: HttpAuthConfig,
    pub request: HttpRequestTemplate,
    pub response: HttpResponseExtraction,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ProviderProfile {
    pub name: String,
    pub version: String,
    pub description: Option<String>,
    pub provider: String,
    pub model: String,
    pub endpoint_env: Option<String>,
    pub auth_env: Option<String>,
    pub budget: ProviderBudget,
    pub allowed_operations: Vec<String>,
    pub exposure: ProviderExposure,
    pub response_contract: ProviderResponseContract,
    #[serde(default)]
    pub http: Option<HttpGenerationProfile>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct ProviderBudget {
    pub max_input_tokens: Option<u32>,
    pub max_output_tokens: Option<u32>,
    pub max_cost_usd: Option<f32>,
    pub max_latency_ms: Option<u32>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ProviderExposure {
    pub allow_prose_objects: bool,
    pub allow_structured_declarations: bool,
    pub allow_work_surface_only: bool,
    pub allow_export_requests: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ProviderResponseContract {
    pub format: String,
    pub must_return_candidate_only: bool,
    pub must_include_lineage: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ProviderRequest {
    pub request_id: String,
    pub run_id: String,
    pub work_packet: ObjectRef,
    pub provider_profile: VersionRef,
    pub instruction_text: String,
    pub context_text: Option<String>,
    pub input_text: String,
    pub work_surface_manifest: Option<String>,
    pub inputs: Vec<ObjectRef>,
    pub response_contract: ProviderResponseContract,
    pub issued_at: Timestamp,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ProviderResponse {
    pub request_id: String,
    pub provider: String,
    pub model: String,
    pub status: String,
    pub candidate_payload: String,
    pub metadata: BTreeMap<String, ScalarValue>,
    pub advisory_warnings: Vec<String>,
    pub usage: Option<ProviderUsage>,
    pub received_at: Timestamp,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct ProviderUsage {
    pub input_tokens: Option<u32>,
    pub output_tokens: Option<u32>,
    pub estimated_cost_usd: Option<f32>,
    pub latency_ms: Option<u32>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ProviderRecord {
    pub record_id: String,
    pub request_id: String,
    pub run_id: String,
    pub work_packet: ObjectRef,
    pub provider_profile: VersionRef,
    pub provider: String,
    pub model: String,
    pub status: String,
    pub metadata: BTreeMap<String, ScalarValue>,
    pub advisory_warnings: Vec<String>,
    pub usage: Option<ProviderUsage>,
    pub message: Option<String>,
    pub recorded_at: Timestamp,
}
