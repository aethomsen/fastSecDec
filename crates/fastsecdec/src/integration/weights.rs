//! Accepted-production complex weight diagnostics. No samples or RNG state are retained.
use super::{IntegrationError, IntegrationProblem, Result, VectorEstimate};
use numerica::domains::float::{DoubleFloat, RealLike, SingleFloat};
use serde::{Deserialize, Serialize};

/// Availability of the same accepted allocation used by the native estimate.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum WeightDiagnosticStatus {
    Available,
    PilotOnly,
    WaitingForCoverage,
    /// Some accepted work predates the diagnostic payload; it cannot be reconstructed.
    LegacyUnavailable,
    NumericRange,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct WeightOrderDiagnostics {
    pub order: i32,
    pub stochastic_absolute_mean: Option<f64>,
    /// Magnitude of the combined exact coefficient, included once in the numerator.
    pub exact_absolute: Option<f64>,
    pub absolute_integral_estimate: Option<f64>,
    /// Observed maximum, retaining continuous importance and transform weights.
    /// Discrete sector-selection inverse probability is removed. Not a supremum.
    pub sampled_sector_local_maximum: Option<f64>,
    pub total_mean_magnitude: Option<f64>,
    pub cancellation_ratio: Option<f64>,
    pub maximum_ratio: Option<f64>,
}

/// Sector-decomposition and proposal-dependent diagnostics, never stopping criteria.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct WeightDiagnostics {
    pub status: WeightDiagnosticStatus,
    /// Accepted samples in the allocations selected for the reported estimate.
    pub accepted_points: u64,
    pub orders: Vec<WeightOrderDiagnostics>,
}

/// IntegrationProblem declares the complete common output layout. Omitted components
/// are structural zeros, including the Real-only constructor's certified real layout.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct WeightLayout {
    orders: Vec<i32>,
    indices: Vec<Vec<usize>>,
}
impl WeightLayout {
    pub(crate) fn new(problem: &IntegrationProblem) -> Self {
        Self::from_orders(&problem.orders)
    }
    fn from_orders(orders: &[i32]) -> Self {
        let mut groups = std::collections::BTreeMap::<i32, Vec<usize>>::new();
        for (i, &order) in orders.iter().enumerate() {
            groups.entry(order).or_default().push(i);
        }
        let (orders, indices) = groups.into_iter().unzip();
        Self { orders, indices }
    }
    pub(crate) fn len(&self) -> usize {
        self.orders.len()
    }
    pub(crate) fn validate(&self, outputs: usize) -> Result<()> {
        let mut indices = self.indices.iter().flatten().copied().collect::<Vec<_>>();
        indices.sort_unstable();
        if self.orders.is_empty()
            || self.orders.len() != self.indices.len()
            || self.orders.windows(2).any(|w| w[0] >= w[1])
            || self.indices.iter().any(|r| r.is_empty() || r.len() > 2)
            || indices != (0..outputs).collect::<Vec<_>>()
        {
            return Err(IntegrationError::InvalidReturn(
                "invalid complex weight layout".into(),
            ));
        }
        Ok(())
    }
    pub(crate) fn magnitudes(&self, values: &[f64], scale: f64, output: &mut Vec<f64>) {
        output.clear();
        for indices in &self.indices {
            output.push(
                indices
                    .iter()
                    .fold(0.0_f64, |a, &i| a.hypot(values[i] * scale)),
            );
        }
    }
}

/// Numerica double-float centered means, one complex magnitude/max per Laurent order.
/// Native StatisticsAccumulator unconditionally stores sums and square sums;
/// even centered native sums can overflow for finite nonnegative means. This
/// narrow mean-only state uses normalized weights and retains no error estimator.
/// Range failure invalidates diagnostics only, never the accepted integrand vector.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub(crate) struct WeightStatistics {
    pub(crate) points: u64,
    means: Vec<DoubleFloat>,
    maxima: Vec<f64>,
    finite: bool,
}
impl WeightStatistics {
    pub(crate) fn new(n: usize) -> Self {
        Self {
            points: 0,
            means: vec![DoubleFloat::from(0.0); n],
            maxima: vec![0.0; n],
            finite: true,
        }
    }
    pub(crate) fn push(&mut self, absolute: &[f64], local: &[f64]) -> Result<()> {
        self.points = self
            .points
            .checked_add(1)
            .ok_or(IntegrationError::NumericRange)?;
        if !self.finite {
            return Ok(());
        }
        for i in 0..self.means.len() {
            let delta = (DoubleFloat::from(absolute[i]) - self.means[i])
                / DoubleFloat::from(self.points as f64);
            self.means[i] += delta;
            self.maxima[i] = self.maxima[i].max(local[i]);
        }
        if absolute.iter().chain(local).any(|v| !v.is_finite())
            || self.means.iter().any(|v| !v.is_finite())
        {
            self.invalidate();
        }
        Ok(())
    }
    fn invalidate(&mut self) {
        self.finite = false;
        self.means.fill(DoubleFloat::from(0.0));
        self.maxima.fill(0.0);
    }
    pub(crate) fn validate(&self, n: usize, points: u64) -> Result<()> {
        if self.points != points
            || self.means.len() != n
            || self.maxima.len() != n
            || self
                .means
                .iter()
                .any(|v| !v.is_finite() || *v < DoubleFloat::from(0.0))
            || self.maxima.iter().any(|v| !v.is_finite() || *v < 0.0)
            || (points == 0 || !self.finite)
                && (self.means.iter().any(|v| *v != DoubleFloat::from(0.0))
                    || self.maxima.iter().any(|v| *v != 0.0))
        {
            return Err(IntegrationError::InvalidReturn(
                "invalid accepted weight statistics".into(),
            ));
        }
        Ok(())
    }
    pub(crate) fn merge(&mut self, other: &Self) -> Result<()> {
        other.validate(self.means.len(), other.points)?;
        if other.points == 0 {
            return Ok(());
        }
        self.points = self
            .points
            .checked_add(other.points)
            .ok_or(IntegrationError::NumericRange)?;
        if !self.finite || !other.finite {
            self.invalidate();
            return Ok(());
        }
        for i in 0..self.means.len() {
            let delta = (other.means[i] - self.means[i])
                * (DoubleFloat::from(other.points as f64) / DoubleFloat::from(self.points as f64));
            self.means[i] += delta;
            self.maxima[i] = self.maxima[i].max(other.maxima[i]);
        }
        if self.means.iter().any(|v| !v.is_finite()) {
            self.invalidate();
        }
        Ok(())
    }
}

