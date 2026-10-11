//! A chamber becomes a template capability only after complete uniform density
//! admission. The underlying prefix owner alone is not this certificate.
use super::*;
use crate::{
    kernel::parameter_guard::ExactParameterGuard,
    threshold::maps::{ParameterChamber, ParameterInputLayout},
};

pub struct UniformParameterChamber {
    geometry: Arc<ParameterChamber>,
    integration_axes: Vec<usize>,
}
impl UniformParameterChamber {
    // Called by density::assemble only after all charts, unit margins, phases,
    // endpoint powers and the common meromorphic witness have been admitted.
    pub(super) fn issued(geometry: Arc<ParameterChamber>) -> Self {
        let integration_axes = geometry
            .layout()
            .raw_to_unit()
            .iter()
            .enumerate()
            .filter_map(|(axis, slot)| slot.map(|_| axis))
            .collect();
        Self {
            geometry,
            integration_axes,
        }
    }
    pub fn runtime_parameters(&self) -> &[Symbol] {
        self.geometry.layout().parameters()
    }
    pub fn raw_prefix_count(&self) -> usize {
        self.runtime_parameters().len()
    }
    pub fn integration_axes(&self) -> &[usize] {
        &self.integration_axes
    }
    pub fn layout(&self) -> &ParameterInputLayout {
        self.geometry.layout()
    }
    pub fn decomposition(&self) -> &Arc<VerifiedDecomposition> {
        self.geometry.decomposition()
    }
    pub fn cell_indices(&self) -> impl ExactSizeIterator<Item = usize> + '_ {
        self.geometry
            .prefixes()
            .iter()
            .map(|p| p.source().cell_index())
    }
    pub(crate) fn guard(&self) -> &Arc<ExactParameterGuard> {
        self.geometry.guard()
    }
}
