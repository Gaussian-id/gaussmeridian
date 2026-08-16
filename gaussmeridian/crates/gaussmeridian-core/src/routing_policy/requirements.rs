//! Typed request requirements before catalog eligibility is evaluated.
//!
//! Explicit operator and request constraints are authoritative. Inferred semantic
//! features are advisory evidence and cannot silently become eligibility filters.

use std::collections::BTreeSet;

use serde::{Deserialize, Serialize};
use thiserror::Error;

use super::{CapabilityBand, DeploymentKind, SkillRequirement, SKILL_DIMENSIONS};

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum Modality {
    Text,
    ImageInput,
    AudioInput,
    VideoInput,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum AdapterFeature {
    ToolUse,
    JsonResponse,
    JsonSchemaResponse,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct HardRequirements {
    pub required_modalities: BTreeSet<Modality>,
    pub required_adapter_features: BTreeSet<AdapterFeature>,
    pub required_skills: Vec<SkillRequirement>,
    pub allowed_deployments: BTreeSet<DeploymentKind>,
    pub compliance: BTreeSet<String>,
    pub allowed_model_ids: BTreeSet<String>,
    pub denied_model_ids: BTreeSet<String>,
    pub absolute_capability_ceiling: CapabilityBand,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct AdvisoryFeatures {
    pub inferred_skills: Vec<SkillRequirement>,
    pub semantic_tags: BTreeSet<String>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct RoutingRequirements {
    hard: HardRequirements,
    advisory: AdvisoryFeatures,
    degradations: Vec<RequirementDegradation>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum RequirementDegradation {
    InvalidAdvisorySkillDropped { skill_index: usize },
    BlankSemanticTagDropped,
}

#[derive(Clone, Debug, Error, PartialEq, Eq)]
pub enum RequirementError {
    #[error("explicit routing requirements are malformed")]
    MalformedExplicitRequirements,
    #[error("explicit allowed deployments cannot be empty")]
    EmptyAllowedDeployments,
    #[error("explicit hard skill {skill_index} is duplicated")]
    DuplicateHardSkill { skill_index: usize },
    #[error("request and explicit model allowlists do not overlap")]
    NoAllowedModelOverlap,
    #[error("model {model_id} is both explicitly allowed and denied")]
    ModelAllowDenyConflict { model_id: String },
    #[error("explicit hard skill {skill_index} has an invalid index or proficiency")]
    InvalidHardSkill { skill_index: usize },
    #[error("compliance requirements cannot be blank")]
    BlankComplianceRequirement,
    #[error("model allow/deny identifiers cannot be blank")]
    BlankModelId,
}

impl RoutingRequirements {
    pub fn new(
        hard: HardRequirements,
        mut advisory: AdvisoryFeatures,
    ) -> Result<Self, RequirementError> {
        validate_hard_requirements(&hard)?;

        // Advisory extraction is fail-soft, but never invisible. The caller carries these
        // records into the authoritative snapshot degradation list.
        let mut degradations = Vec::new();
        advisory.inferred_skills.retain(|requirement| {
            let valid = valid_skill_requirement(requirement);
            if !valid {
                degradations.push(RequirementDegradation::InvalidAdvisorySkillDropped {
                    skill_index: requirement.skill_index,
                });
            }
            valid
        });
        advisory.semantic_tags.retain(|tag| {
            let valid = !tag.trim().is_empty();
            if !valid {
                degradations.push(RequirementDegradation::BlankSemanticTagDropped);
            }
            valid
        });

        Ok(Self {
            hard,
            advisory,
            degradations,
        })
    }

    pub fn hard(&self) -> &HardRequirements {
        &self.hard
    }

    pub fn advisory(&self) -> &AdvisoryFeatures {
        &self.advisory
    }

    pub fn hard_skill_requirements(&self) -> &[SkillRequirement] {
        &self.hard.required_skills
    }

    pub fn degradations(&self) -> &[RequirementDegradation] {
        &self.degradations
    }
}

fn validate_hard_requirements(hard: &HardRequirements) -> Result<(), RequirementError> {
    if hard.compliance.iter().any(|value| value.trim().is_empty()) {
        return Err(RequirementError::BlankComplianceRequirement);
    }

    if hard
        .allowed_model_ids
        .iter()
        .chain(&hard.denied_model_ids)
        .any(|model_id| model_id.trim().is_empty())
    {
        return Err(RequirementError::BlankModelId);
    }

    if let Some(model_id) = hard
        .allowed_model_ids
        .intersection(&hard.denied_model_ids)
        .next()
    {
        return Err(RequirementError::ModelAllowDenyConflict {
            model_id: model_id.clone(),
        });
    }

    if let Some(requirement) = hard
        .required_skills
        .iter()
        .find(|requirement| !valid_skill_requirement(requirement))
    {
        return Err(RequirementError::InvalidHardSkill {
            skill_index: requirement.skill_index,
        });
    }

    Ok(())
}

fn valid_skill_requirement(requirement: &SkillRequirement) -> bool {
    requirement.skill_index < SKILL_DIMENSIONS
        && requirement.minimum_proficiency.is_finite()
        && (0.0..=1.0).contains(&requirement.minimum_proficiency)
}
