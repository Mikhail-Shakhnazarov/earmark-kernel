//! System definition and runtime profiles.

use serde::{Deserialize, Serialize};

use crate::ids::VersionRef;
use crate::standing::StandingDimensionDefinition;
use crate::values::Timestamp;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SystemDefinition {
    pub system_id: String,
    pub namespace: String,
    pub title: String,
    pub description: Option<String>,
    pub classes: Vec<VersionRef>,
    pub instructions: Vec<VersionRef>,
    pub policies: Vec<VersionRef>,
    pub workflows: Vec<VersionRef>,
    pub compiled_contexts: Vec<VersionRef>,
    pub provider_profiles: Vec<VersionRef>,
    pub default_compiled_context: Option<VersionRef>,
    pub default_provider_profile: Option<VersionRef>,
    #[serde(default)]
    pub standing_dimensions: Vec<StandingDimensionDefinition>,
    pub runtime_profile: RuntimeProfile,
    pub activated_at: Option<Timestamp>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RuntimeProfile {
    pub execution_surface: String,
    pub machine_output_default: String,
    pub work_surface_mode: String,
}
