//! Absolute Taylor coefficients of compact original-source numerator calls.
//! The selected roots extend to this face; this does not certify density units.
pub(super) mod admission;
pub(super) mod definitions;
pub mod program;
mod progress;
mod series;
#[cfg(test)]
mod tests;
use super::{Error, Limits, Result, SourceNumerators, Work, native};
use crate::{
    kernel::algebraic::Scope,
    threshold::{
        continued::Definition,
        maps::{endpoint_transport::cover::IssuedEndpointChart, regular::Budget},
    },
};
pub use admission::{FiniteArgument, SelectedExtension};
pub use progress::{PreparationProgress, PreparationStage, PreparationState};
use progress::{
    PreparationState::{Completed, Started},
    emit,
};
use std::{ops::ControlFlow, sync::Arc};
use symbolica::{
    atom::{Atom, AtomCore, Symbol},
    evaluate::OptimizationSettings,
};

/// A bounded jet of one actual source record. Order j is the Taylor
/// coefficient, not the j-th derivative, and not an inferred leading valuation.
pub struct NormalCoefficient {
    record: usize,
    values: Vec<Atom>,
}
impl NormalCoefficient {
    pub fn source_record(&self) -> usize {
        self.record
    }
    pub fn coefficients(&self) -> &[Atom] {
        &self.values
    }
}
/// Geometry/source ownership is retained once, not copied per coefficient.
pub struct NormalNumeratorJets {
    catalogue: Arc<SourceNumerators>,
    chart: IssuedEndpointChart,
    normal: Symbol,
    depth: u32,
    directions: Vec<Symbol>,
    arguments: Vec<FiniteArgument>,
    sections: Vec<SelectedExtension>,
    coefficients: Vec<NormalCoefficient>,
    definitions: Vec<Definition>,
    head: Symbol,
    scope: Scope,
}
impl NormalNumeratorJets {
    pub fn catalogue(&self) -> &Arc<SourceNumerators> {
        &self.catalogue
    }
    pub fn chart(&self) -> &IssuedEndpointChart {
        &self.chart
    }
    pub fn normal(&self) -> Symbol {
        self.normal
    }
    pub fn depth(&self) -> u32 {
        self.depth
    }
    pub fn direction_units(&self) -> &[Symbol] {
        &self.directions
    }
    pub fn finite_arguments(&self) -> &[FiniteArgument] {
        &self.arguments
    }
    pub fn selected_extensions(&self) -> &[SelectedExtension] {
        &self.sections
    }
    pub fn coefficients(&self) -> &[NormalCoefficient] {
        &self.coefficients
    }
    pub fn source_definition_count(&self) -> usize {
        self.definitions.len()
    }
    pub fn closed_density_certified(&self) -> bool {
        false
    }
    pub fn prepare(
        catalogue: Arc<SourceNumerators>,
        chart: IssuedEndpointChart,
        normal: Symbol,
        depth: u32,
        settings: OptimizationSettings,
        limits: &Limits,
        progress: impl FnMut() -> ControlFlow<()>,
    ) -> Result<Arc<Self>> {
        Self::prepare_with_progress(
            catalogue,
            chart,
            normal,
            depth,
            settings,
            limits,
            progress,
            progress::ignore,
        )
    }

