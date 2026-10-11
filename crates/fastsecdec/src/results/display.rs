use super::*;
use std::fmt;

/// Borrowed, validated result view with a separate row ordering.
pub struct SavedResultView<'a> {
    result: &'a SavedIntegrationResult,
    sector_ids: Vec<u64>,
}

impl SavedIntegrationResult {
    pub fn display_with_sort(&self, sort: ResultSectorSort) -> Result<SavedResultView<'_>> {
        Ok(SavedResultView {
            result: self,
            sector_ids: self.sector_order(sort)?,
        })
    }
    /// Return stable sector IDs without mutating payload order or statistics.
    pub fn sector_order(&self, sort: ResultSectorSort) -> Result<Vec<u64>> {
        self.validate()?;
        let index = match sort {
            ResultSectorSort::Id => None,
            ResultSectorSort::Magnitude(key) | ResultSectorSort::StandardError(key) => Some(
                self.manifest
                    .orders
                    .iter()
                    .zip(&self.manifest.components)
                    .position(|(&order, &component)| {
                        key.order == order && key.component == component
                    })
                    .ok_or(ResultError::UnknownCoefficient(key))?,
            ),
        };
        let mut rows: Vec<_> = self.contributions.sectors.iter().collect();
        rows.sort_by(|a, b| {
            let value = |row: &crate::integration::SectorContribution| {
                index.and_then(|i| {
                    row.estimate.as_ref().map(|e| match sort {
                        ResultSectorSort::Magnitude(_) => e.mean[i].abs(),
                        _ => e.standard_error[i],
                    })
                })
            };
            match (value(a), value(b)) {
                (Some(a), Some(b)) => b.total_cmp(&a),
                (Some(_), None) => std::cmp::Ordering::Less,
                (None, Some(_)) => std::cmp::Ordering::Greater,
                _ => std::cmp::Ordering::Equal,
            }
            .then_with(|| a.progress.id.cmp(&b.progress.id))
        });
        Ok(rows.into_iter().map(|row| row.progress.id).collect())
    }
}

impl fmt::Display for SavedIntegrationResult {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self.display_with_sort(ResultSectorSort::Id) {
            Ok(view) => view.fmt(f),
            Err(error) => write!(f, "Invalid saved integration result: {error}"),
        }
    }
}

impl fmt::Display for SavedResultView<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let result = self.result;
        result.fmt_ordered(f, &self.sector_ids)
    }
}

impl SavedIntegrationResult {
    fn fmt_ordered(&self, f: &mut fmt::Formatter<'_>, ids: &[u64]) -> fmt::Result {
        writeln!(
            f,
            "Saved integration result: {}",
            self.manifest.kernel_content_id
        )?;
        if let Some(selection) = &self.manifest.source_selection {
            writeln!(
                f,
                "Generation extent: original source charts {:?} of {}; partial original integral",
                selection.source_sectors(),
                selection.original_source_count()
            )?;
        }
        match &self.scope {
            ResultScope::FullIntegral => {
                writeln!(f, "Scope: full integral, including all exact contributions")?
            }
            ResultScope::SelectedSectors {
                sector_ids,
                exact_policy,
            } => writeln!(
                f,
                "Scope: {} selected stochastic sectors; exact contribution policy {exact_policy:?}",
                sector_ids.len()
            )?,
        }
        writeln!(
            f,
            "Stop: {:?}; uncertainty {:?}; independent evidence {:?}",
            self.stopping_reason, self.contributions.uncertainty, self.validation
        )?;
        if let Some(design) = &self.qmc_design {
            writeln!(f, "{design}")?;
        }
        if let Some(estimate) = &self.contributions.total {
            writeln!(f, "Authoritative total:")?;
            for (i, (&order, component)) in
                estimate.orders.iter().zip(&estimate.components).enumerate()
            {
                writeln!(
                    f,
                    "  {component:?} eps^{order}: {:.10e} +/- {:.3e}",
                    estimate.mean[i], estimate.standard_error[i]
                )?;
            }
        } else {
            writeln!(f, "Authoritative total unavailable")?;
        }
        if let Some(weights) = &self.weight_diagnostics {
            writeln!(
                f,
                "Accepted weight diagnostics: {:?}; {} production points",
                weights.status, weights.accepted_points
            )?;
            let number = |value: Option<f64>| {
                value.map_or_else(|| "unavailable".into(), |v| format!("{v:.4e}"))
            };
            for row in &weights.orders {
                let norm = self
                    .contributions
                    .total
                    .as_ref()
                    .map(|total| total.complex_norm_uncertainty(row.order));
                writeln!(
                    f,
                    "  eps^{}: C {}; max/|I| {}; norm relative SE {}{}",
                    row.order,
                    number(row.cancellation_ratio),
                    number(row.maximum_ratio),
                    number(norm.as_ref().and_then(|n| n.radial_relative_error)),
                    if norm.as_ref().is_some_and(|n| n.linearization_warning) {
                        " (first-order propagation warning)"
                    } else {
                        ""
                    }
                )?;
            }
        }
        writeln!(
            f,
            "Exact offset: {:?}",
            self.contributions.exact_coefficients
        )?;
        if let Some(diagnostics) = &self.evaluation_diagnostics {
            writeln!(f, "{diagnostics}")?;
        }
        let rows: std::collections::BTreeMap<_, _> = self
            .contributions
            .sectors
            .iter()
            .map(|row| (row.progress.id, row))
            .collect();
        self.contributions
            .fmt_rows(f, ids.iter().map(|id| rows[id]))?;
        match self.comparison() {
            Ok(ResultComparison::Compared(comparison)) => write!(f, "{comparison}"),
            Ok(ResultComparison::Unavailable(reason)) => {
                write!(f, "Reference comparison unavailable: {reason}")
            }
            Err(error) => write!(f, "Reference comparison unavailable: {error}"),
        }
    }
}

impl fmt::Display for ResultComparisonUnavailable {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if let Self::NumericRange { key, quantity } = self {
            return write!(
                f,
                "derived {quantity} for {:?} eps^{} exceeds numeric range",
                key.component, key.order
            );
        }
        f.write_str(match self {
            Self::NoEstimate => "no numerical estimate",
            Self::NoReference => "no stored reference",
            Self::SelectedScope => "selected-sector scope",
            Self::Pilot => "pilot observations only",
            Self::Cancelled => "computation cancelled",
            Self::NumericalFailure => "computation stopped with a numerical failure",
            Self::NumericRange { .. } => unreachable!(),
        })
    }
}
