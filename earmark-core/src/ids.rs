/*
 * Copyright (c) 2026 Mikhail Shakhnazarov. Dual-licensed under AGPL-3.0-or-later or commercial terms.
 * PROPRIETARY AND INTERNAL. ONLY LOCALLY COMMITTED.
 * v0.1_internal kernel.
 */

use crate::errors::CoreError;
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use std::fmt;
use std::marker::PhantomData;
use std::str::FromStr;
use uuid::Uuid;

pub trait IdSpec {
    const PREFIX: &'static str;
    fn generate_body() -> String {
        Uuid::new_v4().to_string().replace("-", "")
    }
    fn validate_body(body: &str) -> Result<(), CoreError> {
        if body.len() != 32 && body.len() != 36 {
            // Basic validation, detailed ones can be overridden
        }
        Ok(())
    }
}

#[derive(Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct TypedId<S: IdSpec>(String, PhantomData<S>);

impl<S: IdSpec> TypedId<S> {
    pub fn generate() -> Self {
        Self(format!("{}_{}", S::PREFIX, S::generate_body()), PhantomData)
    }

    pub fn parse(s: &str) -> Result<Self, CoreError> {
        let prefix = format!("{}_", S::PREFIX);
        if !s.starts_with(&prefix) {
            return Err(CoreError::InvalidIdentifier(format!(
                "expected prefix {}, got {}",
                prefix, s
            )));
        }
        let body = &s[prefix.len()..];
        S::validate_body(body)?;
        Ok(Self(s.to_string(), PhantomData))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl<S: IdSpec> fmt::Debug for TypedId<S> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl<S: IdSpec> fmt::Display for TypedId<S> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl<S: IdSpec> FromStr for TypedId<S> {
    type Err = CoreError;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Self::parse(s)
    }
}

impl<S: IdSpec> Serialize for TypedId<S> {
    fn serialize<Ser: Serializer>(&self, serializer: Ser) -> Result<Ser::Ok, Ser::Error> {
        serializer.serialize_str(&self.0)
    }
}

impl<'de, S: IdSpec> Deserialize<'de> for TypedId<S> {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let s = String::deserialize(deserializer)?;
        Self::parse(&s).map_err(serde::de::Error::custom)
    }
}