    /// Prepare the same source-owned symbolic jets with sparse caller-owned
    /// stage observations. The cancellation callback retains its ordinary
    /// polling frequency; the observer is never called for node visits.
    /// Neither callback makes an indivisible native call interruptible.
    #[expect(
        clippy::too_many_arguments,
        reason = "Keep existing cancellation separate from sparse observation"
    )]
    pub fn prepare_with_progress(
        catalogue: Arc<SourceNumerators>,
        chart: IssuedEndpointChart,
        normal: Symbol,
        depth: u32,
        settings: OptimizationSettings,
        limits: &Limits,
        mut progress: impl FnMut() -> ControlFlow<()>,
        mut observer: impl FnMut(PreparationProgress) -> ControlFlow<()>,
    ) -> Result<Arc<Self>> {
        let mut work = Work {
            limits,
            budget: Budget::new(limits.algebra.clone()),
            callback: &mut progress,
        };
        emit(
            PreparationStage::Preparation,
            Started,
            &mut work,
            &mut observer,
        )?;
        emit(
            PreparationStage::Validation,
            Started,
            &mut work,
            &mut observer,
        )?;
        if !Arc::ptr_eq(
            catalogue.preparation(),
            chart.parent_prefix().source().decomposition().preparation(),
        ) || !chart.transport().transport_complete()
            || !chart.target_units().contains(&normal)
        {
            return Err(Error::Invalid(
                "normal jet source/chart/normal association".into(),
            ));
        }
        if depth == 0 || depth > limits.max_relative_depth {
            return Err(Error::ResourceIncomplete("normal jet depth".into()));
        }
        work.entries(
            catalogue
                .records()
                .len()
                .checked_mul(depth as usize)
                .ok_or_else(|| {
                    Error::ResourceIncomplete("normal coefficient count overflow".into())
                })?,
        )?;
        emit(
            PreparationStage::Validation,
            Completed,
            &mut work,
            &mut observer,
        )?;
        emit(
            PreparationStage::Admission,
            Started,
            &mut work,
            &mut observer,
        )?;
        let admitted = admission::prepare(
            &catalogue,
            &chart,
            normal,
            settings,
            &mut work,
            &mut observer,
        )?;
        emit(
            PreparationStage::Admission,
            Completed,
            &mut work,
            &mut observer,
        )?;
        emit(
            PreparationStage::Template,
            Started,
            &mut work,
            &mut observer,
        )?;
        let head = definitions::fresh_head(&catalogue, &chart);
        let mut coefficients = Vec::new();
        if catalogue.records().iter().any(|r| r.degrees().is_some()) {
            let template = series::prepare(
                &catalogue,
                &chart,
                &admitted,
                head,
                normal,
                depth,
                &mut work,
                &mut observer,
            )?;
            for (record, source) in catalogue.records().iter().enumerate() {
                if source.degrees().is_some() {
                    emit(
                        PreparationStage::Record { record },
                        Started,
                        &mut work,
                        &mut observer,
                    )?;
                    coefficients.push(NormalCoefficient {
                        record,
                        values: template.record(record, &mut work)?,
                    });
                    emit(
                        PreparationStage::Record { record },
                        Completed,
                        &mut work,
                        &mut observer,
                    )?;
                }
            }
        }
        emit(
            PreparationStage::Template,
            Completed,
            &mut work,
            &mut observer,
        )?;
        emit(
            PreparationStage::Definitions,
            Started,
            &mut work,
            &mut observer,
        )?;
        let definitions = definitions::collect(
            &catalogue,
            head,
            &coefficients,
            depth,
            &mut work,
            &mut observer,
        )?;
        emit(
            PreparationStage::Definitions,
            Completed,
            &mut work,
            &mut observer,
        )?;
        emit(
            PreparationStage::Assembly,
            Started,
            &mut work,
            &mut observer,
        )?;
        let directions = chart
            .target_units()
            .iter()
            .copied()
            .filter(|s| *s != normal)
            .collect();
        work.check()?;
        let owner = Arc::new(Self {
            catalogue,
            chart,
            normal,
            depth,
            directions,
            arguments: admitted.arguments,
            sections: admitted.sections,
            coefficients,
            definitions,
            head,
            scope: admitted.scope,
        });
        emit(
            PreparationStage::Assembly,
            Completed,
            &mut work,
            &mut observer,
        )?;
        emit(
            PreparationStage::Preparation,
            Completed,
            &mut work,
            &mut observer,
        )?;
        Ok(owner)
    }
}
