//! Shared native ascent from an actual matched lower center. Caller owners
//! establish source/history association; this checks full ambient unit authority.
use super::super::*;
use super::component_localization::pull_ideal;
use std::sync::Arc;
type Result<T> = std::result::Result<T, Error>;

pub(super) struct MatchedCenterGeometry {
    pub ideal: Ideal,
    pub normals: Vec<Poly>,
    pub clearings: Vec<UnitClearing>,
    pub child: Ideal,
}
pub(super) fn lift_matched_center(
    matched: &Arc<MatchedPhysicalSupport>,
    parent: &Arc<MarkedIdeal>,
    lower_ideal: &Ideal,
    lower_normals: &[Poly],
    b: &mut Budget,
) -> Result<MatchedCenterGeometry> {
    let child = pull_ideal(matched.lower_map(), lower_ideal, b)?;
    b.reserve_slots(lower_normals.len())?;
    let normals = lower_normals
        .iter()
        .map(|p| matched.lower_map().pull(p, b))
        .collect::<Result<Vec<_>>>()?;
    // Existing ascent needs these inverse factors on the full ambient
    // open. A unit only on the center is not an ambient coordinate inverse.
    let ambient = matched.ambient().local();
    for f in &normals {
        let c = clear_units(matched.frame().local(), f, b)?;
        for p in [&c.numerator, &c.denominator] {
            if (ambient.ring().len()..p.nvars()).any(|i| p.degree(i) > 0) {
                return Err(Error::ResourceIncomplete(
                    "outer center normal retains support graph slot",
                ));
            }
        }
        let d = super::super::elimination::remap(&c.denominator, ambient.ring(), b)?;
        let d = Ideal::new(ambient.ring().clone(), vec![d], b)?;
        if !ambient.ideal().sum(&d, b)?.contains(
            &ambient.ring().one(),
            ambient.unit_relations(),
            b,
        )? {
            return Err(Error::ResourceIncomplete(
                "outer center normal needs ambient denominator cover",
            ));
        }
    }
    let (ideal, normals, clearings) = super::lift::lift_embedded_geometry(
        matched.ambient(),
        parent,
        matched.frame(),
        matched.extension(),
        matched.support().equations().generators().to_vec(),
        &child,
        &normals,
        b,
    )?;
    if normals.is_empty() {
        return Err(Error::Invalid("outer dispatch positive codimension"));
    }
    Ok(MatchedCenterGeometry {
        ideal,
        normals,
        clearings,
        child,
    })
}