// ID types for the earmark ontology.

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ObjectIdSpec;
impl IdSpec for ObjectIdSpec {
    const PREFIX: &'static str = "obj";
}
pub type ObjectId = TypedId<ObjectIdSpec>;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct VersionIdSpec;
impl IdSpec for VersionIdSpec {
    const PREFIX: &'static str = "ver";
}
pub type VersionId = TypedId<VersionIdSpec>;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct RelationIdSpec;
impl IdSpec for RelationIdSpec {
    const PREFIX: &'static str = "rel";
}
pub type RelationId = TypedId<RelationIdSpec>;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct RunIdSpec;
impl IdSpec for RunIdSpec {
    const PREFIX: &'static str = "run";
}
pub type RunId = TypedId<RunIdSpec>;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct DispatchIdSpec;
impl IdSpec for DispatchIdSpec {
    const PREFIX: &'static str = "dis";
}
pub type DispatchId = TypedId<DispatchIdSpec>;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct PacketIdSpec;
impl IdSpec for PacketIdSpec {
    const PREFIX: &'static str = "pkt";
}
pub type PacketId = TypedId<PacketIdSpec>;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ChangeSetIdSpec;
impl IdSpec for ChangeSetIdSpec {
    const PREFIX: &'static str = "cs";
}
pub type ChangeSetId = TypedId<ChangeSetIdSpec>;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct CheckResultIdSpec;
impl IdSpec for CheckResultIdSpec {
    const PREFIX: &'static str = "chk";
}
pub type CheckResultId = TypedId<CheckResultIdSpec>;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct HandoffManifestIdSpec;
impl IdSpec for HandoffManifestIdSpec {
    const PREFIX: &'static str = "hnd";
}
pub type HandoffManifestId = TypedId<HandoffManifestIdSpec>;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ReviewIdSpec;
impl IdSpec for ReviewIdSpec {
    const PREFIX: &'static str = "rev";
}
pub type ReviewId = TypedId<ReviewIdSpec>;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ActorIdSpec;
impl IdSpec for ActorIdSpec {
    const PREFIX: &'static str = "act";
}
pub type ActorId = TypedId<ActorIdSpec>;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct SystemIdSpec;
impl IdSpec for SystemIdSpec {
    const PREFIX: &'static str = "sys";
}
pub type SystemId = TypedId<SystemIdSpec>;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct SystemPackIdSpec;
impl IdSpec for SystemPackIdSpec {
    const PREFIX: &'static str = "sp";
}
pub type SystemPackId = TypedId<SystemPackIdSpec>;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ClassIdSpec;
impl IdSpec for ClassIdSpec {
    const PREFIX: &'static str = "cls";
}
pub type ClassId = TypedId<ClassIdSpec>;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct TransitionIdSpec;
impl IdSpec for TransitionIdSpec {
    const PREFIX: &'static str = "tr";
}
pub type TransitionId = TypedId<TransitionIdSpec>;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct PacketTemplateIdSpec;
impl IdSpec for PacketTemplateIdSpec {
    const PREFIX: &'static str = "pt";
}
pub type PacketTemplateId = TypedId<PacketTemplateIdSpec>;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct RuntimeProtocolIdSpec;
impl IdSpec for RuntimeProtocolIdSpec {
    const PREFIX: &'static str = "rp";
}
pub type RuntimeProtocolId = TypedId<RuntimeProtocolIdSpec>;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ValidatorIdSpec;
impl IdSpec for ValidatorIdSpec {
    const PREFIX: &'static str = "val";
}
pub type ValidatorId = TypedId<ValidatorIdSpec>;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct SelectionPolicyIdSpec;
impl IdSpec for SelectionPolicyIdSpec {
    const PREFIX: &'static str = "sel";
}
pub type SelectionPolicyId = TypedId<SelectionPolicyIdSpec>;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ReportIdSpec;
impl IdSpec for ReportIdSpec {
    const PREFIX: &'static str = "rep";
}
pub type ReportId = TypedId<ReportIdSpec>;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ProviderProfileIdSpec;
impl IdSpec for ProviderProfileIdSpec {
    const PREFIX: &'static str = "pro";
}
pub type ProviderProfileId = TypedId<ProviderProfileIdSpec>;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct WorkerProfileIdSpec;
impl IdSpec for WorkerProfileIdSpec {
    const PREFIX: &'static str = "wrk";
}
pub type WorkerProfileId = TypedId<WorkerProfileIdSpec>;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ExternalConnectionIdSpec;
impl IdSpec for ExternalConnectionIdSpec {
    const PREFIX: &'static str = "conn";
}
pub type ExternalConnectionId = TypedId<ExternalConnectionIdSpec>;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct WorkflowIdSpec;
impl IdSpec for WorkflowIdSpec {
    const PREFIX: &'static str = "wf";
}
pub type WorkflowId = TypedId<WorkflowIdSpec>;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct RelationRuleIdSpec;
impl IdSpec for RelationRuleIdSpec {
    const PREFIX: &'static str = "rule";
}
pub type RelationRuleId = TypedId<RelationRuleIdSpec>;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ObjectRef {
    pub id: ObjectId,
    pub version_id: VersionId,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ReviewTargetRef {
    Object(ObjectRef),
    Relation(RelationId),
    ChangeSet(ChangeSetId),
    Dispatch(DispatchId),
    Run(RunId),
    Report(ReportId),
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum StandingTargetRef {
    Object(ObjectId),
    Relation(RelationId),
    Dispatch(DispatchId),
    Run(RunId),
    Review(ReviewId),
    CheckResult(CheckResultId),
}
