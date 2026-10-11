//! Exact finite replacement of one endpoint chart by half cubes and native fans.
mod build;
#[cfg(test)]
mod tests;
mod view;
use super::{EndpointChart, EndpointTransport, FaceSide, Limits, Result};
use fastsecdec_sectors::{Decomposition, PolynomialSupport};
use std::{ops::ControlFlow, sync::Arc};
use symbolica::atom::{Atom, Symbol};
pub use view::IssuedEndpointChart;

/// Domain-boundary ancestry, not a claim about a physical zero or real unit.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ChildBoundary {
    Parent { unit: Symbol, side: FaceSide },
    Midpoint { unit: Symbol },
}
#[derive(Clone, Debug)]
pub struct SplitInterval {
    unit: Symbol,
    axis: usize,
    half: FaceSide,
}
impl SplitInterval {
    pub fn unit(&self) -> Symbol {
        self.unit
    }
    pub fn axis(&self) -> usize {
        self.axis
    }
    pub fn half(&self) -> FaceSide {
        self.half
    }
    /// The upper half is oriented from the original upper face toward 1/2.
    pub fn orientation(&self) -> i8 {
        if self.half == FaceSide::Lower { 1 } else { -1 }
    }
    pub fn zero_boundary(&self) -> ChildBoundary {
        ChildBoundary::Parent {
            unit: self.unit,
            side: self.half,
        }
    }
    pub fn one_boundary(&self) -> ChildBoundary {
        ChildBoundary::Midpoint { unit: self.unit }
    }
}
/// One member of the complete Cartesian partition. Images use the cover's
/// target role names before a possible native monomial substitution.
#[derive(Debug)]
pub struct HalfCube {
    mask: usize,
    intervals: Vec<SplitInterval>,
    images: Vec<Atom>,
    orientation: i8,
    supports: Vec<PolynomialSupport>,
    support_indices: Vec<usize>,
    geometry: Option<Decomposition>,
    first_leaf: usize,
    leaf_count: usize,
}
impl HalfCube {
    pub fn mask(&self) -> usize {
        self.mask
    }
    pub fn intervals(&self) -> &[SplitInterval] {
        &self.intervals
    }
    pub fn images(&self) -> &[Atom] {
        &self.images
    }
    pub fn orientation(&self) -> i8 {
        self.orientation
    }
    pub fn supports(&self) -> &[PolynomialSupport] {
        &self.supports
    }
    pub fn support_indices(&self) -> &[usize] {
        &self.support_indices
    }
    pub fn geometry(&self) -> Option<&Decomposition> {
        self.geometry.as_ref()
    }
    pub fn leaf_range(&self) -> std::ops::Range<usize> {
        self.first_leaf..self.first_leaf + self.leaf_count
    }
}
#[derive(Debug)]
pub struct CoverLeaf {
    half: usize,
    native_sector: Option<usize>,
    parent_chart_images: Vec<Atom>,
    added_jacobian: Atom,
    chart: EndpointChart,
}
impl CoverLeaf {
    pub fn half_index(&self) -> usize {
        self.half
    }
    pub fn native_sector(&self) -> Option<usize> {
        self.native_sector
    }
    /// Parent endpoint-chart inputs in their retained target order.
    pub fn parent_chart_images(&self) -> &[Atom] {
        &self.parent_chart_images
    }
    /// Half width times native sector measure, excluding the parent measure.
    pub fn added_jacobian(&self) -> &Atom {
        &self.added_jacobian
    }
    pub fn chart(&self) -> &EndpointChart {
        &self.chart
    }
}
/// Complete replacement cover of exactly one issued chart. Other siblings of
/// `parent_chart()` remain unchanged; adding this cover alongside the replaced
/// chart would count that region twice. `parent()` retains the original transport.
/// No closed-face regularity is issued.
#[derive(Debug)]
pub struct EndpointChartCover {
    parent: Arc<EndpointTransport>,
    parent_chart: IssuedEndpointChart,
    split_axes: Vec<usize>,
    target_units: Vec<Symbol>,
    halves: Vec<HalfCube>,
    leaves: Vec<CoverLeaf>,
}
impl EndpointChartCover {
    pub fn parent(&self) -> &Arc<EndpointTransport> {
        &self.parent
    }
    /// Ordinal in the immediate parent's inventory, which can be a prior cover.
    pub fn parent_chart_index(&self) -> usize {
        self.parent_chart.ordinal()
    }
    /// Immediate chart replaced by this complete cover. Recursion retains every
    /// earlier cover, its artificial midpoint faces and original source owner.
    pub fn parent_chart(&self) -> &IssuedEndpointChart {
        &self.parent_chart
    }
    pub fn split_axes(&self) -> &[usize] {
        &self.split_axes
    }
    pub fn target_units(&self) -> &[Symbol] {
        &self.target_units
    }
    pub fn halves(&self) -> &[HalfCube] {
        &self.halves
    }
    pub fn leaves(&self) -> &[CoverLeaf] {
        &self.leaves
    }
    pub fn replacement_complete(&self) -> bool {
        let Some(expected) = u32::try_from(self.split_axes.len())
            .ok()
            .and_then(|n| 1usize.checked_shl(n))
        else {
            return false;
        };
        let mut frontier = 0usize;
        self.halves.len() == expected
            && self.halves.iter().enumerate().all(|(i, h)| {
                let valid = h.mask == i && h.leaf_count != 0 && h.first_leaf == frontier;
                frontier = match frontier.checked_add(h.leaf_count) {
                    Some(n) => n,
                    None => return false,
                };
                valid
            })
            && frontier == self.leaves.len()
    }
    pub fn closed_faces_certified(&self) -> bool {
        false
    }
    pub fn prepare(
        parent: Arc<EndpointTransport>,
        parent_chart: usize,
        split_axes: &[Symbol],
        refined_masks: &[usize],
        limits: &Limits,
        progress: impl FnMut() -> ControlFlow<()>,
    ) -> Result<Arc<Self>> {
        Self::refine(
            parent.issued_chart(parent_chart)?,
            split_axes,
            refined_masks,
            limits,
            progress,
        )
    }
    /// Replace one issued original chart or prior cover leaf. This never adds
    /// its children alongside the replaced parent, nor claims coverage of other
    /// siblings. The exact original factors and source powers are re-pulled.
    pub fn refine(
        parent_chart: IssuedEndpointChart,
        split_axes: &[Symbol],
        refined_masks: &[usize],
        limits: &Limits,
        mut progress: impl FnMut() -> ControlFlow<()>,
    ) -> Result<Arc<Self>> {
        build::prepare(
            parent_chart,
            split_axes,
            refined_masks,
            limits,
            &mut progress,
        )
    }
}
