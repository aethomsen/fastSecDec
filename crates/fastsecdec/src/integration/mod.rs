//! Caller-driven integration of complete sector coefficient vectors.
//!
//! Scheduling and worker pools belong to the caller. Lattices, random shifts,
//! and partial statistics come from FastSecDec's QMC crate; ordinary Monte Carlo
//! grids and their adaptation reuse Numerica/Havana.
mod accuracy;
mod adaptive;
mod config;
mod contributions;
mod estimate;
mod estimate_norm;
pub use estimate_norm::ComplexNormUncertainty;
mod live;
pub mod mc;
pub mod mc_discrete;
mod mc_live;
mod observation;
mod qmc;
#[cfg(test)]
mod refinement_tests;
pub mod serial;
mod streams;
mod weights;
mod worker;
pub use weights::{WeightDiagnosticStatus, WeightDiagnostics, WeightOrderDiagnostics};

pub use crate::status::CoefficientComponent;
pub use accuracy::{AccuracyAssessment, AccuracyTarget};
pub use config::{
    IntegrationProblem, Periodization, QmcSettings, RuleSource, SectorSpec, Tolerance,
};
pub use contributions::{ContributionReport, ReplicaRelation, SectorContribution};
pub use estimate::VectorEstimate;
pub use fastsecdec_qmc::PublishedLattice;
pub use live::{
    LiveEstimate, LiveObservation, LiveSector, LiveSource, LiveStatus, OperationalMetrics,
    SectorOperationalMetrics,
};
pub use mc_live::{McLiveBatch, McLiveView, mc_live_observation};
pub use observation::IntegrationObservation;
pub use qmc::{ProductionAllocation, QmcDesign, QmcSession};
pub use worker::{QmcReturn, QmcTask, QmcWorker};

#[derive(Debug, thiserror::Error)]
pub enum IntegrationError {
    #[error("invalid integration configuration: {0}")]
    Invalid(String),
    #[error("invalid worker return: {0}")]
    InvalidReturn(String),
    #[error("integrand evaluation failed: {0}")]
    Evaluation(String),
    #[error(
        "weighted MC pilot envelope {weighted_envelope} cannot be represented by the native Havana training API at sampled weight {sampled_weight}; production accepts final weighted coefficients without inverse scaling"
    )]
    UnrepresentablePilotEnvelope {
        weighted_envelope: f64,
        sampled_weight: f64,
    },
    #[error("integration uncertainty is not available: {0}")]
    Unavailable(String),
    #[error("native numerical accumulation exceeded representable range")]
    NumericRange,
    #[error(transparent)]
    Qmc(#[from] fastsecdec_qmc::QmcError),
    #[error("invalid integration checkpoint: {0}")]
    Checkpoint(#[from] serde_json::Error),
}

impl IntegrationError {
    /// Whether an observation exceeded the native accumulator's numeric range.
    /// Callers may hide such provisional evidence without changing accepted work.
    pub fn is_statistical_range(&self) -> bool {
        matches!(
            self,
            Self::NumericRange
                | Self::Qmc(
                    fastsecdec_qmc::QmcError::NumericOverflow
                        | fastsecdec_qmc::QmcError::NonFiniteValue
                )
        )
    }
}

pub type Result<T> = std::result::Result<T, IntegrationError>;
