//! Absolute Taylor coefficients of compact original-source numerator calls.
//! The selected roots extend to this face; this does not certify density units.
mod admission;
mod definitions;
pub mod program;
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
use std::{ops::ControlFlow, sync::Arc};
use symbolica::{
    atom::{Atom, AtomCore, Symbol},
    evaluate::OptimizationSettings,
    poly::series::SeriesDepth,
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
        mut progress: impl FnMut() -> ControlFlow<()>,
    ) -> Result<Arc<Self>> {
        let mut work = Work {
            limits,
            budget: Budget::new(limits.algebra.clone()),
            callback: &mut progress,
        };
        work.check()?;
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
        let admitted = admission::prepare(&catalogue, &chart, normal, settings, &mut work)?;
        let head = definitions::fresh_head(&catalogue, &chart);
        let mut coefficients = Vec::new();
        for (record, source) in catalogue.records().iter().enumerate() {
            if source.degrees().is_none() {
                continue;
            }
            work.check()?;
            let mut args = vec![Atom::num(record)];
            args.extend(admitted.images.iter().cloned());
            let tagged = symbolica::atom::FunctionBuilder::new(head)
                .add_args(&args)
                .finish();
            work.nodes(&tagged, limits.max_coefficient_nodes)?;
            // Native series differentiates both actual root coefficient maps
            // and tagged source calls before restriction. No N(phi) expansion.
            let series = tagged
                .series(normal, 0, SeriesDepth::absolute(depth as i64 - 1))
                .map_err(native)?;
            let values = (0..depth)
                .map(|j| series.coefficient(j.into()).unwrap_or_else(Atom::zero))
                .collect::<Vec<_>>();
            for value in &values {
                work.nodes(value, limits.max_coefficient_nodes)?;
            }
            coefficients.push(NormalCoefficient { record, values });
        }
        let definitions = definitions::collect(&catalogue, head, &coefficients, depth, &mut work)?;
        let directions = chart
            .target_units()
            .iter()
            .copied()
            .filter(|s| *s != normal)
            .collect();
        work.check()?;
        Ok(Arc::new(Self {
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
        }))
    }
}
