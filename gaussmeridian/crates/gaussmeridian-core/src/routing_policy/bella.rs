//! Pure contracts for the BELLA skill-evidence and capability boundary.
//!
//! The deterministic centroid profiler in this module is Meridian-owned. BELLA
//! supplies the task-skill and model-skill capability framing, but does not
//! prescribe this production profiler or the conservative Beta confidence rule.
//! This module performs no I/O and can only narrow a caller-supplied hard-eligible
//! route set.

use std::collections::{BTreeMap, BTreeSet};

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use thiserror::Error;

use super::{predictors::RouteIdentity, SKILL_DIMENSIONS};

pub const PROFILER_VERSION: &str = "meridian-skill-profiler/v1";
pub const ESTIMATOR_VERSION: &str = "beta-normal-lcb/v1";
pub const MAX_PROFILE_TOKENS: usize = 256;
pub const MAX_PROFILE_VOCABULARY: usize = 4_096;
pub const MAX_RATIONALE_BYTES: usize = 512;

#[derive(Clone, Debug, Error, PartialEq, Eq)]
pub enum BellaError {
    #[error("BELLA value is blank: {field}")]
    Blank { field: &'static str },
    #[error("BELLA value is non-finite: {field}")]
    NonFinite { field: &'static str },
    #[error("BELLA value is out of range: {field}")]
    OutOfRange { field: &'static str },
    #[error("BELLA collection is empty: {field}")]
    Empty { field: &'static str },
    #[error("BELLA collection contains a duplicate value: {field}={value}")]
    Duplicate { field: &'static str, value: String },
    #[error("BELLA taxonomy index exceeds the routing skill dimension: {index}")]
    SkillIndexOverflow { index: u16 },
    #[error("BELLA profiler references an unknown skill: {skill_id}")]
    UnknownSkill { skill_id: String },
    #[error("BELLA profiler has no controlled examples for skill: {skill_id}")]
    MissingTrainingData { skill_id: String },
    #[error("BELLA profiler vocabulary exceeds its bound")]
    VocabularyOverflow,
    #[error("BELLA posterior observation count overflowed")]
    ObservationCountOverflow,
    #[error("BELLA versions are incompatible: {field}")]
    VersionMismatch { field: &'static str },
    #[error("BELLA learner state is not canonically ordered: {field}")]
    NonCanonical { field: &'static str },
    #[error("BELLA learner training-content hash does not match its canonical content")]
    TrainingContentHashMismatch,
    #[error("BELLA canonical serialization failed: {reason}")]
    Serialization { reason: String },
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct SkillDefinition {
    pub index: u16,
    pub skill_id: String,
    pub name: String,
    pub description: String,
    pub minimum_proficiency: f64,
}

impl SkillDefinition {
    pub fn new(
        index: u16,
        skill_id: impl Into<String>,
        name: impl Into<String>,
        description: impl Into<String>,
        minimum_proficiency: f64,
    ) -> Result<Self, BellaError> {
        let definition = Self {
            index,
            skill_id: skill_id.into(),
            name: name.into(),
            description: description.into(),
            minimum_proficiency,
        };
        require_nonblank("skill.skill_id", &definition.skill_id)?;
        require_nonblank("skill.name", &definition.name)?;
        require_nonblank("skill.description", &definition.description)?;
        require_probability("skill.minimum_proficiency", definition.minimum_proficiency)?;
        if usize::from(definition.index) >= SKILL_DIMENSIONS {
            return Err(BellaError::SkillIndexOverflow {
                index: definition.index,
            });
        }
        Ok(definition)
    }

    fn validate(&self) -> Result<(), BellaError> {
        Self::new(
            self.index,
            self.skill_id.clone(),
            self.name.clone(),
            self.description.clone(),
            self.minimum_proficiency,
        )
        .map(|_| ())
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct SkillTaxonomy {
    version: String,
    skills: Vec<SkillDefinition>,
}

impl SkillTaxonomy {
    pub fn new(
        version: impl Into<String>,
        mut skills: Vec<SkillDefinition>,
    ) -> Result<Self, BellaError> {
        let version = version.into();
        require_nonblank("taxonomy.version", &version)?;
        if skills.is_empty() {
            return Err(BellaError::Empty {
                field: "taxonomy.skills",
            });
        }
        for skill in &skills {
            skill.validate()?;
        }

        skills.sort_unstable_by(|left, right| {
            left.index
                .cmp(&right.index)
                .then_with(|| left.skill_id.cmp(&right.skill_id))
        });
        reject_duplicate(
            skills.iter().map(|skill| skill.index.to_string()),
            "taxonomy.skill_index",
        )?;
        reject_duplicate(
            skills.iter().map(|skill| skill.skill_id.clone()),
            "taxonomy.skill_id",
        )?;

        Ok(Self { version, skills })
    }

    pub fn version(&self) -> &str {
        &self.version
    }

    pub fn skills(&self) -> &[SkillDefinition] {
        &self.skills
    }

    pub fn skill(&self, skill_id: &str) -> Option<&SkillDefinition> {
        self.skills.iter().find(|skill| skill.skill_id == skill_id)
    }

    fn validate(&self) -> Result<(), BellaError> {
        let canonical = Self::new(self.version.clone(), self.skills.clone())?;
        if canonical != *self {
            return Err(BellaError::NonCanonical {
                field: "taxonomy.skills",
            });
        }
        Ok(())
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct ProfilerPolicy {
    activation_threshold: f64,
    ambiguity_margin: f64,
    rationale_term_limit: usize,
}

impl ProfilerPolicy {
    pub fn new(
        activation_threshold: f64,
        ambiguity_margin: f64,
        rationale_term_limit: usize,
    ) -> Result<Self, BellaError> {
        require_probability("profiler.activation_threshold", activation_threshold)?;
        require_probability("profiler.ambiguity_margin", ambiguity_margin)?;
        if rationale_term_limit == 0 {
            return Err(BellaError::OutOfRange {
                field: "profiler.rationale_term_limit",
            });
        }
        Ok(Self {
            activation_threshold,
            ambiguity_margin,
            rationale_term_limit,
        })
    }

    fn validate(&self) -> Result<(), BellaError> {
        Self::new(
            self.activation_threshold,
            self.ambiguity_margin,
            self.rationale_term_limit,
        )
        .map(|_| ())
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProfilerTrainingExample {
    example_id: String,
    task: String,
    skill_ids: Vec<String>,
}

impl ProfilerTrainingExample {
    pub fn new(
        example_id: impl Into<String>,
        task: impl Into<String>,
        mut skill_ids: Vec<String>,
    ) -> Result<Self, BellaError> {
        let example_id = example_id.into();
        let task = task.into();
        require_nonblank("profiler.example_id", &example_id)?;
        require_nonblank("profiler.task", &task)?;
        if skill_ids.is_empty() {
            return Err(BellaError::Empty {
                field: "profiler.skill_ids",
            });
        }
        for skill_id in &skill_ids {
            require_nonblank("profiler.skill_id", skill_id)?;
        }
        skill_ids.sort_unstable();
        reject_duplicate(skill_ids.iter().cloned(), "profiler.skill_id")?;
        Ok(Self {
            example_id,
            task,
            skill_ids,
        })
    }

    fn validate(&self) -> Result<(), BellaError> {
        let canonical = Self::new(
            self.example_id.clone(),
            self.task.clone(),
            self.skill_ids.clone(),
        )?;
        if canonical != *self {
            return Err(BellaError::NonCanonical {
                field: "profiler.example",
            });
        }
        Ok(())
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
struct SkillCentroid {
    skill_index: u16,
    skill_id: String,
    skill_name: String,
    minimum_proficiency: f64,
    weights: Vec<f64>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct MeridianSkillProfiler {
    profiler_version: String,
    taxonomy_version: String,
    policy: ProfilerPolicy,
    vocabulary: Vec<String>,
    centroids: Vec<SkillCentroid>,
}

impl MeridianSkillProfiler {
    pub fn train(
        taxonomy: &SkillTaxonomy,
        mut examples: Vec<ProfilerTrainingExample>,
        policy: ProfilerPolicy,
    ) -> Result<Self, BellaError> {
        taxonomy.validate()?;
        policy.validate()?;
        if examples.is_empty() {
            return Err(BellaError::Empty {
                field: "profiler.examples",
            });
        }

        examples.sort_unstable_by(|left, right| left.example_id.cmp(&right.example_id));
        reject_duplicate(
            examples.iter().map(|example| example.example_id.clone()),
            "profiler.example_id",
        )?;
        for example in &examples {
            example.validate()?;
            for skill_id in &example.skill_ids {
                if taxonomy.skill(skill_id).is_none() {
                    return Err(BellaError::UnknownSkill {
                        skill_id: skill_id.clone(),
                    });
                }
            }
        }

        let tokenized: Vec<Vec<String>> = examples
            .iter()
            .map(|example| bounded_tokens(&example.task))
            .collect();
        if tokenized.iter().any(Vec::is_empty) {
            return Err(BellaError::Empty {
                field: "profiler.example_tokens",
            });
        }

        let vocabulary: Vec<String> = tokenized
            .iter()
            .flat_map(|tokens| tokens.iter().cloned())
            .collect::<BTreeSet<_>>()
            .into_iter()
            .collect();
        if vocabulary.len() > MAX_PROFILE_VOCABULARY {
            return Err(BellaError::VocabularyOverflow);
        }
        let vocabulary_index: BTreeMap<&str, usize> = vocabulary
            .iter()
            .enumerate()
            .map(|(index, term)| (term.as_str(), index))
            .collect();
        let row_vectors: Vec<Vec<f64>> = tokenized
            .iter()
            .map(|tokens| normalized_term_vector(tokens, &vocabulary_index))
            .collect();

        let mut centroids = Vec::with_capacity(taxonomy.skills.len());
        for skill in &taxonomy.skills {
            let mut weights = vec![0.0; vocabulary.len()];
            let mut supporting_rows = 0_u64;
            for (example, vector) in examples.iter().zip(&row_vectors) {
                if example.skill_ids.binary_search(&skill.skill_id).is_ok() {
                    supporting_rows += 1;
                    for (weight, value) in weights.iter_mut().zip(vector) {
                        *weight += value;
                    }
                }
            }
            if supporting_rows == 0 {
                return Err(BellaError::MissingTrainingData {
                    skill_id: skill.skill_id.clone(),
                });
            }
            normalize(&mut weights);
            centroids.push(SkillCentroid {
                skill_index: skill.index,
                skill_id: skill.skill_id.clone(),
                skill_name: skill.name.clone(),
                minimum_proficiency: skill.minimum_proficiency,
                weights,
            });
        }

        Ok(Self {
            profiler_version: PROFILER_VERSION.into(),
            taxonomy_version: taxonomy.version.clone(),
            policy,
            vocabulary,
            centroids,
        })
    }

    pub fn version(&self) -> &str {
        &self.profiler_version
    }

    pub fn taxonomy_version(&self) -> &str {
        &self.taxonomy_version
    }

    pub fn profile(&self, task: &str) -> TaskProfileResult {
        let tokens = bounded_tokens(task);
        let task_fingerprint = fingerprint_tokens(&tokens);
        let vocabulary_index: BTreeMap<&str, usize> = self
            .vocabulary
            .iter()
            .enumerate()
            .map(|(index, term)| (term.as_str(), index))
            .collect();
        let task_vector = normalized_term_vector(&tokens, &vocabulary_index);
        if vector_norm(&task_vector) == 0.0 {
            return TaskProfileResult::Abstained(ProfileAbstention {
                profiler_version: self.profiler_version.clone(),
                taxonomy_version: self.taxonomy_version.clone(),
                task_fingerprint,
                reason: ProfileAbstentionReason::NoVocabularyOverlap,
            });
        }

        let mut ranked: Vec<(usize, f64)> = self
            .centroids
            .iter()
            .enumerate()
            .map(|(index, centroid)| (index, dot(&task_vector, &centroid.weights)))
            .collect();
        ranked.sort_unstable_by(|(left_index, left), (right_index, right)| {
            right.total_cmp(left).then_with(|| {
                self.centroids[*left_index]
                    .skill_index
                    .cmp(&self.centroids[*right_index].skill_index)
            })
        });

        let active_indices: BTreeSet<usize> = ranked
            .iter()
            .filter_map(|(index, similarity)| {
                (*similarity >= self.policy.activation_threshold).then_some(*index)
            })
            .collect();
        if active_indices.is_empty() {
            let ambiguous = ranked.len() > 1
                && (ranked[0].1 - ranked[1].1).abs() <= self.policy.ambiguity_margin;
            return TaskProfileResult::Abstained(ProfileAbstention {
                profiler_version: self.profiler_version.clone(),
                taxonomy_version: self.taxonomy_version.clone(),
                task_fingerprint,
                reason: if ambiguous {
                    ProfileAbstentionReason::AmbiguousSkillMatch
                } else {
                    ProfileAbstentionReason::BelowActivationThreshold
                },
            });
        }

        let similarities: BTreeMap<usize, f64> = ranked.into_iter().collect();
        let requirements = self
            .centroids
            .iter()
            .enumerate()
            .filter(|(index, _)| active_indices.contains(index))
            .map(|(index, centroid)| TaskSkillRequirement {
                skill_index: centroid.skill_index,
                skill_id: centroid.skill_id.clone(),
                skill_name: centroid.skill_name.clone(),
                minimum_proficiency: centroid.minimum_proficiency,
                similarity: similarities[&index],
                rationale: rationale(
                    &task_vector,
                    &centroid.weights,
                    &self.vocabulary,
                    self.policy.rationale_term_limit,
                ),
            })
            .collect();
        TaskProfileResult::Profiled(FrozenTaskSkillProfile {
            profiler_version: self.profiler_version.clone(),
            taxonomy_version: self.taxonomy_version.clone(),
            task_fingerprint,
            requirements,
        })
    }

    fn validate_against(&self, taxonomy: &SkillTaxonomy) -> Result<(), BellaError> {
        require_nonblank("profiler.version", &self.profiler_version)?;
        if self.profiler_version != PROFILER_VERSION {
            return Err(BellaError::VersionMismatch {
                field: "profiler.version",
            });
        }
        if self.taxonomy_version != taxonomy.version {
            return Err(BellaError::VersionMismatch {
                field: "profiler.taxonomy_version",
            });
        }
        self.policy.validate()?;
        if self.vocabulary.is_empty() {
            return Err(BellaError::Empty {
                field: "profiler.vocabulary",
            });
        }
        if self.vocabulary.len() > MAX_PROFILE_VOCABULARY {
            return Err(BellaError::VocabularyOverflow);
        }
        let mut vocabulary = self.vocabulary.clone();
        vocabulary.sort_unstable();
        vocabulary.dedup();
        if vocabulary != self.vocabulary {
            return Err(BellaError::NonCanonical {
                field: "profiler.vocabulary",
            });
        }
        if self.centroids.len() != taxonomy.skills.len() {
            return Err(BellaError::VersionMismatch {
                field: "profiler.centroids",
            });
        }
        for (centroid, skill) in self.centroids.iter().zip(&taxonomy.skills) {
            if centroid.skill_index != skill.index
                || centroid.skill_id != skill.skill_id
                || centroid.skill_name != skill.name
                || centroid.minimum_proficiency != skill.minimum_proficiency
            {
                return Err(BellaError::VersionMismatch {
                    field: "profiler.skill",
                });
            }
            if centroid.weights.len() != self.vocabulary.len()
                || centroid
                    .weights
                    .iter()
                    .any(|weight| !weight.is_finite() || *weight < 0.0)
            {
                return Err(BellaError::OutOfRange {
                    field: "profiler.centroid",
                });
            }
            if (vector_norm(&centroid.weights) - 1.0).abs() > 1e-12 {
                return Err(BellaError::OutOfRange {
                    field: "profiler.centroid_norm",
                });
            }
        }
        Ok(())
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct TaskSkillRequirement {
    pub skill_index: u16,
    pub skill_id: String,
    pub skill_name: String,
    pub minimum_proficiency: f64,
    pub similarity: f64,
    pub rationale: String,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct FrozenTaskSkillProfile {
    pub profiler_version: String,
    pub taxonomy_version: String,
    pub task_fingerprint: String,
    pub requirements: Vec<TaskSkillRequirement>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum ProfileAbstentionReason {
    NoVocabularyOverlap,
    AmbiguousSkillMatch,
    BelowActivationThreshold,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProfileAbstention {
    pub profiler_version: String,
    pub taxonomy_version: String,
    pub task_fingerprint: String,
    pub reason: ProfileAbstentionReason,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum TaskProfileResult {
    Profiled(FrozenTaskSkillProfile),
    Abstained(ProfileAbstention),
}

impl TaskProfileResult {
    pub fn task_fingerprint(&self) -> &str {
        match self {
            Self::Profiled(profile) => &profile.task_fingerprint,
            Self::Abstained(abstention) => &abstention.task_fingerprint,
        }
    }

    pub fn abstention_reason(&self) -> Option<ProfileAbstentionReason> {
        match self {
            Self::Profiled(_) => None,
            Self::Abstained(abstention) => Some(abstention.reason),
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct BellaEstimatorPolicy {
    minimum_support: u64,
    uncertainty_multiplier: f64,
}

impl BellaEstimatorPolicy {
    pub fn new(minimum_support: u64, uncertainty_multiplier: f64) -> Result<Self, BellaError> {
        if minimum_support == 0 {
            return Err(BellaError::OutOfRange {
                field: "estimator.minimum_support",
            });
        }
        require_nonnegative_finite("estimator.uncertainty_multiplier", uncertainty_multiplier)?;
        Ok(Self {
            minimum_support,
            uncertainty_multiplier,
        })
    }

    fn validate(&self) -> Result<(), BellaError> {
        Self::new(self.minimum_support, self.uncertainty_multiplier).map(|_| ())
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct SkillPosterior {
    pub route: RouteIdentity,
    pub taxonomy_version: String,
    pub skill_id: String,
    pub alpha_prior: f64,
    pub beta_prior: f64,
    pub positive_count: u64,
    pub negative_count: u64,
}

impl SkillPosterior {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        route: RouteIdentity,
        taxonomy_version: impl Into<String>,
        skill_id: impl Into<String>,
        alpha_prior: f64,
        beta_prior: f64,
        positive_count: u64,
        negative_count: u64,
    ) -> Result<Self, BellaError> {
        let posterior = Self {
            route,
            taxonomy_version: taxonomy_version.into(),
            skill_id: skill_id.into(),
            alpha_prior,
            beta_prior,
            positive_count,
            negative_count,
        };
        posterior.validate()?;
        Ok(posterior)
    }

    pub fn estimate(&self, policy: &BellaEstimatorPolicy) -> Result<SkillEstimate, BellaError> {
        self.validate()?;
        policy.validate()?;
        let support = self
            .positive_count
            .checked_add(self.negative_count)
            .ok_or(BellaError::ObservationCountOverflow)?;
        if support < policy.minimum_support {
            return Ok(SkillEstimate::Abstained {
                route: self.route.clone(),
                taxonomy_version: self.taxonomy_version.clone(),
                skill_id: self.skill_id.clone(),
                support,
                reason: SkillEstimateAbstentionReason::InsufficientSupport,
            });
        }

        let alpha = self.alpha_prior + self.positive_count as f64;
        let beta = self.beta_prior + self.negative_count as f64;
        let total = alpha + beta;
        let mean = alpha / total;
        let variance = alpha * beta / (total * total * (total + 1.0));
        let uncertainty = variance.sqrt();
        let conservative_proficiency =
            (mean - policy.uncertainty_multiplier * uncertainty).max(0.0);
        Ok(SkillEstimate::Estimated {
            route: self.route.clone(),
            taxonomy_version: self.taxonomy_version.clone(),
            skill_id: self.skill_id.clone(),
            estimator_version: ESTIMATOR_VERSION.into(),
            support,
            mean,
            variance,
            uncertainty,
            conservative_proficiency,
        })
    }

    fn validate(&self) -> Result<(), BellaError> {
        require_nonblank("posterior.route.provider_id", self.route.provider_id())?;
        require_nonblank("posterior.route.model_id", self.route.model_id())?;
        require_nonblank("posterior.taxonomy_version", &self.taxonomy_version)?;
        require_nonblank("posterior.skill_id", &self.skill_id)?;
        require_positive_finite("posterior.alpha_prior", self.alpha_prior)?;
        require_positive_finite("posterior.beta_prior", self.beta_prior)?;
        self.positive_count
            .checked_add(self.negative_count)
            .ok_or(BellaError::ObservationCountOverflow)?;
        Ok(())
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum SkillEstimateAbstentionReason {
    InsufficientSupport,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum SkillEstimate {
    Estimated {
        route: RouteIdentity,
        taxonomy_version: String,
        skill_id: String,
        estimator_version: String,
        support: u64,
        mean: f64,
        variance: f64,
        uncertainty: f64,
        conservative_proficiency: f64,
    },
    Abstained {
        route: RouteIdentity,
        taxonomy_version: String,
        skill_id: String,
        support: u64,
        reason: SkillEstimateAbstentionReason,
    },
}

impl SkillEstimate {
    fn route(&self) -> &RouteIdentity {
        match self {
            Self::Estimated { route, .. } | Self::Abstained { route, .. } => route,
        }
    }

    fn skill_id(&self) -> &str {
        match self {
            Self::Estimated { skill_id, .. } | Self::Abstained { skill_id, .. } => skill_id,
        }
    }

    fn taxonomy_version(&self) -> &str {
        match self {
            Self::Estimated {
                taxonomy_version, ..
            }
            | Self::Abstained {
                taxonomy_version, ..
            } => taxonomy_version,
        }
    }

    fn validate(&self, policy: &BellaEstimatorPolicy) -> Result<(), BellaError> {
        require_nonblank("estimate.route.provider_id", self.route().provider_id())?;
        require_nonblank("estimate.route.model_id", self.route().model_id())?;
        match self {
            Self::Estimated {
                taxonomy_version,
                skill_id,
                estimator_version,
                mean,
                variance,
                uncertainty,
                conservative_proficiency,
                support,
                ..
            } => {
                require_nonblank("estimate.taxonomy_version", taxonomy_version)?;
                require_nonblank("estimate.skill_id", skill_id)?;
                if estimator_version != ESTIMATOR_VERSION {
                    return Err(BellaError::VersionMismatch {
                        field: "estimate.estimator_version",
                    });
                }
                require_probability("estimate.mean", *mean)?;
                require_nonnegative_finite("estimate.variance", *variance)?;
                require_nonnegative_finite("estimate.uncertainty", *uncertainty)?;
                require_probability(
                    "estimate.conservative_proficiency",
                    *conservative_proficiency,
                )?;
                if *support < policy.minimum_support
                    || *variance > 0.25
                    || *uncertainty > 0.5
                    || (*uncertainty - variance.sqrt()).abs() > 1e-12
                {
                    return Err(BellaError::OutOfRange {
                        field: "estimate.posterior_math",
                    });
                }
                let expected_conservative =
                    (*mean - policy.uncertainty_multiplier * *uncertainty).max(0.0);
                if (*conservative_proficiency - expected_conservative).abs() > 1e-12 {
                    return Err(BellaError::OutOfRange {
                        field: "estimate.conservative_proficiency",
                    });
                }
                Ok(())
            }
            Self::Abstained {
                taxonomy_version,
                skill_id,
                support,
                ..
            } => {
                require_nonblank("estimate.taxonomy_version", taxonomy_version)?;
                require_nonblank("estimate.skill_id", skill_id)?;
                if *support >= policy.minimum_support {
                    return Err(BellaError::OutOfRange {
                        field: "estimate.abstained_support",
                    });
                }
                Ok(())
            }
        }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct BellaLearnerState {
    taxonomy: SkillTaxonomy,
    profiler: MeridianSkillProfiler,
    critic_version: String,
    evaluator_version: String,
    corpus_version: String,
    catalog_version: String,
    estimator_policy: BellaEstimatorPolicy,
    posteriors: Vec<SkillPosterior>,
    training_content_hash: String,
}

#[derive(Serialize)]
struct LearnerHashPayload<'a> {
    taxonomy: &'a SkillTaxonomy,
    profiler: &'a MeridianSkillProfiler,
    critic_version: &'a str,
    evaluator_version: &'a str,
    corpus_version: &'a str,
    catalog_version: &'a str,
    estimator_policy: &'a BellaEstimatorPolicy,
    posteriors: &'a [SkillPosterior],
}

impl BellaLearnerState {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        taxonomy: SkillTaxonomy,
        profiler: MeridianSkillProfiler,
        critic_version: impl Into<String>,
        evaluator_version: impl Into<String>,
        corpus_version: impl Into<String>,
        catalog_version: impl Into<String>,
        estimator_policy: BellaEstimatorPolicy,
        mut posteriors: Vec<SkillPosterior>,
    ) -> Result<Self, BellaError> {
        posteriors.sort_unstable_by(|left, right| {
            left.route
                .cmp(&right.route)
                .then_with(|| left.skill_id.cmp(&right.skill_id))
        });
        let mut state = Self {
            taxonomy,
            profiler,
            critic_version: critic_version.into(),
            evaluator_version: evaluator_version.into(),
            corpus_version: corpus_version.into(),
            catalog_version: catalog_version.into(),
            estimator_policy,
            posteriors,
            training_content_hash: String::new(),
        };
        state.validate_content()?;
        state.training_content_hash = state.compute_training_content_hash()?;
        Ok(state)
    }

    pub fn training_content_hash(&self) -> &str {
        &self.training_content_hash
    }

    pub fn taxonomy_version(&self) -> &str {
        self.taxonomy.version()
    }

    pub fn profiler_version(&self) -> &str {
        self.profiler.version()
    }

    pub fn critic_version(&self) -> &str {
        &self.critic_version
    }

    pub fn evaluator_version(&self) -> &str {
        &self.evaluator_version
    }

    pub fn corpus_version(&self) -> &str {
        &self.corpus_version
    }

    pub fn catalog_version(&self) -> &str {
        &self.catalog_version
    }

    pub fn validate(&self) -> Result<(), BellaError> {
        self.validate_content()?;
        if self.compute_training_content_hash()? != self.training_content_hash {
            return Err(BellaError::TrainingContentHashMismatch);
        }
        Ok(())
    }

    pub fn freeze(
        &self,
        task: &str,
        state_id: impl Into<String>,
        runtime_catalog_version: &str,
    ) -> FrozenBellaEvidence {
        let state_id = state_id.into();
        if self.validate().is_err() || state_id.trim().is_empty() {
            return FrozenBellaEvidence::Unavailable(FrozenBellaUnavailable {
                state_id: nonblank_owned(state_id),
                taxonomy_version: Some(self.taxonomy.version.clone()),
                critic_version: None,
                reason: BellaUnavailableReason::InvalidState,
            });
        }
        if runtime_catalog_version != self.catalog_version {
            return FrozenBellaEvidence::Unavailable(FrozenBellaUnavailable {
                state_id: Some(state_id),
                taxonomy_version: Some(self.taxonomy.version.clone()),
                critic_version: Some(self.critic_version.clone()),
                reason: BellaUnavailableReason::CatalogVersionMismatch,
            });
        }

        match self.profiler.profile(task) {
            TaskProfileResult::Abstained(abstention) => {
                FrozenBellaEvidence::Abstained(FrozenBellaAbstention {
                    state_id,
                    critic_version: Some(self.critic_version.clone()),
                    profile: abstention,
                })
            }
            TaskProfileResult::Profiled(profile) => {
                let required_skills: BTreeSet<&str> = profile
                    .requirements
                    .iter()
                    .map(|requirement| requirement.skill_id.as_str())
                    .collect();
                let estimates = self
                    .posteriors
                    .iter()
                    .filter(|posterior| required_skills.contains(posterior.skill_id.as_str()))
                    .filter_map(|posterior| posterior.estimate(&self.estimator_policy).ok())
                    .collect();
                FrozenBellaEvidence::Active(FrozenBellaActive {
                    state_id,
                    training_content_hash: self.training_content_hash.clone(),
                    critic_version: self.critic_version.clone(),
                    evaluator_version: self.evaluator_version.clone(),
                    corpus_version: self.corpus_version.clone(),
                    catalog_version: self.catalog_version.clone(),
                    estimator_policy: self.estimator_policy,
                    profile,
                    estimates,
                })
            }
        }
    }

    fn validate_content(&self) -> Result<(), BellaError> {
        self.taxonomy.validate()?;
        self.profiler.validate_against(&self.taxonomy)?;
        self.estimator_policy.validate()?;
        require_nonblank("learner.critic_version", &self.critic_version)?;
        require_nonblank("learner.evaluator_version", &self.evaluator_version)?;
        require_nonblank("learner.corpus_version", &self.corpus_version)?;
        require_nonblank("learner.catalog_version", &self.catalog_version)?;

        let mut previous: Option<(&RouteIdentity, &str)> = None;
        for posterior in &self.posteriors {
            posterior.validate()?;
            if posterior.taxonomy_version != self.taxonomy.version {
                return Err(BellaError::VersionMismatch {
                    field: "posterior.taxonomy_version",
                });
            }
            if self.taxonomy.skill(&posterior.skill_id).is_none() {
                return Err(BellaError::UnknownSkill {
                    skill_id: posterior.skill_id.clone(),
                });
            }
            let current = (&posterior.route, posterior.skill_id.as_str());
            if let Some(previous) = previous {
                match previous
                    .0
                    .cmp(current.0)
                    .then_with(|| previous.1.cmp(current.1))
                {
                    std::cmp::Ordering::Equal => {
                        return Err(BellaError::Duplicate {
                            field: "learner.route_skill",
                            value: format!(
                                "{}/{}:{}",
                                current.0.provider_id(),
                                current.0.model_id(),
                                current.1
                            ),
                        });
                    }
                    std::cmp::Ordering::Greater => {
                        return Err(BellaError::NonCanonical {
                            field: "learner.posteriors",
                        });
                    }
                    std::cmp::Ordering::Less => {}
                }
            }
            previous = Some(current);
        }
        Ok(())
    }

    fn compute_training_content_hash(&self) -> Result<String, BellaError> {
        let payload = serde_json::to_vec(&LearnerHashPayload {
            taxonomy: &self.taxonomy,
            profiler: &self.profiler,
            critic_version: &self.critic_version,
            evaluator_version: &self.evaluator_version,
            corpus_version: &self.corpus_version,
            catalog_version: &self.catalog_version,
            estimator_policy: &self.estimator_policy,
            posteriors: &self.posteriors,
        })
        .map_err(|error| BellaError::Serialization {
            reason: error.to_string(),
        })?;
        Ok(format!("{:x}", Sha256::digest(payload)))
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum BellaUnavailableReason {
    RepositoryUnavailable,
    InvalidState,
    EvaluatorVersionMismatch,
    CatalogVersionMismatch,
    PriceVersionMismatch,
    ProductionPromotionBlocked,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct FrozenBellaUnavailable {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub state_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub taxonomy_version: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub critic_version: Option<String>,
    pub reason: BellaUnavailableReason,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct FrozenBellaAbstention {
    pub state_id: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub critic_version: Option<String>,
    pub profile: ProfileAbstention,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct FrozenBellaActive {
    pub state_id: String,
    pub training_content_hash: String,
    pub critic_version: String,
    pub evaluator_version: String,
    pub corpus_version: String,
    pub catalog_version: String,
    pub estimator_policy: BellaEstimatorPolicy,
    pub profile: FrozenTaskSkillProfile,
    pub estimates: Vec<SkillEstimate>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum FrozenBellaEvidence {
    Active(FrozenBellaActive),
    Abstained(FrozenBellaAbstention),
    Unavailable(FrozenBellaUnavailable),
    Inactive { reason: BellaInactiveReason },
}

impl FrozenBellaEvidence {
    pub fn inactive(reason: BellaInactiveReason) -> Self {
        Self::Inactive { reason }
    }

    pub fn is_inactive(&self) -> bool {
        matches!(self, Self::Inactive { .. })
    }

    pub fn is_compatible_with(&self, catalog_version: &str, evaluator_version: &str) -> bool {
        match self {
            Self::Active(active) => {
                active.catalog_version == catalog_version
                    && active.evaluator_version == evaluator_version
            }
            Self::Inactive { .. } | Self::Abstained(_) | Self::Unavailable(_) => true,
        }
    }

    pub fn critic_version(&self) -> Option<&str> {
        match self {
            Self::Active(active) => Some(&active.critic_version),
            Self::Abstained(abstention) => abstention.critic_version.as_deref(),
            Self::Unavailable(unavailable) => unavailable.critic_version.as_deref(),
            Self::Inactive { .. } => None,
        }
    }

    pub fn validate(&self) -> Result<(), BellaError> {
        match self {
            Self::Inactive { .. } => Ok(()),
            Self::Unavailable(unavailable) => validate_unavailable(unavailable),
            Self::Abstained(abstained) => {
                require_nonblank("frozen.state_id", &abstained.state_id)?;
                require_optional_nonblank(
                    "frozen.critic_version",
                    abstained.critic_version.as_deref(),
                )?;
                validate_profile_abstention(&abstained.profile)
            }
            Self::Active(active) => active.validate(),
        }
    }

    pub fn capability_decision(
        &self,
        hard_eligible: &[RouteIdentity],
    ) -> Result<BellaCapabilityDecision, BellaError> {
        match self {
            Self::Inactive { reason } => Ok(BellaCapabilityDecision::fallback(
                BellaUseStatus::Inactive,
                hard_eligible,
                0,
                (*reason).into(),
            )),
            Self::Unavailable(unavailable) => Ok(BellaCapabilityDecision::fallback(
                BellaUseStatus::Unavailable,
                hard_eligible,
                0,
                unavailable.reason.into(),
            )),
            Self::Abstained(abstention) => Ok(BellaCapabilityDecision::fallback(
                BellaUseStatus::Abstained,
                hard_eligible,
                0,
                abstention.profile.reason.into(),
            )),
            Self::Active(active) => {
                let assessments: Vec<BellaRouteAssessment> = hard_eligible
                    .iter()
                    .map(|route| active.assess_route(route))
                    .collect();
                let capable: Vec<RouteIdentity> = assessments
                    .iter()
                    .filter(|assessment| assessment.capability == BellaRouteCapability::Capable)
                    .map(|assessment| assessment.route.clone())
                    .collect();
                if capable.is_empty() {
                    let dominant_reason = dominant_assessment_fallback_reason(&assessments);
                    Ok(BellaCapabilityDecision::with_assessments(
                        BellaUseStatus::NoCapableFallback,
                        hard_eligible.to_vec(),
                        active.profile.requirements.len(),
                        assessments,
                        Some(dominant_reason),
                    ))
                } else {
                    Ok(BellaCapabilityDecision::with_assessments(
                        BellaUseStatus::Applied,
                        capable,
                        active.profile.requirements.len(),
                        assessments,
                        None,
                    ))
                }
            }
        }
    }
}

impl FrozenBellaActive {
    fn validate(&self) -> Result<(), BellaError> {
        require_nonblank("frozen.state_id", &self.state_id)?;
        require_sha256("frozen.training_content_hash", &self.training_content_hash)?;
        require_nonblank("frozen.critic_version", &self.critic_version)?;
        require_nonblank("frozen.evaluator_version", &self.evaluator_version)?;
        require_nonblank("frozen.corpus_version", &self.corpus_version)?;
        require_nonblank("frozen.catalog_version", &self.catalog_version)?;
        self.estimator_policy.validate()?;
        validate_task_profile(&self.profile)?;

        let required_skills: BTreeSet<&str> = self
            .profile
            .requirements
            .iter()
            .map(|requirement| requirement.skill_id.as_str())
            .collect();
        let mut route_skills = Vec::with_capacity(self.estimates.len());
        for estimate in &self.estimates {
            estimate.validate(&self.estimator_policy)?;
            if estimate.taxonomy_version() != self.profile.taxonomy_version {
                return Err(BellaError::VersionMismatch {
                    field: "frozen.estimate_taxonomy_version",
                });
            }
            if !required_skills.contains(estimate.skill_id()) {
                return Err(BellaError::UnknownSkill {
                    skill_id: estimate.skill_id().to_owned(),
                });
            }
            let identity = (estimate.route().clone(), estimate.skill_id().to_owned());
            if route_skills.last() == Some(&identity) {
                return Err(BellaError::Duplicate {
                    field: "frozen.route_skill",
                    value: format!(
                        "{}/{}:{}",
                        estimate.route().provider_id(),
                        estimate.route().model_id(),
                        estimate.skill_id()
                    ),
                });
            }
            if route_skills
                .last()
                .is_some_and(|previous| previous > &identity)
            {
                return Err(BellaError::NonCanonical {
                    field: "frozen.estimates",
                });
            }
            route_skills.push(identity);
        }
        Ok(())
    }

    fn assess_route(&self, route: &RouteIdentity) -> BellaRouteAssessment {
        let skills: Vec<BellaSkillAssessment> = self
            .profile
            .requirements
            .iter()
            .map(|requirement| {
                let outcome =
                    self.estimates
                        .iter()
                        .find(|estimate| {
                            estimate.route() == route && estimate.skill_id() == requirement.skill_id
                        })
                        .map_or(BellaSkillAssessmentOutcome::MissingEvidence, |estimate| {
                            match estimate {
                                SkillEstimate::Estimated {
                                    support,
                                    conservative_proficiency,
                                    ..
                                } => {
                                    let margin =
                                        *conservative_proficiency - requirement.minimum_proficiency;
                                    if margin >= 0.0 {
                                        BellaSkillAssessmentOutcome::MeetsThreshold {
                                            support: *support,
                                            conservative_proficiency: *conservative_proficiency,
                                            margin,
                                        }
                                    } else {
                                        BellaSkillAssessmentOutcome::BelowThreshold {
                                            support: *support,
                                            conservative_proficiency: *conservative_proficiency,
                                            margin,
                                        }
                                    }
                                }
                                SkillEstimate::Abstained {
                                    support, reason, ..
                                } => BellaSkillAssessmentOutcome::Abstained {
                                    support: *support,
                                    reason: *reason,
                                },
                            }
                        });
                BellaSkillAssessment {
                    skill_id: requirement.skill_id.clone(),
                    minimum_proficiency: requirement.minimum_proficiency,
                    outcome,
                }
            })
            .collect();
        let capability = if skills.iter().all(|skill| {
            matches!(
                skill.outcome,
                BellaSkillAssessmentOutcome::MeetsThreshold { .. }
            )
        }) {
            BellaRouteCapability::Capable
        } else {
            BellaRouteCapability::NotCapable
        };
        BellaRouteAssessment {
            route: route.clone(),
            capability,
            skills,
        }
    }
}

impl Default for FrozenBellaEvidence {
    fn default() -> Self {
        Self::inactive(BellaInactiveReason::NoActiveState)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum BellaInactiveReason {
    NoActiveState,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum BellaUseStatus {
    Applied,
    Inactive,
    Abstained,
    Unavailable,
    NoCapableFallback,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum BellaFallbackReason {
    NoActiveState,
    RepositoryUnavailable,
    InvalidState,
    EvaluatorVersionMismatch,
    CatalogVersionMismatch,
    PriceVersionMismatch,
    ProductionPromotionBlocked,
    NoVocabularyOverlap,
    AmbiguousSkillMatch,
    BelowActivationThreshold,
    MissingEvidence,
    InsufficientSupport,
    BelowProficiencyThreshold,
}

impl From<BellaInactiveReason> for BellaFallbackReason {
    fn from(reason: BellaInactiveReason) -> Self {
        match reason {
            BellaInactiveReason::NoActiveState => Self::NoActiveState,
        }
    }
}

impl From<BellaUnavailableReason> for BellaFallbackReason {
    fn from(reason: BellaUnavailableReason) -> Self {
        match reason {
            BellaUnavailableReason::RepositoryUnavailable => Self::RepositoryUnavailable,
            BellaUnavailableReason::InvalidState => Self::InvalidState,
            BellaUnavailableReason::EvaluatorVersionMismatch => Self::EvaluatorVersionMismatch,
            BellaUnavailableReason::CatalogVersionMismatch => Self::CatalogVersionMismatch,
            BellaUnavailableReason::PriceVersionMismatch => Self::PriceVersionMismatch,
            BellaUnavailableReason::ProductionPromotionBlocked => Self::ProductionPromotionBlocked,
        }
    }
}

impl From<ProfileAbstentionReason> for BellaFallbackReason {
    fn from(reason: ProfileAbstentionReason) -> Self {
        match reason {
            ProfileAbstentionReason::NoVocabularyOverlap => Self::NoVocabularyOverlap,
            ProfileAbstentionReason::AmbiguousSkillMatch => Self::AmbiguousSkillMatch,
            ProfileAbstentionReason::BelowActivationThreshold => Self::BelowActivationThreshold,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct BellaUseSummary {
    pub status: BellaUseStatus,
    pub required_skill_count: usize,
    pub capable_route_count: usize,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub dominant_fallback_reason: Option<BellaFallbackReason>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum BellaRouteCapability {
    Capable,
    NotCapable,
    NotAssessed,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum BellaSkillAssessmentOutcome {
    MeetsThreshold {
        support: u64,
        conservative_proficiency: f64,
        margin: f64,
    },
    BelowThreshold {
        support: u64,
        conservative_proficiency: f64,
        margin: f64,
    },
    Abstained {
        support: u64,
        reason: SkillEstimateAbstentionReason,
    },
    MissingEvidence,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct BellaSkillAssessment {
    skill_id: String,
    minimum_proficiency: f64,
    outcome: BellaSkillAssessmentOutcome,
}

impl BellaSkillAssessment {
    pub fn skill_id(&self) -> &str {
        &self.skill_id
    }

    pub fn minimum_proficiency(&self) -> f64 {
        self.minimum_proficiency
    }

    pub fn outcome(&self) -> &BellaSkillAssessmentOutcome {
        &self.outcome
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct BellaRouteAssessment {
    route: RouteIdentity,
    capability: BellaRouteCapability,
    skills: Vec<BellaSkillAssessment>,
}

impl BellaRouteAssessment {
    fn not_assessed(route: RouteIdentity) -> Self {
        Self {
            route,
            capability: BellaRouteCapability::NotAssessed,
            skills: Vec::new(),
        }
    }

    pub fn route(&self) -> &RouteIdentity {
        &self.route
    }

    pub fn capability(&self) -> BellaRouteCapability {
        self.capability
    }

    pub fn skills(&self) -> &[BellaSkillAssessment] {
        &self.skills
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct BellaCapabilityDecision {
    selected_routes: Vec<RouteIdentity>,
    assessments: Vec<BellaRouteAssessment>,
    summary: BellaUseSummary,
}

impl BellaCapabilityDecision {
    fn with_assessments(
        status: BellaUseStatus,
        selected_routes: Vec<RouteIdentity>,
        required_skill_count: usize,
        assessments: Vec<BellaRouteAssessment>,
        dominant_fallback_reason: Option<BellaFallbackReason>,
    ) -> Self {
        let capable_route_count = if status == BellaUseStatus::Applied {
            selected_routes.len()
        } else {
            0
        };
        Self {
            selected_routes,
            assessments,
            summary: BellaUseSummary {
                status,
                required_skill_count,
                capable_route_count,
                dominant_fallback_reason,
            },
        }
    }

    fn fallback(
        status: BellaUseStatus,
        hard_eligible: &[RouteIdentity],
        required_skill_count: usize,
        dominant_fallback_reason: BellaFallbackReason,
    ) -> Self {
        Self::with_assessments(
            status,
            hard_eligible.to_vec(),
            required_skill_count,
            hard_eligible
                .iter()
                .cloned()
                .map(BellaRouteAssessment::not_assessed)
                .collect(),
            Some(dominant_fallback_reason),
        )
    }

    pub fn status(&self) -> BellaUseStatus {
        self.summary.status
    }

    pub fn selected_routes(&self) -> &[RouteIdentity] {
        &self.selected_routes
    }

    pub fn assessments(&self) -> &[BellaRouteAssessment] {
        &self.assessments
    }

    pub fn summary(&self) -> &BellaUseSummary {
        &self.summary
    }

    pub fn is_authorization_neutral(&self) -> bool {
        matches!(
            self.status(),
            BellaUseStatus::Inactive | BellaUseStatus::Abstained | BellaUseStatus::Unavailable
        )
    }
}

fn dominant_assessment_fallback_reason(
    assessments: &[BellaRouteAssessment],
) -> BellaFallbackReason {
    let mut missing = 0_usize;
    let mut insufficient = 0_usize;
    let mut below_threshold = 0_usize;
    for skill in assessments
        .iter()
        .flat_map(|assessment| assessment.skills())
    {
        match skill.outcome() {
            BellaSkillAssessmentOutcome::MeetsThreshold { .. } => {}
            BellaSkillAssessmentOutcome::BelowThreshold { .. } => below_threshold += 1,
            BellaSkillAssessmentOutcome::Abstained {
                reason: SkillEstimateAbstentionReason::InsufficientSupport,
                ..
            } => insufficient += 1,
            BellaSkillAssessmentOutcome::MissingEvidence => missing += 1,
        }
    }

    [
        (BellaFallbackReason::MissingEvidence, missing, 3_usize),
        (BellaFallbackReason::InsufficientSupport, insufficient, 2),
        (
            BellaFallbackReason::BelowProficiencyThreshold,
            below_threshold,
            1,
        ),
    ]
    .into_iter()
    .max_by_key(|(_, count, priority)| (*count, *priority))
    .map(|(reason, _, _)| reason)
    .unwrap_or(BellaFallbackReason::MissingEvidence)
}

fn validate_profile_abstention(abstention: &ProfileAbstention) -> Result<(), BellaError> {
    require_nonblank("profile.profiler_version", &abstention.profiler_version)?;
    require_nonblank("profile.taxonomy_version", &abstention.taxonomy_version)?;
    require_sha256("profile.task_fingerprint", &abstention.task_fingerprint)
}

fn validate_unavailable(unavailable: &FrozenBellaUnavailable) -> Result<(), BellaError> {
    require_optional_nonblank("frozen.state_id", unavailable.state_id.as_deref())?;
    require_optional_nonblank(
        "frozen.taxonomy_version",
        unavailable.taxonomy_version.as_deref(),
    )?;
    require_optional_nonblank(
        "frozen.critic_version",
        unavailable.critic_version.as_deref(),
    )?;
    if matches!(
        unavailable.reason,
        BellaUnavailableReason::EvaluatorVersionMismatch
            | BellaUnavailableReason::CatalogVersionMismatch
            | BellaUnavailableReason::PriceVersionMismatch
            | BellaUnavailableReason::ProductionPromotionBlocked
    ) {
        require_nonblank(
            "frozen.state_id",
            unavailable.state_id.as_deref().unwrap_or_default(),
        )?;
        require_nonblank(
            "frozen.taxonomy_version",
            unavailable.taxonomy_version.as_deref().unwrap_or_default(),
        )?;
    }
    Ok(())
}

fn validate_task_profile(profile: &FrozenTaskSkillProfile) -> Result<(), BellaError> {
    require_nonblank("profile.profiler_version", &profile.profiler_version)?;
    if profile.profiler_version != PROFILER_VERSION {
        return Err(BellaError::VersionMismatch {
            field: "profile.profiler_version",
        });
    }
    require_nonblank("profile.taxonomy_version", &profile.taxonomy_version)?;
    require_sha256("profile.task_fingerprint", &profile.task_fingerprint)?;
    if profile.requirements.is_empty() {
        return Err(BellaError::Empty {
            field: "profile.requirements",
        });
    }
    let mut skill_ids = BTreeSet::new();
    let mut skill_indices = BTreeSet::new();
    for requirement in &profile.requirements {
        if usize::from(requirement.skill_index) >= SKILL_DIMENSIONS {
            return Err(BellaError::SkillIndexOverflow {
                index: requirement.skill_index,
            });
        }
        require_nonblank("profile.skill_id", &requirement.skill_id)?;
        require_nonblank("profile.skill_name", &requirement.skill_name)?;
        require_probability(
            "profile.minimum_proficiency",
            requirement.minimum_proficiency,
        )?;
        require_probability("profile.similarity", requirement.similarity)?;
        require_nonblank("profile.rationale", &requirement.rationale)?;
        if requirement.rationale.len() > MAX_RATIONALE_BYTES {
            return Err(BellaError::OutOfRange {
                field: "profile.rationale",
            });
        }
        if !skill_ids.insert(requirement.skill_id.clone()) {
            return Err(BellaError::Duplicate {
                field: "profile.skill_id",
                value: requirement.skill_id.clone(),
            });
        }
        if !skill_indices.insert(requirement.skill_index) {
            return Err(BellaError::Duplicate {
                field: "profile.skill_index",
                value: requirement.skill_index.to_string(),
            });
        }
    }
    Ok(())
}

fn require_nonblank(field: &'static str, value: &str) -> Result<(), BellaError> {
    if value.trim().is_empty() {
        Err(BellaError::Blank { field })
    } else {
        Ok(())
    }
}

fn require_optional_nonblank(field: &'static str, value: Option<&str>) -> Result<(), BellaError> {
    value.map_or(Ok(()), |value| require_nonblank(field, value))
}

fn nonblank_owned(value: String) -> Option<String> {
    (!value.trim().is_empty()).then_some(value)
}

fn require_sha256(field: &'static str, value: &str) -> Result<(), BellaError> {
    if value.len() != 64 || !value.bytes().all(|byte| byte.is_ascii_hexdigit()) {
        Err(BellaError::OutOfRange { field })
    } else {
        Ok(())
    }
}

fn require_probability(field: &'static str, value: f64) -> Result<(), BellaError> {
    if !value.is_finite() {
        Err(BellaError::NonFinite { field })
    } else if !(0.0..=1.0).contains(&value) {
        Err(BellaError::OutOfRange { field })
    } else {
        Ok(())
    }
}

fn require_positive_finite(field: &'static str, value: f64) -> Result<(), BellaError> {
    if !value.is_finite() {
        Err(BellaError::NonFinite { field })
    } else if value <= 0.0 {
        Err(BellaError::OutOfRange { field })
    } else {
        Ok(())
    }
}

fn require_nonnegative_finite(field: &'static str, value: f64) -> Result<(), BellaError> {
    if !value.is_finite() {
        Err(BellaError::NonFinite { field })
    } else if value < 0.0 {
        Err(BellaError::OutOfRange { field })
    } else {
        Ok(())
    }
}

fn reject_duplicate<I>(values: I, field: &'static str) -> Result<(), BellaError>
where
    I: IntoIterator<Item = String>,
{
    let mut seen = BTreeSet::new();
    for value in values {
        if !seen.insert(value.clone()) {
            return Err(BellaError::Duplicate { field, value });
        }
    }
    Ok(())
}

fn bounded_tokens(input: &str) -> Vec<String> {
    input
        .split(|character: char| !character.is_alphanumeric() && character != '_')
        .filter(|token| !token.is_empty())
        .map(str::to_lowercase)
        .take(MAX_PROFILE_TOKENS)
        .collect()
}

fn normalized_term_vector(tokens: &[String], vocabulary_index: &BTreeMap<&str, usize>) -> Vec<f64> {
    let mut values = vec![0.0; vocabulary_index.len()];
    for token in tokens {
        if let Some(index) = vocabulary_index.get(token.as_str()) {
            values[*index] += 1.0;
        }
    }
    normalize(&mut values);
    values
}

fn normalize(values: &mut [f64]) {
    let norm = vector_norm(values);
    if norm > 0.0 {
        for value in values {
            *value /= norm;
        }
    }
}

fn vector_norm(values: &[f64]) -> f64 {
    values.iter().map(|value| value * value).sum::<f64>().sqrt()
}

fn dot(left: &[f64], right: &[f64]) -> f64 {
    left.iter()
        .zip(right)
        .map(|(left, right)| left * right)
        .sum()
}

fn fingerprint_tokens(tokens: &[String]) -> String {
    let mut digest = Sha256::new();
    digest.update(b"meridian-profile-task/v1");
    for token in tokens {
        digest.update((token.len() as u64).to_be_bytes());
        digest.update(token.as_bytes());
    }
    format!("{:x}", digest.finalize())
}

fn rationale(
    task_vector: &[f64],
    centroid: &[f64],
    vocabulary: &[String],
    term_limit: usize,
) -> String {
    let mut contributions: Vec<(&str, f64)> = vocabulary
        .iter()
        .zip(task_vector.iter().zip(centroid))
        .filter_map(|(term, (task_weight, skill_weight))| {
            let contribution = task_weight * skill_weight;
            (contribution > 0.0).then_some((term.as_str(), contribution))
        })
        .collect();
    contributions.sort_unstable_by(|(left_term, left), (right_term, right)| {
        right
            .total_cmp(left)
            .then_with(|| left_term.cmp(right_term))
    });
    let terms = contributions
        .into_iter()
        .take(term_limit)
        .map(|(term, _)| term)
        .collect::<Vec<_>>()
        .join(", ");
    truncate_utf8(&format!("shared terms: {terms}"), MAX_RATIONALE_BYTES)
}

fn truncate_utf8(value: &str, maximum_bytes: usize) -> String {
    if value.len() <= maximum_bytes {
        return value.to_owned();
    }
    let boundary = value
        .char_indices()
        .map(|(index, _)| index)
        .take_while(|index| *index <= maximum_bytes)
        .last()
        .unwrap_or(0);
    value[..boundary].to_owned()
}
