//! Borrowed source authority for issued continuation families. This projection
//! cannot manufacture a request, change gauge, or issue endpoint coverage.
use crate::threshold::gcad::{
    GcadError, GcadRequest, PositiveProjectiveQuotient, Result, SourceProvenance,
};
use std::sync::Arc;

#[derive(Clone, Copy)]
pub(crate) enum SourceAuthority<'a> {
    Request(&'a GcadRequest),
    PositiveProjective(&'a PositiveProjectiveQuotient),
}

impl SourceAuthority<'_> {
    pub fn source_identity(self) -> Result<String> {
        match self {
            // Preserve the exact identity implementation of every older recipe.
            Self::Request(request) => request.source_identity(),
            Self::PositiveProjective(owner) => match owner.provenance() {
                SourceProvenance::PreparametricGraph(graph) => {
                    if !Arc::ptr_eq(graph.exact(), owner.preparation().input()) {
                        return Err(GcadError::RequestMismatch);
                    }
                    Ok(graph.source_identity().to_owned())
                }
                SourceProvenance::Exact => {
                    crate::generation::source_identity(owner.preparation().input(), &[], &[])
                        .map_err(|e| GcadError::Invalid(e.to_string()))
                }
                SourceProvenance::ParametricRepresented(_)
                | SourceProvenance::PreparametricFamily(_) => Err(GcadError::Unsupported(
                    "positive-projective continuation source provenance".into(),
                )),
            },
        }
    }
}
