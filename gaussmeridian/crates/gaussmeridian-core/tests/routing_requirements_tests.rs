use std::collections::BTreeSet;

use gaussmeridian_core::routing_policy::{
    requirements::{
        AdapterFeature, AdvisoryFeatures, HardRequirements, Modality, RequirementDegradation,
        RequirementError, RoutingRequirements,
    },
    CapabilityBand, DeploymentKind, SkillRequirement,
};

fn set<T: Ord>(values: impl IntoIterator<Item = T>) -> BTreeSet<T> {
    values.into_iter().collect()
}

fn complete_hard_requirements() -> HardRequirements {
    HardRequirements {
        required_modalities: set([Modality::Text, Modality::ImageInput]),
        required_adapter_features: set([AdapterFeature::ToolUse, AdapterFeature::JsonResponse]),
        required_skills: vec![SkillRequirement {
            skill_index: 1,
            minimum_proficiency: 0.8,
        }],
        allowed_deployments: set([DeploymentKind::Managed, DeploymentKind::BringYourOwnKey]),
        compliance: set(["au-residency".to_string(), "soc2".to_string()]),
        allowed_model_ids: set(["frontier-a".to_string(), "frontier-b".to_string()]),
        denied_model_ids: set(["legacy-c".to_string()]),
        absolute_capability_ceiling: CapabilityBand::Frontier,
    }
}

#[test]
fn explicit_requirements_remain_hard_and_inference_remains_advisory() {
    let hard = complete_hard_requirements();
    let advisory = AdvisoryFeatures {
        inferred_skills: vec![SkillRequirement {
            skill_index: 7,
            minimum_proficiency: 0.99,
        }],
        semantic_tags: set(["code".to_string(), "legal".to_string()]),
    };

    let requirements = RoutingRequirements::new(hard.clone(), advisory.clone()).unwrap();

    assert_eq!(requirements.hard(), &hard);
    assert_eq!(requirements.advisory(), &advisory);
    assert_eq!(requirements.hard_skill_requirements(), hard.required_skills);
    assert!(!requirements
        .hard_skill_requirements()
        .iter()
        .any(|skill| skill.skill_index == 7));
}

#[test]
fn malformed_explicit_model_policy_fails_validation() {
    let mut hard = complete_hard_requirements();
    hard.denied_model_ids.insert("frontier-a".to_string());

    let error = RoutingRequirements::new(hard, AdvisoryFeatures::default()).unwrap_err();

    assert_eq!(
        error,
        RequirementError::ModelAllowDenyConflict {
            model_id: "frontier-a".to_string(),
        }
    );
}

#[test]
fn malformed_explicit_skill_and_compliance_values_fail_validation() {
    let mut invalid_skill = complete_hard_requirements();
    invalid_skill.required_skills[0].minimum_proficiency = f64::NAN;
    assert_eq!(
        RoutingRequirements::new(invalid_skill, AdvisoryFeatures::default()).unwrap_err(),
        RequirementError::InvalidHardSkill { skill_index: 1 }
    );

    let mut blank_compliance = complete_hard_requirements();
    blank_compliance.compliance.insert("  ".to_string());
    assert_eq!(
        RoutingRequirements::new(blank_compliance, AdvisoryFeatures::default()).unwrap_err(),
        RequirementError::BlankComplianceRequirement
    );
}

#[test]
fn malformed_advisory_inference_fails_soft_with_visible_degradation() {
    let advisory = AdvisoryFeatures {
        inferred_skills: vec![SkillRequirement {
            skill_index: 4,
            minimum_proficiency: f64::NAN,
        }],
        semantic_tags: set(["  ".to_string()]),
    };

    let requirements = RoutingRequirements::new(complete_hard_requirements(), advisory).unwrap();

    assert!(requirements.advisory().inferred_skills.is_empty());
    assert!(requirements.advisory().semantic_tags.is_empty());
    assert_eq!(
        requirements.degradations(),
        &[
            RequirementDegradation::InvalidAdvisorySkillDropped { skill_index: 4 },
            RequirementDegradation::BlankSemanticTagDropped,
        ]
    );
}
