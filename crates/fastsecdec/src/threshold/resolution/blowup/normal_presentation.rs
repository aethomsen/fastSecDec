//! Optional native presentation simplification of an already issued center.
//! Equality of generators does not certify a regular parameter system; the
//! unchanged adaptation producer still checks all minors and the whole cover.
use super::super::geometry::generators::VerifiedGeneratorPresentation;
use super::super::*;
use super::general::CheckedRecursiveCenter;
use std::sync::Arc;

#[derive(Clone, Debug)]
pub struct CenterNormalPresentation {
    source: Arc<CheckedRecursiveCenter>,
    presentation: Arc<VerifiedGeneratorPresentation>,
}
impl CenterNormalPresentation {
    pub fn source(&self) -> &Arc<CheckedRecursiveCenter> {
        &self.source
    }
    pub fn normalization(&self) -> &Arc<VerifiedQuotientNormalization> {
        self.presentation.normalization()
    }
    pub fn selected(&self) -> &Arc<MarkedIdeal> {
        self.presentation.selected()
    }
    pub fn candidate_checks(&self) -> usize {
        self.presentation.candidate_checks()
    }
}

pub(super) fn prepare(
    source: Arc<CheckedRecursiveCenter>,
    b: &mut Budget,
) -> Result<Arc<CenterNormalPresentation>, Error> {
    let local = source.history().ledger().frame().local();
    let center = Arc::new(MarkedIdeal::new(source.ideal().clone(), 1, b)?);
    let presentation = VerifiedGeneratorPresentation::prepare(local.clone(), center, b)?;
    let selected = presentation.selected().ideal().generators();
    if selected.is_empty() || selected.len() > source.history().ledger().frame().free_axes().len() {
        return Err(Error::ResourceIncomplete(
            "center presentation needs other generators",
        ));
    }
    Ok(Arc::new(CenterNormalPresentation {
        source,
        presentation,
    }))
}
