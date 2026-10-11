use super::{
    Budget, Error, NormalizedPrefix, PositiveProjectiveCellMap, Result, algebra,
    certificate::SectionProposal, native,
};
use symbolica::{atom::Atom, domains::rational::Rational};
use symgcad::{
    algebra::{Algebra, Poly, rational},
    output::{RootBound, RootIndexDomain},
    roots::{FiberRoot, isolate_union, refine_once},
};

pub(super) fn polynomial(
    source: &PositiveProjectiveCellMap,
    bound: &RootBound,
    b: &Budget,
) -> Result<Poly> {
    let raw = source.decomposition().native_result();
    let a = Algebra::new(&raw.order).map_err(native)?;
    let p = a
        .parse(
            raw.polynomials
                .get(bound.polynomial)
                .ok_or_else(|| Error::Invalid("native bound polynomial index".into()))?,
        )
        .map_err(native)?;
    algebra::check(&p, b)?;
    Ok(p)
}
pub(super) fn constant_bound(
    source: &PositiveProjectiveCellMap,
    axis: usize,
    upper: bool,
    b: &Budget,
) -> Result<Rational> {
    b.check()?;
    let axis_record = source
        .native_axes()
        .get(axis)
        .ok_or_else(|| Error::Invalid("source axis inventory".into()))?;
    let bound = if upper {
        &axis_record.upper
    } else {
        &axis_record.lower
    }
    .as_ref()
    .ok_or_else(|| Error::Unsupported("constant finite interval bound required".into()))?;
    let p = polynomial(source, bound, b)?;
    if (0..p.nvars()).any(|i| i != axis && p.degree(i) > 0) {
        return Err(Error::Unsupported("source bound is not constant".into()));
    }
    let lo = rational(&bound.lower).map_err(native)?;
    let hi = rational(&bound.upper).map_err(native)?;
    if lo != hi || !p.replace(axis, &lo).is_zero() {
        return Err(Error::Unsupported(
            "exact rational native constant bound required".into(),
        ));
    }
    b.rational(&lo)?;
    Ok(lo)
}
fn selected(
    source: &PositiveProjectiveCellMap,
    axis: usize,
    bound: &RootBound,
    prefix: &[Rational],
    b: &Budget,
) -> Result<FiberRoot> {
    if bound.index_domain != RootIndexDomain::Real {
        return Err(Error::Unsupported(
            "normalized witness currently requires distinct-real native selectors".into(),
        ));
    }
    let raw = source.decomposition().native_result();
    let a = Algebra::new(&raw.order).map_err(native)?;
    let p = polynomial(source, bound, b)?;
    let fiber = a.specialize_univariate(&p, axis, prefix).map_err(native)?;
    let roots = isolate_union(vec![(bound.polynomial, fiber)]).map_err(native)?;
    b.check()?;
    roots
        .get(bound.index)
        .cloned()
        .ok_or_else(|| Error::Invalid("original selected witness root absent".into()))
}
fn compare(root: &mut FiberRoot, value: &Rational, b: &Budget) -> Result<std::cmp::Ordering> {
    for _ in 0..b.limits.max_refinements {
        b.check()?;
        if root.interval.1 < *value {
            return Ok(std::cmp::Ordering::Less);
        }
        if root.interval.0 > *value {
            return Ok(std::cmp::Ordering::Greater);
        }
        if root.fiber.evaluate(value).is_zero() {
            return Ok(std::cmp::Ordering::Equal);
        }
        refine_once(root);
    }
    Err(Error::ResourceIncomplete(
        "source witness root comparison allowance".into(),
    ))
}
pub(super) fn associate(
    prefix: &NormalizedPrefix,
    p: &SectionProposal,
    bound: &RootBound,
    original: &Poly,
    lower: &Rational,
    b: &Budget,
) -> Result<FiberRoot> {
    let c = &p.chart;
    for (q, [lo, hi]) in c.witness.iter().zip(&c.bounds) {
        b.rational(q)?;
        if q <= lo || q >= hi {
            return Err(Error::Invalid(
                "interior coefficient witness required".into(),
            ));
        }
    }
    let values = c.witness.iter().cloned().map(Atom::num).collect::<Vec<_>>();
    let evaluate = |a: &Atom| {
        Rational::try_from(algebra::pull(a, &c.parameters, &values).as_view()).map_err(native)
    };
    let point = c
        .source_prefix
        .iter()
        .map(evaluate)
        .collect::<Result<Vec<_>>>()?;
    // This is a point of the exact original prefix, not merely of its outer
    // parameter box. Combined with the existing open-bijection induction and
    // nonsingular monomial chart it proves the witness belongs to the same
    // pulled coefficient branch, including h=v*Z for the two-level case.
    for axis in 0..point.len() {
        for (upper, which) in [
            (false, &prefix.source.native_axes()[axis].lower),
            (true, &prefix.source.native_axes()[axis].upper),
        ] {
            if let Some(bound) = which {
                let mut root = selected(&prefix.source, axis, bound, &point[..axis], b)?;
                let order = compare(&mut root, &point[axis], b)?;
                if order
                    != if upper {
                        std::cmp::Ordering::Greater
                    } else {
                        std::cmp::Ordering::Less
                    }
                {
                    return Err(Error::Invalid(
                        "coefficient witness lies outside original native prefix".into(),
                    ));
                }
            }
        }
    }
    let scale = evaluate(&p.scale)?;
    if scale <= Rational::zero() {
        return Err(Error::Invalid("witness root scale is not positive".into()));
    }
    let lo = lower + &(&scale * &p.bracket.lower);
    let hi = lower + &(&scale * &p.bracket.upper);
    let mut anchor = selected(&prefix.source, prefix.len(), bound, &point, b)?;
    let _ = original; // Original polynomial is retained by the caller; selected() reuses the same table row.
    for _ in 0..b.limits.max_refinements {
        b.check()?;
        if anchor.interval.0 >= lo && anchor.interval.1 <= hi {
            return Ok(anchor);
        }
        if anchor.interval.1 < lo || anchor.interval.0 > hi {
            return Err(Error::Invalid(
                "normalized bracket selects another original root".into(),
            ));
        }
        for endpoint in [&lo, &hi] {
            if anchor.interval.0 <= *endpoint
                && anchor.interval.1 >= *endpoint
                && anchor.fiber.evaluate(endpoint).is_zero()
            {
                anchor.interval = (endpoint.clone(), endpoint.clone());
                return Ok(anchor);
            }
        }
        refine_once(&mut anchor);
    }
    Err(Error::ResourceIncomplete(
        "normalized original-selector association allowance".into(),
    ))
}
