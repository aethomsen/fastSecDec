//! Verified open-cell descriptions, linear lowering, and checked regular sections.
//!
//! The exact native selectors remain attached to the proof owner. Executable
//! nonlinear sections, endpoint regularity and full-integral continuation are
//! separate certificates; this module does not infer them from sample fibers.

mod linear;
mod parameters;
mod program;
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
        let request = owner.request();
        let raw = owner.native_result();
        let cell = raw.cells.get(cell_index).ok_or_else(|| {
            GcadError::Invalid("cell-map index is outside the verified raw cells".into())
        })?;
        if unit_coordinates.len() != request.domain().coordinates().len()
            || unit_coordinates
                .iter()
                .copied()
                .collect::<BTreeSet<_>>()
                .len()
                != unit_coordinates.len()
        {
            return Err(GcadError::Invalid(
                "cell map needs distinct unit symbols for exactly its integration dimensions"
                    .into(),
            ));
        }
        let occupied = coordinate_symbols(request);
        if unit_coordinates.iter().any(|s| occupied.contains(s)) {
            return Err(GcadError::Invalid(
                "unit coordinates collide with an original density/domain symbol".into(),
            ));
        }
        if cell.axes.len() != raw.order.len() {
            return Err(GcadError::Invalid(
                "verified cell axis/order shape differs".into(),
            ));
        }
        let mut axes = Vec::with_capacity(raw.order.len());
        let mut integration_count = 0;
        for (name, axis) in raw.order.iter().zip(&cell.axes) {
            let alias = request
                .aliases()
                .iter()
                .find(|a| &a.name == name)
                .ok_or_else(|| {
                    GcadError::Invalid("cell lifting symbol has no source association".into())
                })?;
            if &axis.variable != name
                || (integration_count > 0 && alias.role == AliasRole::RuntimeParameter)
            {
                return Err(GcadError::Invalid(
                    "cell map changes native axis order or mixes parameter fibers".into(),
                ));
            }
            let unit_index = if alias.role == AliasRole::IntegrationCoordinate {
                let index = integration_count;
                integration_count += 1;
                Some(index)
            } else {
                None
            };
            axes.push(MapAxis {
                symbol: alias.symbol,
                role: alias.role,
                unit_index,
            });
        }
        if integration_count != unit_coordinates.len() {
            return Err(GcadError::Invalid(
                "cell map integration roles differ from prepared domain".into(),
            ));
        }
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
mod tests;
