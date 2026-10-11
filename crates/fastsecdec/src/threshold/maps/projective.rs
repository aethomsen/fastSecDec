use super::{GcadError, LinearAxis, MapAxis, Result, linear, source::GeometryView};
use crate::threshold::gcad::PositiveProjectiveQuotient;
use std::sync::Arc;
use symbolica::atom::Symbol;
use symgcad::output::Axis;

/// Exact quotient-cell description. Its ordinal is also the original cell
/// ordinal by the freshly replayed native homogeneous reconstruction. It is not
/// an affine-simplex CausalCell and carries no endpoint/infinity authorization.
#[derive(Debug)]
pub struct PositiveProjectiveCellMap {
    owner: Arc<PositiveProjectiveQuotient>,
    cell_index: usize,
    axes: Vec<MapAxis>,
    unit_coordinates: Vec<Symbol>,
}
impl PositiveProjectiveCellMap {
    pub fn new(
        owner: Arc<PositiveProjectiveQuotient>,
        cell_index: usize,
        unit_coordinates: Vec<Symbol>,
    ) -> Result<Arc<Self>> {
        let axes = GeometryView::projective(&owner).admit(cell_index, &unit_coordinates)?;
        Ok(Arc::new(Self {
            owner,
            cell_index,
            axes,
            unit_coordinates,
        }))
    }
    pub fn decomposition(&self) -> &Arc<PositiveProjectiveQuotient> {
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
    pub fn native_axes(&self) -> &[Axis] {
        &self.owner.native_result().cells[self.cell_index].axes
    }
    /// Attempt exactly the existing native finite-linear lowering. Unsupported
    /// bounds remain explicit; even success does not issue endpoint certificates.
    pub fn linear_axes(&self) -> Result<Vec<LinearAxis>> {
        if self.axes.is_empty() {
            return Err(GcadError::Unsupported(
                "zero-dimensional projective map".into(),
            ));
        }
        linear::root::lower_geometry(
            &GeometryView::projective(&self.owner),
            self.cell_index,
            &self.axes,
            0..self.axes.len(),
        )
    }
}
