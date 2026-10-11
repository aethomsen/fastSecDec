//! Sealed borrowed geometry shared by existing and positive-projective owners.
use super::{AliasRole, GcadError, MapAxis, Result, coordinate_symbols};
use crate::threshold::gcad::{PositiveProjectiveQuotient, SymbolAlias, VerifiedDecomposition};
use std::{borrow::Cow, collections::BTreeSet};
use symbolica::atom::Symbol;
use symgcad::output::SolveResult;

pub(super) struct GeometryView<'a> {
    pub raw: &'a SolveResult,
    aliases: &'a [SymbolAlias],
    coordinates: &'a [Symbol],
    occupied: Cow<'a, BTreeSet<Symbol>>,
}
impl<'a> GeometryView<'a> {
    pub fn standard(owner: &'a VerifiedDecomposition) -> Self {
        Self {
            raw: owner.native_result(),
            aliases: owner.request().aliases(),
            coordinates: owner.request().domain().coordinates(),
            occupied: Cow::Owned(coordinate_symbols(owner.request())),
        }
    }
    pub fn projective(owner: &'a PositiveProjectiveQuotient) -> Self {
        let p = owner.preparation();
        Self {
            raw: owner.native_result(),
            aliases: owner.aliases(),
            coordinates: p.coordinates(),
            occupied: Cow::Borrowed(owner.occupied_symbols()),
        }
    }
    pub fn admit(&self, cell_index: usize, unit_coordinates: &[Symbol]) -> Result<Vec<MapAxis>> {
        let raw = self.raw;
        let cell = raw.cells.get(cell_index).ok_or_else(|| {
            GcadError::Invalid("cell-map index is outside the verified raw cells".into())
        })?;
        if unit_coordinates.len() != self.coordinates.len()
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
        if unit_coordinates.iter().any(|s| self.occupied.contains(s)) {
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
            let alias = self
                .aliases
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
        Ok(axes)
    }
}
