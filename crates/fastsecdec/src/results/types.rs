use crate::{
    integration::{ContributionReport, QmcDesign, SectorSpec, Tolerance},
    reference::{
        CoefficientKey, ComparisonContext, ReferenceComparison, ReferenceProvenance,
        ReferenceResult, ReferenceValidation,
    },
    status::{CoefficientComponent, EvaluationDiagnostics, StoppingReason},
};
use serde::{Deserialize, Serialize};

/// Caller declaration of the complete parent kernel, not an accumulation ID.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct KernelResultManifest {
    /// Separate threshold lineage qualification; absent for historical owners.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub threshold_scope: Option<crate::kernel::ThresholdResultScope>,
    /// Partial original geometry extent, independent of numerical sector IDs.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source_selection: Option<crate::generation::SourceSectorSelection>,
    pub kernel_content_id: String,
    pub orders: Vec<i32>,
    pub components: Vec<CoefficientComponent>,
    pub sectors: Vec<SectorSpec>,
    pub exact_coefficients: Vec<f64>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum ExactContributionPolicy {
    IncludeAll,
    ExcludeAll,
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub enum ResultScope {
    #[default]
    FullIntegral,
    SelectedSectors {
        sector_ids: Vec<u64>,
        exact_policy: ExactContributionPolicy,
    },
}

impl ResultScope {
    pub fn is_full_integral(&self) -> bool {
        matches!(self, Self::FullIntegral)
    }
}

impl std::fmt::Display for ResultScope {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::FullIntegral => f.write_str("full integral"),
            Self::SelectedSectors {
                sector_ids,
                exact_policy,
            } => {
                write!(
                    f,
                    "selected sectors {:?}",
                    &sector_ids[..sector_ids.len().min(6)]
                )?;
                if sector_ids.len() > 6 {
                    write!(f, " … ({} total)", sector_ids.len())?;
                }
                write!(
                    f,
                    "; folded exact contribution {}",
                    match exact_policy {
                        ExactContributionPolicy::IncludeAll => "included",
                        ExactContributionPolicy::ExcludeAll => "excluded",
                    }
                )
            }
        }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct StoredReference {
    pub reference: ReferenceResult,
    /// Historical context only. A new computation requires a fresh context.
    pub context: ComparisonContext,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ResultTimings {
    pub elapsed_seconds: Option<f64>,
    pub artifact_load_seconds: Option<f64>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SavedIntegrationResult {
    pub manifest: KernelResultManifest,
    pub scope: ResultScope,
    pub contributions: ContributionReport,
    /// Accepted-production absolute weights. Absent in historical results.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub weight_diagnostics: Option<crate::integration::WeightDiagnostics>,
    pub stopping_reason: StoppingReason,
    pub requested_tolerance: Option<Tolerance>,
    #[serde(
        default,
        skip_serializing_if = "crate::integration::AccuracyTarget::is_default"
    )]
    pub requested_accuracy_target: crate::integration::AccuracyTarget,
    pub evaluation_diagnostics: Option<EvaluationDiagnostics>,
    pub qmc_design: Option<QmcDesign>,
    pub provenance: ReferenceProvenance,
    pub validation: ReferenceValidation,
    pub stored_reference: Option<StoredReference>,
    pub timings: ResultTimings,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum ResultReferenceSelection {
    Estimate,
    StoredReference,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum ResultSectorSort {
    Id,
    Magnitude(CoefficientKey),
    StandardError(CoefficientKey),
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum ResultComparison {
    Compared(Box<ReferenceComparison>),
    Unavailable(ResultComparisonUnavailable),
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum ResultComparisonUnavailable {
    NoEstimate,
    NoReference,
    SelectedScope,
    Pilot,
    Cancelled,
    NumericalFailure,
    NumericRange {
        key: CoefficientKey,
        quantity: String,
    },
}
