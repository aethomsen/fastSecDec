//! Admissible center descent on a checked principal-open cover of ONE ambient.
//! Includes a separate whole-ambient monomial selection/restriction owner.
//! No general BM invariant, physical driver, parameter-stratification or real atlas.
use super::super::prepared_membership::PreparedMembership;
use super::super::*;
use std::sync::Arc;
mod atlas;
mod center;
mod overlap;
mod selection;
pub use atlas::PrincipalCenterAtlas;
pub use center::LocalCenter;
pub use overlap::CenterOverlap;
pub use selection::{
    MonomialCenterRestriction, MonomialMaximumSelection, SelectedCenterState, SelectedMonomialCover,
};
type Result<T> = std::result::Result<T, Error>;

fn equal_ideals(local: &LocalizedAlgebra, a: &Ideal, z: &Ideal, b: &mut Budget) -> Result<()> {
    for f in a.generators().iter().chain(z.generators()) {
        local.supports(f)?;
    }
    for (left, right) in [(a, z), (z, a)] {
        let right = local.ideal().sum(right, b)?;
        let mut membership = PreparedMembership::new(&right, local.unit_relations(), b)?;
        for f in left.generators() {
            if !membership.contains(f, b)? {
                return Err(Error::Invalid("ambient center ideal equality"));
            }
        }
    }
    Ok(())
}

/// A compatible ideal sheaf represented on the retained finite ambient cover.
/// Local admissibility is inherited from the actual monomial-center issuers.
/// This object does not certify that BM's GLOBAL invariant chose the center.
#[derive(Clone, Debug)]
pub struct GluedAdmissibleCenter {
    atlas: Arc<PrincipalCenterAtlas>,
    centers: Vec<Arc<LocalCenter>>,
    overlaps: Vec<Arc<CenterOverlap>>,
}
impl GluedAdmissibleCenter {
    pub fn verify(
        atlas: Arc<PrincipalCenterAtlas>,
        centers: Vec<Arc<LocalCenter>>,
        overlaps: Vec<Arc<CenterOverlap>>,
        b: &mut Budget,
    ) -> Result<Arc<Self>> {
        let n = atlas.len();
        let pairs = n
            .checked_mul(n.saturating_sub(1))
            .and_then(|v| v.checked_div(2))
            .ok_or(Error::ResourceIncomplete("ambient center pair count"))?;
        b.reserve_slots(n)?;
        b.reserve_slots(pairs)?;
        if centers.len() != n || overlaps.len() != pairs {
            return Err(Error::Invalid(
                "incomplete ambient center/open/overlap inventory",
            ));
        }
        let mut centers = centers;
        centers.sort_by_key(|c| c.index());
        for (i, c) in centers.iter().enumerate() {
            if !Arc::ptr_eq(c.atlas(), &atlas) || c.index() != i {
                return Err(Error::Invalid(
                    "ambient center original source/open association",
                ));
            }
        }
        let mut overlaps = overlaps;
        overlaps.sort_by_key(|o| o.indices());
        let mut k = 0;
        for i in 0..n {
            for j in i + 1..n {
                let o = &overlaps[k];
                if o.indices() != (i, j)
                    || !Arc::ptr_eq(o.left(), &centers[i])
                    || !Arc::ptr_eq(o.right(), &centers[j])
                {
                    return Err(Error::Invalid(
                        "ambient center complete overlap association",
                    ));
                }
                k += 1;
            }
        }
        Ok(Arc::new(Self {
            atlas,
            centers,
            overlaps,
        }))
    }
    pub fn atlas(&self) -> &Arc<PrincipalCenterAtlas> {
        &self.atlas
    }
    pub fn centers(&self) -> &[Arc<LocalCenter>] {
        &self.centers
    }
    pub fn overlaps(&self) -> &[Arc<CenterOverlap>] {
        &self.overlaps
    }
}

#[cfg(test)]
mod tests;
