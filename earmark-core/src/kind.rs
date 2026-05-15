//! Crate-wide object kinds.

use serde::{Deserialize, Serialize};
use std::str::FromStr;

use crate::errors::CoreError;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Kind {
    Object,
    Relation,
    Instruction,
    Policy,
    Workflow,
    CompiledContextTemplate,
    ProviderProfile,
    Review,
    Event,
    WorkPacket,
    RunRecord,
    SystemDefinition,
    TransitionAssignment,
    ChangeSet,
    HandoffManifest,
    TransformationFailure,
    UndoRecord,
}

impl Kind {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Object => "object",
            Self::Relation => "relation",
            Self::Instruction => "instruction",
            Self::Policy => "policy",
            Self::Workflow => "workflow",
            Self::CompiledContextTemplate => "compiled_context_template",
            Self::ProviderProfile => "provider_profile",
            Self::Review => "review",
            Self::Event => "event",
            Self::WorkPacket => "work_packet",
            Self::RunRecord => "run_record",
            Self::SystemDefinition => "system_definition",
            Self::TransitionAssignment => "transition_assignment",
            Self::ChangeSet => "change_set",
            Self::HandoffManifest => "handoff_manifest",
            Self::TransformationFailure => "transformation_failure",
            Self::UndoRecord => "undo_record",
        }
    }
}

impl FromStr for Kind {
    type Err = CoreError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "object" => Ok(Self::Object),
            "relation" => Ok(Self::Relation),
            "instruction" => Ok(Self::Instruction),
            "policy" => Ok(Self::Policy),
            "workflow" => Ok(Self::Workflow),
            "compiled_context_template" => Ok(Self::CompiledContextTemplate),
            "provider_profile" => Ok(Self::ProviderProfile),
            "review" => Ok(Self::Review),
            "event" => Ok(Self::Event),
            "work_packet" => Ok(Self::WorkPacket),
            "run_record" => Ok(Self::RunRecord),
            "system_definition" => Ok(Self::SystemDefinition),
            "transition_assignment" => Ok(Self::TransitionAssignment),
            "change_set" => Ok(Self::ChangeSet),
            "handoff_manifest" => Ok(Self::HandoffManifest),
            "transformation_failure" => Ok(Self::TransformationFailure),
            "undo_record" => Ok(Self::UndoRecord),
            other => Err(CoreError::InvalidKind(other.to_string())),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn kind_round_trips_undo_record() {
        assert_eq!(Kind::UndoRecord.as_str(), "undo_record");
        assert_eq!("undo_record".parse::<Kind>().unwrap(), Kind::UndoRecord);
    }
}