/// Same shift partition as QmcPartial, retaining no individual point values.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub(crate) struct WeightPartial {
    pub(crate) first_shift: u64,
    pub(crate) groups: Vec<WeightStatistics>,
}
impl WeightPartial {
    fn partition(start: u64, count: u64, points: u64) -> Result<(u64, usize, u64)> {
        if count == 0 || points == 0 {
            return Err(IntegrationError::InvalidReturn(
                "empty weight interval or lattice".into(),
            ));
        }
        let end = start
            .checked_add(count)
            .ok_or_else(|| IntegrationError::InvalidReturn("weight interval overflow".into()))?;
        let first = start / points;
        let groups = usize::try_from((end - 1) / points - first + 1).map_err(|_| {
            IntegrationError::InvalidReturn("weight partition size overflow".into())
        })?;
        Ok((first, groups, end))
    }
    pub(crate) fn new(start: u64, count: u64, points: u64, n: usize) -> Result<Self> {
        let (first_shift, groups, _) = Self::partition(start, count, points)?;
        groups
            .checked_mul(n)
            .ok_or_else(|| IntegrationError::InvalidReturn("weight buffer size overflow".into()))?;
        Ok(Self {
            first_shift,
            groups: vec![WeightStatistics::new(n); groups],
        })
    }
    pub(crate) fn validate(&self, start: u64, count: u64, points: u64, n: usize) -> Result<()> {
        let (first, groups, end) = Self::partition(start, count, points)?;
        if self.first_shift != first || self.groups.len() != groups {
            return Err(IntegrationError::InvalidReturn(
                "weight shift partition differs".into(),
            ));
        }
        for (i, row) in self.groups.iter().enumerate() {
            // first+i <= (end-1)/points, so this product is bounded by end-1.
            let base = (first + i as u64) * points;
            let a = base.max(start);
            let b = base.saturating_add(points).min(end);
            row.validate(n, b - a)?;
        }
        Ok(())
    }
}

