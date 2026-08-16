//! Immutable, content-addressed inputs for one routing decision.

use serde::{Deserialize, Serialize};
use serde_json::Value;
use sha2::{Digest, Sha256};
use thiserror::Error;

use super::{CatalogSnapshot, EvidenceSnapshot, ProjectPolicy, RoutingContext};

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum SnapshotDegradation {
    AdvisoryEvidenceUnavailable {
        source: String,
        fallback_version: String,
    },
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct RoutingInputSnapshot {
    pub schema_version: String,
    pub project_id: String,
    pub request: RoutingContext,
    pub policy: ProjectPolicy,
    pub catalog: CatalogSnapshot,
    pub evidence: EvidenceSnapshot,
    pub feature_version: String,
    pub degradations: Vec<SnapshotDegradation>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FrozenRoutingInput {
    pub fingerprint: String,
    pub canonical_payload: Vec<u8>,
}

#[derive(Clone, Debug, Error, PartialEq, Eq)]
pub enum SnapshotError {
    #[error("routing input has blank authoritative identifier: {field}")]
    BlankIdentifier { field: &'static str },
    #[error("routing input contains non-finite value at {field}")]
    NonFinite { field: &'static str },
    #[error("routing input has invalid complexity evidence: {reason}")]
    InvalidComplexity { reason: String },
    #[error("routing input has invalid BELLA evidence: {reason}")]
    InvalidBella { reason: String },
    #[error("routing input has invalid R2 evidence: {reason}")]
    InvalidR2 { reason: String },
    #[error("routing input has invalid compound evidence: {reason}")]
    InvalidCompound { reason: String },
    #[error("routing input cannot be serialized: {reason}")]
    Serialization { reason: String },
}

impl RoutingInputSnapshot {
    pub fn freeze(&self) -> Result<FrozenRoutingInput, SnapshotError> {
        if self.project_id.trim().is_empty() {
            return Err(SnapshotError::BlankIdentifier {
                field: "project_id",
            });
        }
        if !self.policy.cost_weight.is_finite() {
            return Err(SnapshotError::NonFinite {
                field: "policy.cost_weight",
            });
        }
        if matches!(
            self.schema_version.as_str(),
            "routing-input/v5" | "routing-input/v6" | "routing-input/v7" | "routing-input/v8"
        ) {
            let evidence = self.evidence.complexity.as_ref().ok_or_else(|| {
                SnapshotError::InvalidComplexity {
                    reason: "missing evidence".to_string(),
                }
            })?;
            evidence
                .validate()
                .map_err(|error| SnapshotError::InvalidComplexity {
                    reason: error.to_string(),
                })?;
            if (evidence.score - self.request.complexity).abs() > 1e-6 {
                return Err(SnapshotError::InvalidComplexity {
                    reason: "request.complexity does not match evidence.score".to_string(),
                });
            }
            if evidence.estimated_input_tokens != self.request.estimated_input_tokens {
                return Err(SnapshotError::InvalidComplexity {
                    reason: "request tokens do not match complexity evidence".to_string(),
                });
            }
        }
        match self.schema_version.as_str() {
            "routing-input/v6" => {
                if self.evidence.bella.is_inactive() {
                    return Err(SnapshotError::InvalidBella {
                        reason: "routing-input/v6 requires non-inactive BELLA evidence".to_string(),
                    });
                }
                validate_bella(self)?;
                require_inactive_r2(self, "routing-input/v7")?;
                require_inactive_compound(self)?;
            }
            "routing-input/v7" => {
                if !self.evidence.bella.is_inactive() {
                    validate_bella(self)?;
                }
                if self.evidence.r2.is_inactive() {
                    return Err(SnapshotError::InvalidR2 {
                        reason: "routing-input/v7 requires non-inactive R2 evidence".to_string(),
                    });
                }
                self.evidence
                    .r2
                    .validate()
                    .map_err(|error| SnapshotError::InvalidR2 {
                        reason: error.to_string(),
                    })?;
                if self.evidence.r2.provenance().is_some()
                    && !self.evidence.r2.is_compatible_with(
                        &self.evidence.catalog_version,
                        &self.evidence.price_version,
                        &self.evidence.evaluator_version,
                    )
                {
                    return Err(SnapshotError::InvalidR2 {
                        reason: "catalog, price, evaluator, instruction, feature, or label version mismatch"
                            .to_string(),
                        });
                }
                require_inactive_compound(self)?;
            }
            "routing-input/v8" => {
                if !self.evidence.bella.is_inactive() {
                    validate_bella(self)?;
                }
                if !self.evidence.r2.is_inactive() {
                    self.evidence
                        .r2
                        .validate()
                        .map_err(|error| SnapshotError::InvalidR2 {
                            reason: error.to_string(),
                        })?;
                    if self.evidence.r2.provenance().is_some()
                        && !self.evidence.r2.is_compatible_with(
                            &self.evidence.catalog_version,
                            &self.evidence.price_version,
                            &self.evidence.evaluator_version,
                        )
                    {
                        return Err(SnapshotError::InvalidR2 {
                            reason: "catalog, price, evaluator, instruction, feature, or label version mismatch"
                                .to_string(),
                        });
                    }
                }
                if self.evidence.compound.is_inactive() {
                    return Err(SnapshotError::InvalidCompound {
                        reason: "routing-input/v8 requires non-inactive compound evidence"
                            .to_string(),
                    });
                }
                self.evidence.compound.validate().map_err(|error| {
                    SnapshotError::InvalidCompound {
                        reason: error.to_string(),
                    }
                })?;
            }
            _ => {
                if !self.evidence.bella.is_inactive() {
                    return Err(SnapshotError::InvalidBella {
                        reason: "non-inactive BELLA evidence requires routing-input/v6 or v7"
                            .to_string(),
                    });
                }
                require_inactive_r2(self, "routing-input/v7")?;
                require_inactive_compound(self)?;
            }
        }

        let value = serde_json::to_value(self).map_err(|error| SnapshotError::Serialization {
            reason: error.to_string(),
        })?;
        let canonical_payload = serde_json::to_vec(&canonicalize(value)).map_err(|error| {
            SnapshotError::Serialization {
                reason: error.to_string(),
            }
        })?;
        let fingerprint = format!("{:x}", Sha256::digest(&canonical_payload));

        Ok(FrozenRoutingInput {
            fingerprint,
            canonical_payload,
        })
    }
}

fn validate_bella(snapshot: &RoutingInputSnapshot) -> Result<(), SnapshotError> {
    snapshot
        .evidence
        .bella
        .validate()
        .map_err(|error| SnapshotError::InvalidBella {
            reason: error.to_string(),
        })?;
    if !snapshot.evidence.bella.is_compatible_with(
        &snapshot.evidence.catalog_version,
        &snapshot.evidence.evaluator_version,
    ) {
        return Err(SnapshotError::InvalidBella {
            reason: "catalog or evaluator version mismatch".to_string(),
        });
    }
    Ok(())
}

fn require_inactive_r2(
    snapshot: &RoutingInputSnapshot,
    required_schema: &'static str,
) -> Result<(), SnapshotError> {
    if snapshot.evidence.r2.is_inactive() {
        Ok(())
    } else {
        Err(SnapshotError::InvalidR2 {
            reason: format!("non-inactive R2 evidence requires {required_schema}"),
        })
    }
}

fn require_inactive_compound(snapshot: &RoutingInputSnapshot) -> Result<(), SnapshotError> {
    if snapshot.evidence.compound.is_inactive() {
        Ok(())
    } else {
        Err(SnapshotError::InvalidCompound {
            reason: "non-inactive compound evidence requires routing-input/v8".to_string(),
        })
    }
}

fn canonicalize(value: Value) -> Value {
    match value {
        Value::Array(values) => Value::Array(values.into_iter().map(canonicalize).collect()),
        Value::Object(values) => {
            let mut entries: Vec<_> = values.into_iter().collect();
            entries.sort_unstable_by(|(left, _), (right, _)| left.cmp(right));
            Value::Object(
                entries
                    .into_iter()
                    .map(|(key, value)| (key, canonicalize(value)))
                    .collect(),
            )
        }
        scalar => scalar,
    }
}
