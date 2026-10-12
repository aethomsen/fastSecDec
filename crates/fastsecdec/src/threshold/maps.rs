//! Verified open-cell descriptions, linear lowering, and checked regular sections.
//!
//! The exact native selectors remain attached to the proof owner. Executable
//! nonlinear sections, endpoint regularity and full-integral continuation are
//! separate certificates; this module does not infer them from sample fibers.

pub mod endpoint_transport;
mod linear;
pub mod normalized;
mod open;
mod open_interval;
mod parameters;
mod program;
mod projective;
mod source;
pub use open::{IntervalSide, OpenFace, OpenFaceOrigin, OpenFiniteBound, OpenProjectiveAxis};
pub use projective::PositiveProjectiveCellMap;
pub mod regular;

use std::{collections::BTreeSet, sync::Arc};
use symbolica::atom::{AtomCore, Symbol};
use symgcad::output::Axis;

use super::gcad::{AliasRole, GcadError, Result, VerifiedDecomposition};
pub use linear::{LinearAxis, LinearCellMap, ParameterAdmission, RationalMappedPoint};
pub use parameters::{
    AdmittedParameterFiber, ParameterChamber, ParameterInputLayout, ParameterPrefix,
};
pub use program::CompiledCellMap;

/// The symbol roles and unit-coordinate association in native lifting order.
#[derive(Clone, Copy, Debug)]
pub struct MapAxis {
    symbol: Symbol,
    role: AliasRole,
    unit_index: Option<usize>,
}
impl MapAxis {
    pub fn symbol(&self) -> Symbol {
        self.symbol
    }
    pub fn role(&self) -> AliasRole {
        self.role
    }
    pub fn unit_index(&self) -> Option<usize> {
        self.unit_index
    }
}

/// Exact cell description retaining one immutable decomposition owner.
/// Unit coordinates follow integration lifting order; outputs retain the
/// original prepared/source order explicitly rather than reusing cell IDs.
#[derive(Debug)]
pub struct CellMap {
    owner: Arc<VerifiedDecomposition>,
    cell_index: usize,
    axes: Vec<MapAxis>,
    unit_coordinates: Vec<Symbol>,
}
impl CellMap {
    pub fn new(
        owner: Arc<VerifiedDecomposition>,
        cell_index: usize,
        unit_coordinates: Vec<Symbol>,
    ) -> Result<Arc<Self>> {
        let view = source::GeometryView::standard(&owner);
        let axes = view.admit(cell_index, &unit_coordinates)?;
        Ok(Arc::new(Self {
            owner,
            cell_index,
            axes,
            unit_coordinates,
        }))
    }
    pub fn decomposition(&self) -> &Arc<VerifiedDecomposition> {
        &self.owner
    }
    pub fn cell_index(&self) -> usize {
        self.cell_index
    }
    pub fn axes(&self) -> &[MapAxis] {
        &self.axes
    }
    pub fn unit_coordinates(&self) -> &[Symbol] {
        &self.unit_coordinates
    }
    /// The raw RootBounds retain native selectors and sample enclosures. Only
    /// their polynomial/index/domain define a section, never the enclosure.
    pub fn native_axes(&self) -> &[Axis] {
        &self.owner.native_result().cells[self.cell_index].axes
    }
    /// Lower only the actual parameter prefix, preserving nonlinear integration bounds.
    pub fn parameter_prefix(self: &Arc<Self>) -> Result<Arc<ParameterPrefix>> {
        ParameterPrefix::prepare(self.clone())
    }
    pub fn linear(self: &Arc<Self>) -> Result<Arc<LinearCellMap>> {
        LinearCellMap::new(self.clone())
    }
    /// Reuse native finite linear-root admission for a selected triangular
    /// suffix while earlier axes retain separately issued nonlinear sections.
    pub(crate) fn linear_axes(&self, range: std::ops::Range<usize>) -> Result<Vec<LinearAxis>> {
        linear::root::lower_axes(self, range)
    }
}

/// Exact occupied native roles shared by cell admission and deferred preparation.
pub(crate) fn coordinate_symbols(request: &super::gcad::GcadRequest) -> BTreeSet<Symbol> {
    let mut occupied = request
        .input()
        .parameters()
        .iter()
        .copied()
        .collect::<BTreeSet<_>>();
    occupied.insert(request.input().regulator());
    occupied.extend(request.kinematics().runtime_parameters.iter().copied());
    occupied.extend(request.kinematics().exact_values.keys().copied());
    for term in request
        .input()
        .terms()
        .iter()
        .chain(request.prepared_terms())
    {
        occupied.extend(term.prefactor().get_all_symbols(true));
        for power in term.monomial_powers() {
            occupied.extend(power.get_all_symbols(true));
        }
        for factor in term.factors() {
            occupied.extend(factor.polynomial().get_all_symbols(true));
            occupied.extend(factor.exponent().get_all_symbols(true));
        }
    }
    for constraint in request
        .domain()
        .strict_positive()
        .iter()
        .chain(&request.kinematics().strict_positive)
    {
        occupied.extend(constraint.get_all_symbols(true));
    }
    occupied
}

#[cfg(test)]
mod open_tests;
#[cfg(test)]
mod tests;