pub(crate) fn report(
    problem: &IntegrationProblem,
    estimate: Option<&VectorEstimate>,
    mut status: WeightDiagnosticStatus,
    accepted_points: u64,
    groups: &[WeightStatistics],
) -> WeightDiagnostics {
    let layout = WeightLayout::new(problem);
    let mut exact = Vec::new();
    layout.magnitudes(&problem.exact_coefficients, 1.0, &mut exact);
    let mut magnitude = Vec::new();
    if let Some(value) = estimate {
        layout.magnitudes(&value.mean, 1.0, &mut magnitude);
    }
    if groups.iter().any(|g| !g.finite) {
        status = WeightDiagnosticStatus::NumericRange;
    }
    if exact.iter().any(|v| !v.is_finite()) {
        status = WeightDiagnosticStatus::NumericRange;
    }
    let available = status == WeightDiagnosticStatus::Available;
    let ratio = |a: Option<f64>, b: Option<f64>| {
        a.zip(b)
            .and_then(|(a, b)| (b != 0.0).then(|| a / b))
            .filter(|v| v.is_finite())
    };
    let mut orders: Vec<_> = layout
        .orders
        .iter()
        .enumerate()
        .map(|(i, &order)| {
            let stochastic = available
                .then(|| {
                    groups
                        .iter()
                        .fold(DoubleFloat::from(0.0), |sum, g| {
                            sum + if g.points == 0 {
                                DoubleFloat::from(0.0)
                            } else {
                                g.means[i]
                            }
                        })
                        .to_f64()
                })
                .filter(|v| v.is_finite());
            let maximum = (available && accepted_points != 0)
                .then(|| groups.iter().map(|g| g.maxima[i]).fold(0.0_f64, f64::max));
            let absolute = stochastic
                .map(|v| (DoubleFloat::from(v) + DoubleFloat::from(exact[i])).to_f64())
                .filter(|v| v.is_finite());
            let mean = magnitude.get(i).copied().filter(|v| v.is_finite());
            WeightOrderDiagnostics {
                order,
                stochastic_absolute_mean: stochastic,
                exact_absolute: exact[i].is_finite().then_some(exact[i]),
                absolute_integral_estimate: absolute,
                sampled_sector_local_maximum: maximum,
                total_mean_magnitude: mean,
                cancellation_ratio: ratio(absolute, mean),
                maximum_ratio: ratio(maximum, mean),
            }
        })
        .collect();
    if status == WeightDiagnosticStatus::Available
        && orders
            .iter()
            .any(|r| r.stochastic_absolute_mean.is_none() || r.absolute_integral_estimate.is_none())
    {
        status = WeightDiagnosticStatus::NumericRange;
        for r in &mut orders {
            r.stochastic_absolute_mean = None;
            r.absolute_integral_estimate = None;
            r.sampled_sector_local_maximum = None;
            r.cancellation_ratio = None;
            r.maximum_ratio = None;
        }
    }
    WeightDiagnostics {
        status,
        accepted_points,
        orders,
    }
}

#[cfg(test)]
mod tests;

impl WeightDiagnostics {
    /// Validate serialized diagnostics against the complete native layout and
    /// the exact selected estimator/work. No covariance or statistic is inferred.
    pub fn validate(
        &self,
        orders: &[i32],
        components: &[super::CoefficientComponent],
        exact: &[f64],
        estimate: Option<&VectorEstimate>,
        accepted_points: u64,
    ) -> Result<()> {
        let invalid = || {
            IntegrationError::Invalid(
                "weight diagnostics differ from native estimate/layout/work".into(),
            )
        };
        if orders.len() != components.len()
            || exact.len() != orders.len()
            || exact.iter().any(|v| !v.is_finite())
            || self.accepted_points != accepted_points
            || orders
                .iter()
                .zip(components)
                .collect::<std::collections::BTreeSet<_>>()
                .len()
                != orders.len()
        {
            return Err(invalid());
        }
        let layout = WeightLayout::from_orders(orders);
        layout.validate(orders.len())?;
        if self
            .orders
            .iter()
            .map(|r| r.order)
            .ne(layout.orders.iter().copied())
        {
            return Err(invalid());
        }
        let mut exact_norm = Vec::new();
        layout.magnitudes(exact, 1.0, &mut exact_norm);
        let mut mean_norm = Vec::new();
        if let Some(v) = estimate {
            v.validate()?;
            if v.orders != orders || v.components != components {
                return Err(invalid());
            }
            layout.magnitudes(&v.mean, 1.0, &mut mean_norm);
        }
        let finite = |v: f64| v.is_finite().then_some(v);
        let ratio = |a: Option<f64>, b: Option<f64>| {
            a.zip(b)
                .and_then(|(a, b)| (b != 0.0).then(|| a / b))
                .filter(|v| v.is_finite())
        };
        for (i, row) in self.orders.iter().enumerate() {
            let values = [
                row.stochastic_absolute_mean,
                row.exact_absolute,
                row.absolute_integral_estimate,
                row.sampled_sector_local_maximum,
                row.total_mean_magnitude,
                row.cancellation_ratio,
                row.maximum_ratio,
            ];
            if values.iter().flatten().any(|v| !v.is_finite() || *v < 0.0)
                || row.exact_absolute != finite(exact_norm[i])
                || row.total_mean_magnitude != mean_norm.get(i).and_then(|v| finite(*v))
                || row.absolute_integral_estimate
                    != row
                        .stochastic_absolute_mean
                        .zip(row.exact_absolute)
                        .and_then(|(a, b)| {
                            finite((DoubleFloat::from(a) + DoubleFloat::from(b)).to_f64())
                        })
                || row.cancellation_ratio
                    != ratio(row.absolute_integral_estimate, row.total_mean_magnitude)
                || row.maximum_ratio
                    != ratio(row.sampled_sector_local_maximum, row.total_mean_magnitude)
                || (self.status != WeightDiagnosticStatus::Available
                    && (row.stochastic_absolute_mean.is_some()
                        || row.sampled_sector_local_maximum.is_some()))
                || (self.status == WeightDiagnosticStatus::Available
                    && (row.stochastic_absolute_mean.is_none()
                        || (accepted_points != 0 && row.sampled_sector_local_maximum.is_none())
                        || (accepted_points == 0
                            && (row.sampled_sector_local_maximum.is_some()
                                || row.stochastic_absolute_mean != Some(0.0)))))
            {
                return Err(invalid());
            }
        }
        Ok(())
    }
}
