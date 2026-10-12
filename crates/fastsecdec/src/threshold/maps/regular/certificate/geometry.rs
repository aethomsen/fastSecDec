//! Sealed borrowed views of actual issued geometry; no detached public input.
use crate::threshold::maps::{CellMap, MapAxis, PositiveProjectiveCellMap};
use symgcad::output::{Axis, SolveResult};
pub(in crate::threshold::maps) struct Geometry<'a> {
    pub(super) raw: &'a SolveResult,
    pub(super) axes: &'a [Axis],
    pub(super) roles: &'a [MapAxis],
}
impl<'a> Geometry<'a> {
    pub(in crate::threshold::maps) fn affine(source: &'a CellMap) -> Self {
        Self {
            raw: source.decomposition().native_result(),
            axes: source.native_axes(),
            roles: source.axes(),
        }
    }
    pub(in crate::threshold::maps) fn positive(source: &'a PositiveProjectiveCellMap) -> Self {
        Self {
            raw: source.decomposition().native_result(),
            axes: source.native_axes(),
            roles: source.axes(),
        }
    }
}
