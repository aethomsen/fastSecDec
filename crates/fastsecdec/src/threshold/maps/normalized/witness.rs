use super::{
    Budget, Error, NormalizedPrefix, PositiveProjectiveCellMap, Result, algebra,
    certificate::SectionProposal, native,
};
use symbolica::{atom::Atom, domains::rational::Rational};
use symgcad::{
    algebra::{Algebra, Poly, rational},
    output::{Axis, RootBound, RootIndexDomain, SolveResult},
    roots::{FiberRoot, isolate_union, refine_once},
};

/// Only a retained native geometry owner supplies original selector authority.
#[derive(Clone, Copy)]
pub(super) enum Source<'a> {
    Positive(&'a PositiveProjectiveCellMap),
    Affine(&'a super::super::CellMap),
}
impl<'a> Source<'a> {
    pub fn raw(self) -> &'a SolveResult {
        match self {
            Self::Positive(s) => s.decomposition().native_result(),
            Self::Affine(s) => s.decomposition().native_result(),
        }
    }
    pub fn axes(self) -> &'a [Axis] {
        match self {
            Self::Positive(s) => s.native_axes(),
            Self::Affine(s) => s.native_axes(),
        }
    }
}

pub(super) fn polynomial(
    source: &PositiveProjectiveCellMap,
    bound: &RootBound,
    b: &Budget,
) -> Result<Poly> {
    polynomial_in(Source::Positive(source), bound, b)
}
pub(super) fn polynomial_in(source: Source<'_>, bound: &RootBound, b: &Budget) -> Result<Poly> {
    let raw = source.raw();
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
    constant_bound_in(Source::Positive(source), axis, upper, b)
}
pub(super) fn constant_bound_in(
    source: Source<'_>,
    axis: usize,
    upper: bool,
    b: &Budget,
) -> Result<Rational> {
    b.check()?;
    let axis_record = source
        .axes()
        .get(axis)
        .ok_or_else(|| Error::Invalid("source axis inventory".into()))?;
    let bound = if upper {
        &axis_record.upper
    } else {
        &axis_record.lower
    }
    .as_ref()
    .ok_or_else(|| Error::Unsupported("constant finite interval bound required".into()))?;
    let p = polynomial_in(source, bound, b)?;
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
pub(super) fn selected(
    source: Source<'_>,
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
    let raw = source.raw();
    let a = Algebra::new(&raw.order).map_err(native)?;
    let p = polynomial_in(source, bound, b)?;
    let fiber = a.specialize_univariate(&p, axis, prefix).map_err(native)?;
    let roots = isolate_union(vec![(bound.polynomial, fiber)]).map_err(native)?;
    b.check()?;
    roots
        .get(bound.index)
        .cloned()
        .ok_or_else(|| Error::Invalid("original selected witness root absent".into()))
}
pub(super) fn compare(
    root: &mut FiberRoot,
    value: &Rational,
    b: &Budget,
) -> Result<std::cmp::Ordering> {
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
    let _ = original; // Retained by the caller; selection reuses its actual source table row.
    associate_in(
        Source::Positive(&prefix.source),
        prefix.len(),
        p,
        bound,
        lower,
        b,
    )
}
pub(super) fn associate_in(
    source: Source<'_>,
    axis: usize,
    p: &SectionProposal,
    bound: &RootBound,
    lower: &Rational,
    b: &Budget,
) -> Result<FiberRoot> {
    let c = &p.chart;
    if c.source_prefix.len() != axis || c.witness.len() != axis {
        return Err(Error::Invalid(
            "complete original prefix witness required".into(),
        ));
    }
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
            (false, &source.axes()[axis].lower),
            (true, &source.axes()[axis].upper),
        ] {
            if let Some(bound) = which {
                let mut root = selected(source, axis, bound, &point[..axis], b)?;
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
    let mut anchor = selected(source, axis, bound, &point, b)?;
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
