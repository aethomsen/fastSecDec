use super::{ReplicaMoments, SerialTask, streams::Streams};
use crate::integration::{
    AccuracyTarget, IntegrationError, IntegrationProblem, QmcSettings, Result, RuleSource,
    Tolerance, VectorEstimate, mc::HavanaSettings,
};
use numerica::numerical_integration::ContinuousGrid;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum SerialMethod {
    Qmc(QmcSettings),
    AdaptiveQmc(QmcSettings),
    Mc(HavanaSettings),
    AdaptiveMc(HavanaSettings),
}
impl SerialMethod {
    pub(super) fn pilot(&self) -> bool {
        matches!(self, Self::AdaptiveQmc(_) | Self::AdaptiveMc(_))
    }
    pub(super) fn points(&self) -> u64 {
        match self {
            Self::Qmc(s) | Self::AdaptiveQmc(s) => s.points,
            Self::Mc(s) | Self::AdaptiveMc(s) => s.points_per_batch as u64,
        }
    }
    pub(super) fn replicas(&self) -> u64 {
        match self {
            Self::Qmc(s) | Self::AdaptiveQmc(s) => s.shifts.into(),
            Self::Mc(s) | Self::AdaptiveMc(s) => s.batches.into(),
        }
    }
    pub(super) fn seed(&self) -> u64 {
        match self {
            Self::Qmc(s) | Self::AdaptiveQmc(s) => s.seed,
            Self::Mc(s) | Self::AdaptiveMc(s) => s.seed,
        }
    }
    pub(super) fn grid(&self, dimension: usize) -> Result<Option<ContinuousGrid<f64>>> {
        match self {
            Self::Mc(s) | Self::AdaptiveMc(s) => Ok(Some(s.grid(dimension)?)),
            _ => Ok(None),
        }
    }
    pub(super) fn validate(&self) -> Result<()> {
        match self {
            Self::Qmc(s) | Self::AdaptiveQmc(s) => s.validate(),
            Self::Mc(s) | Self::AdaptiveMc(s) => s.validate(),
        }
    }
    pub(super) fn next_points(&self, points: u64, double: bool) -> Result<u64> {
        if !double {
            return Ok(points);
        }
        let maximum = match self {
            Self::Qmc(s) | Self::AdaptiveQmc(s) => match s.rule {
                RuleSource::Kuo => fastsecdec_qmc::PublishedLattice::Kuo33002.max_points(),
                RuleSource::Published(c) => c.max_points(),
                // Rank1Rule requires exactly representable f64 indices.
                RuleSource::Supplied(_) => 1 << 53,
            },
            _ => u64::MAX,
        };
        if points >= maximum {
            return Ok(points);
        }
        points
            .checked_mul(2)
            .filter(|&p| p <= maximum)
            .ok_or_else(|| IntegrationError::Invalid("serial point refinement overflow".into()))
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SerialSettings {
    pub method: SerialMethod,
    pub double_points: bool,
    /// Initial production is round one. None permits unbounded refinement.
    pub max_rounds: Option<usize>,
    /// Bound all live/retry reservations, independently of run duration.
    pub max_in_flight: usize,
    pub target: AccuracyTarget,
    pub tolerance: Tolerance,
    pub pilot_iterations: u32,
    pub learning_rate: f64,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct AcceptedAllocation {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub(crate) weights: Option<crate::integration::weights::WeightStatistics>,
    pub estimate: VectorEstimate,
    pub(super) moments: ReplicaMoments,
    pub replicas: u64,
    pub points_per_replica: u64,
    pub epoch: u64,
    pub round: usize,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub(super) struct SectorState {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub(crate) weights: Option<crate::integration::weights::WeightStatistics>,
    pub id: u64,
    pub dimension: usize,
    pub pilot: bool,
    pub pilot_iteration: u32,
    pub epoch: u64,
    pub round: usize,
    pub points: u64,
    pub target: u64,
    pub next_replica: u64,
    pub moments: ReplicaMoments,
    pub previous: Option<AcceptedAllocation>,
    pub grid: Option<ContinuousGrid<f64>>,
    pub training: Option<ContinuousGrid<f64>>,
    pub first_visit: bool,
    pub worker_seconds: f64,
    pub accepted_points: u64,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub(super) struct Pending {
    pub task: SerialTask,
    pub issued: bool,
}

/// Metadata, compact moments and bounded reservations only: no kernels/pool.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SerialSession {
    pub(super) problem: IntegrationProblem,
    pub(super) settings: SerialSettings,
    pub(super) sectors: Vec<SectorState>,
    pub(super) streams: Streams,
    pub(super) pending: BTreeMap<u64, Pending>,
    pub(super) run: u64,
    pub(super) next_lease: u64,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SerialSectorSnapshot {
    pub id: u64,
    pub dimension: usize,
    pub pilot: bool,
    pub epoch: u64,
    pub round: usize,
    pub points_per_replica: u64,
    pub replicas: u64,
    pub target_replicas: u64,
    pub in_flight: usize,
    pub first_visit_complete: bool,
    pub allocation_complete: bool,
    pub exhausted: bool,
    pub worker_seconds: f64,
    pub accepted_points: u64,
    pub current: Option<VectorEstimate>,
    pub previous: Option<VectorEstimate>,
    pub priority: Option<f64>,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SerialSnapshot {
    pub sectors: Vec<SerialSectorSnapshot>,
    pub total: Option<VectorEstimate>,
    pub live_total: Option<VectorEstimate>,
    pub first_coverage_complete: bool,
    pub exhausted: bool,
    pub in_flight: usize,
}
