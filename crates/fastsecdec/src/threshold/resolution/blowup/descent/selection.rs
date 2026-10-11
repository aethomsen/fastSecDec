//! One whole-ambient native monomial maximum, then its exact restrictions.
//! This is the monomial terminal-invariant case only, not general BM selection.
use super::*;
mod restriction;
pub use restriction::{MonomialCenterRestriction, SelectedCenterState};

/// Selection is issued on the atlas's exact current source and history.
/// Equal polynomials or the same ledger alone do not import a selection event.
#[derive(Debug)]
pub struct MonomialMaximumSelection {
    atlas: Arc<PrincipalCenterAtlas>,
    proof: MonomialProduction,
}
impl MonomialMaximumSelection {
    pub fn produce(
        atlas: Arc<PrincipalCenterAtlas>,
        powers: Vec<usize>,
        quotients: Vec<Poly>,
        b: &mut Budget,
    ) -> Result<Arc<Self>> {
        let proof = produce_monomial_center(
            atlas.history().ledger().clone(),
            MonomialWitness {
                source: atlas.source().clone(),
                powers,
                quotients,
            },
            b,
        )?;
        if let MonomialProduction::Incomplete { reason, .. } = &proof {
            return Err(Error::ResourceIncomplete(reason));
        }
        Ok(Arc::new(Self { atlas, proof }))
    }
    pub fn atlas(&self) -> &Arc<PrincipalCenterAtlas> {
        &self.atlas
    }
    pub fn native_proof(&self) -> &MonomialProduction {
        &self.proof
    }
    pub fn restrict(self: &Arc<Self>, index: usize, b: &mut Budget) -> Result<Arc<LocalCenter>> {
        let restriction = restriction::prepare(self.clone(), index, b)?;
        Ok(LocalCenter::from_selection(Arc::new(restriction)))
    }
    /// Every local owner must restrict this SAME globally issued selection;
    /// independently recomputed local maxima are not accepted by this method.
    pub fn verify_cover(
        self: &Arc<Self>,
        centers: Vec<Arc<LocalCenter>>,
        overlaps: Vec<Arc<CenterOverlap>>,
        b: &mut Budget,
    ) -> Result<Arc<SelectedMonomialCover>> {
        b.count(centers.len())?;
        for center in &centers {
            let restriction = center.selection_restriction().ok_or(Error::Invalid(
                "selected monomial cover requires selection restrictions",
            ))?;
            if !Arc::ptr_eq(restriction.selection(), self)
                || !Arc::ptr_eq(center.atlas(), &self.atlas)
                || restriction.index() != center.index()
            {
                return Err(Error::Invalid("selected monomial source/history event"));
            }
        }
        let descent = GluedAdmissibleCenter::verify(self.atlas.clone(), centers, overlaps, b)?;
        Ok(Arc::new(SelectedMonomialCover {
            selection: self.clone(),
            descent,
        }))
    }
}
/// Complete compatible restrictions of the one native monomial maximum.
/// No physical maps, general invariant, or real integration atlas are issued.
#[derive(Debug)]
pub struct SelectedMonomialCover {
    selection: Arc<MonomialMaximumSelection>,
    descent: Arc<GluedAdmissibleCenter>,
}
impl SelectedMonomialCover {
    pub fn selection(&self) -> &Arc<MonomialMaximumSelection> {
        &self.selection
    }
    pub fn descent(&self) -> &Arc<GluedAdmissibleCenter> {
        &self.descent
    }
}

#[cfg(test)]
mod tests;
