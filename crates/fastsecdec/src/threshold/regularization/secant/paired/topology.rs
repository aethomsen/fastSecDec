//! One complete paired partition in a proved source domain. The projective
//! sufficient class is the actual one-coordinate delta-gauge preparation.
use super::*;
use crate::threshold::gcad::{AliasRole, DomainOrigin};

pub(super) fn fixed(
    owner: &Arc<VerifiedDecomposition>,
    coordinates: &[Symbol],
) -> Result<[Arc<CellMap>; 3]> {
    let request = owner.request();
    if request.domain().coordinates().is_empty()
        || !matches!(request.domain().origin(), DomainOrigin::NativeUnitCube)
        || !request.kinematics().runtime_parameters.is_empty()
    {
        return Err(unsupported("paired issuer requires a fixed native cube"));
    }
    partition(owner, coordinates, 0)
}

pub(super) fn uniform(
    owner: &Arc<VerifiedDecomposition>,
    coordinates: &[Symbol],
) -> Result<[Arc<CellMap>; 3]> {
    let request = owner.request();
    let n = request.domain().coordinates().len();
    let prefix = request.kinematics().runtime_parameters.len();
    if n == 0 || prefix == 0 {
        return Err(unsupported(
            "uniform paired issuer requires integration axes and runtime parameters",
        ));
    }
    match request.domain().origin() {
        DomainOrigin::NativeUnitCube => {}
        DomainOrigin::AffineProjective { eliminated_index } if n == 1 => {
            let preparation = request
                .projective_preparation()
                .ok_or_else(|| invalid("projective paired source lost its native preparation"))?;
            if preparation.input().parameters().len() != 2
                || preparation.eliminated_index() != *eliminated_index
                || preparation.coordinates() != request.domain().coordinates()
                || preparation.strict_positive() != request.domain().strict_positive()
                || preparation.measure_jacobian() != &Atom::one()
            {
                return Err(invalid("projective paired source/gauge association"));
            }
        }
        _ => {
            return Err(unsupported(
                "uniform paired source requires a native cube or actual one-axis affine projective preparation",
            ));
        }
    }
    partition(owner, coordinates, prefix)
}

fn partition(
    owner: &Arc<VerifiedDecomposition>,
    coordinates: &[Symbol],
    prefix: usize,
) -> Result<[Arc<CellMap>; 3]> {
    let request = owner.request();
    let raw_count = request
        .domain()
        .coordinates()
        .len()
        .checked_add(prefix)
        .ok_or_else(|| invalid("paired raw input count"))?;
    if owner.cells().len() != 3 {
        return Err(unsupported(
            "paired issuer requires the complete three-cell inventory",
        ));
    }
    let mut outer_lower = None;
    let mut middle = None;
    let mut outer_upper = None;
    for cell in owner.cells() {
        let map =
            CellMap::new(owner.clone(), cell.index(), coordinates.to_vec()).map_err(native)?;
        if map.axes().len() != raw_count
            || map.axes()[..prefix]
                .iter()
                .any(|a| a.role() != AliasRole::RuntimeParameter)
            || map.axes()[prefix..]
                .iter()
                .any(|a| a.role() != AliasRole::IntegrationCoordinate)
        {
            return Err(unsupported(
                "paired parameter-prefix/integration-axis roles",
            ));
        }
        // Metadata currently binds section axes in original input order. Decline
        // other valid lifting orders until that projection is generalized.
        if !map.axes()[prefix..]
            .iter()
            .map(|axis| axis.symbol())
            .eq(request.domain().coordinates().iter().copied())
        {
            return Err(unsupported(
                "paired issuer requires original integration-axis order",
            ));
        }
        for axis in prefix..raw_count - 1 {
            if geometry::constant_bound(&map, axis, SectionSide::Lower)? != Some(Rational::zero())
                || geometry::constant_bound(&map, axis, SectionSide::Upper)?
                    != Some(Rational::one())
            {
                return Err(unsupported(
                    "paired prefix requires additional coverage partition",
                ));
            }
        }
        let l = geometry::constant_bound(&map, raw_count - 1, SectionSide::Lower)?;
        let u = geometry::constant_bound(&map, raw_count - 1, SectionSide::Upper)?;
        let slot = match (l, u) {
            (Some(l), None) if l.is_zero() => &mut outer_lower,
            (None, None) => &mut middle,
            (None, Some(u)) if u == Rational::one() => &mut outer_upper,
            _ => {
                return Err(unsupported(
                    "paired cells do not form a unit-interval partition",
                ));
            }
        };
        if slot.replace(map).is_some() {
            return Err(invalid("duplicate paired cell role"));
        }
    }
    let lower = outer_lower.ok_or_else(|| invalid("missing lower outer cell"))?;
    let middle = middle.ok_or_else(|| invalid("missing middle cell"))?;
    let upper = outer_upper.ok_or_else(|| invalid("missing upper outer cell"))?;
    let axis = raw_count - 1;
    if lower.native_axes()[axis].upper != middle.native_axes()[axis].lower
        || middle.native_axes()[axis].upper != upper.native_axes()[axis].lower
    {
        return Err(invalid("paired neighbor branch association"));
    }
    Ok([lower, middle, upper])
}
