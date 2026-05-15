//! Declaration-domain types (Classes, Workflows, Policies, Contexts).

use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

use crate::errors::CoreError;
use crate::ids::VersionRef;
use crate::relations::RelationRule;
use crate::standing::ClassStandingRules;
use crate::values::{JsonSchemaRef, MarkdownBody, ScalarValue};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum WorkflowOperationKind {
    CompileContext,
    Transform,
    Review,
    Export,
    Nop,
}

impl WorkflowOperationKind {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::CompileContext => "compile_context",
            Self::Transform => "transform",
            Self::Review => "review",
            Self::Export => "export",
            Self::Nop => "nop",
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub(crate) struct InstructionFrontmatter {
    pub name: String,
    pub version: String,
    pub purpose: String,
    pub input_classes: Vec<String>,
    pub output_classes: Vec<String>,
    pub execution_policy: String,
    pub provider_profile: Option<VersionRef>,
    pub trace_policy: String,
    pub register: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ClassDefinition {
    pub name: String,
    pub version: String,
    pub kind: String,
    pub required_headers: Vec<String>,
    pub payload_schema: JsonSchemaRef,
    pub standing_rules: ClassStandingRules,
    pub relation_rules: Vec<RelationRule>,
    pub validators: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct InstructionPayload {
    pub name: String,
    pub version: String,
    pub purpose: String,
    pub input_classes: Vec<String>,
    pub output_classes: Vec<String>,
    pub execution_policy: String,
    pub provider_profile: Option<VersionRef>,
    pub trace_policy: String,
    pub register: String,
    pub body: MarkdownBody,
}

impl InstructionPayload {
    pub fn parse_markdown(input: &str) -> Result<Self, CoreError> {
        let (frontmatter, body) =
            crate::serde_helpers::parse_markdown_frontmatter::<InstructionFrontmatter>(input)?;
        Ok(Self {
            name: frontmatter.name,
            version: frontmatter.version,
            purpose: frontmatter.purpose,
            input_classes: frontmatter.input_classes,
            output_classes: frontmatter.output_classes,
            execution_policy: frontmatter.execution_policy,
            provider_profile: frontmatter.provider_profile,
            trace_policy: frontmatter.trace_policy,
            register: frontmatter.register,
            body: MarkdownBody::new(body),
        })
    }

    pub fn to_markdown(&self) -> Result<String, CoreError> {
        let frontmatter = InstructionFrontmatter {
            name: self.name.clone(),
            version: self.version.clone(),
            purpose: self.purpose.clone(),
            input_classes: self.input_classes.clone(),
            output_classes: self.output_classes.clone(),
            execution_policy: self.execution_policy.clone(),
            provider_profile: self.provider_profile.clone(),
            trace_policy: self.trace_policy.clone(),
            register: self.register.clone(),
        };
        crate::serde_helpers::to_markdown_frontmatter(&frontmatter, self.body.as_str())
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct WorkflowDefinition {
    pub name: String,
    pub version: String,
    pub description: Option<String>,
    #[serde(default)]
    pub operations: Vec<WorkflowOperation>,
    #[serde(default)]
    pub edges: Vec<WorkflowEdge>,
    #[serde(default)]
    pub guards: Vec<WorkflowGuard>,
    #[serde(default)]
    pub output_contracts: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct WorkflowDeclaration {
    pub name: String,
    pub version: String,
    pub description: Option<String>,
    #[serde(default)]
    pub operations: Vec<WorkflowDeclarationOperation>,
    #[serde(default)]
    pub edges: Vec<WorkflowEdge>,
    #[serde(default)]
    pub guards: Vec<WorkflowGuard>,
    #[serde(default)]
    pub output_contracts: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum FlexibleVersionRef {
    Ref(VersionRef),
    Path(String),
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct WorkflowOperation {
    pub id: String,
    pub kind: WorkflowOperationKind,
    #[serde(default)]
    pub input_contracts: Vec<String>,
    #[serde(default)]
    pub output_contracts: Vec<String>,
    pub instruction: Option<VersionRef>,
    pub compiled_context: Option<VersionRef>,
    pub policy: Option<VersionRef>,
    pub provider_profile: Option<VersionRef>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct WorkflowDeclarationOperation {
    pub id: String,
    pub kind: WorkflowOperationKind,
    #[serde(default)]
    pub input_contracts: Vec<String>,
    #[serde(default)]
    pub output_contracts: Vec<String>,
    pub instruction: Option<FlexibleVersionRef>,
    pub compiled_context: Option<FlexibleVersionRef>,
    pub policy: Option<FlexibleVersionRef>,
    pub provider_profile: Option<FlexibleVersionRef>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct WorkflowEdge {
    pub from: String,
    pub to: String,
    pub condition: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct WorkflowGuard {
    pub id: String,
    pub expression: String,
    pub message: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct StandingPolicy {
    pub name: String,
    pub version: String,
    pub description: Option<String>,
    pub transition_rules: Vec<StandingTransitionRule>,
    pub operation_requirements: Vec<OperationRequirement>,
    pub escalations: Vec<EscalationRule>,
    pub rationale: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct StandingTransitionRule {
    pub dimension: String,
    pub from: Vec<String>,
    pub to: Vec<String>,
    pub requires_review: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Default)]
pub struct OperationRequirement {
    pub operation: String,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub required_standing: BTreeMap<String, String>,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub forbidden_standing: BTreeMap<String, Vec<String>>,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub required_protocols: BTreeMap<String, BTreeMap<String, ScalarValue>>,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub forbidden_protocols: BTreeMap<String, BTreeMap<String, ScalarValue>>,
}

impl<'de> Deserialize<'de> for OperationRequirement {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        #[derive(Deserialize)]
        struct Raw {
            operation: String,
            #[serde(default)]
            required_standing: Option<BTreeMap<String, String>>,
            #[serde(default)]
            forbidden_standing: Option<BTreeMap<String, Vec<String>>>,
            #[serde(default)]
            required_protocols: Option<BTreeMap<String, BTreeMap<String, ScalarValue>>>,
            #[serde(default)]
            forbidden_protocols: Option<BTreeMap<String, BTreeMap<String, ScalarValue>>>,
            #[serde(default)]
            minimums: Option<BTreeMap<String, String>>,
            #[serde(default)]
            forbidden: Option<BTreeMap<String, Vec<String>>>,
        }
        let raw = Raw::deserialize(deserializer)?;
        let required_standing = raw.required_standing.or(raw.minimums).unwrap_or_default();
        let forbidden_standing = raw.forbidden_standing.or(raw.forbidden).unwrap_or_default();
        Ok(OperationRequirement {
            operation: raw.operation,
            required_standing,
            forbidden_standing,
            required_protocols: raw.required_protocols.unwrap_or_default(),
            forbidden_protocols: raw.forbidden_protocols.unwrap_or_default(),
        })
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct EscalationRule {
    pub trigger: String,
    pub severity: String,
    pub message: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CompiledContextTemplate {
    pub name: String,
    pub version: String,
    pub description: Option<String>,
    pub select: CompiledContextSelect,
    pub group_by: Vec<String>,
    pub render: CompiledContextRender,
    pub visibility: CompiledContextVisibility,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CompiledContextSelect {
    pub classes: Vec<String>,
    pub standing: BTreeMap<String, Vec<String>>,
    pub relations: Vec<String>,
    pub time_range: Option<String>,
    #[serde(default)]
    pub expansion: CompiledContextExpansion,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CompiledContextExpansion {
    #[serde(default)]
    pub object_filter: ExpansionObjectFilter,
    #[serde(default)]
    pub include_boundary_relations: bool,
}

impl Default for CompiledContextExpansion {
    fn default() -> Self {
        Self {
            object_filter: ExpansionObjectFilter::Inherit,
            include_boundary_relations: false,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum ExpansionObjectFilter {
    #[default]
    Inherit,
    None,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CompiledContextRender {
    pub mode: String,
    pub manifest_format: Option<String>,
    pub prose_template: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CompiledContextVisibility {
    pub include_lineage: bool,
    pub include_constraints: bool,
    pub include_provenance: bool,
}
