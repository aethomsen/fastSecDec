use super::super::projective_family::algebra::{Field, Polynomial, pull};
use super::*;
use crate::{parametric::FactorRole, threshold::maps::PositiveProjectiveCellMap};
use std::cmp::Ordering;
#[path = "pair.rs"]
mod pair;
#[path = "ray.rs"]
mod ray;
struct Context<'a> {
    owner: &'a Arc<PositiveProjectiveQuotient>,
    cell: usize,
    units: &'a [Symbol],
    root: Symbol,
    lo: Atom,
    hi: Option<Atom>,
    factors: Vec<(usize, usize, Atom)>,
    field: Field,
}

fn canonical(a: &Atom, f: &mut Field, w: &Work<'_>) -> Result<Atom> {
    let r = f.ratio(a, w)?;
    Ok(f.expression(&r.numerator)? / f.expression(&r.denominator)?)
}
fn zero(a: &Atom, f: &mut Field, w: &Work<'_>) -> Result<bool> {
    Ok(f.ratio(a, w)?.numerator.is_zero())
}
fn coeffs(p: &Atom, f: &mut Field, w: &Work<'_>) -> Result<Vec<Atom>> {
    let p = f.polynomial(p, w)?;
    let u = p.to_univariate(1);
    u.coefficients().iter().map(|c| f.expression(c)).collect()
}
fn at(a: &Atom, u: Symbol, x: &Atom, f: &mut Field, w: &Work<'_>) -> Result<Atom> {
    canonical(&pull(a, &[u], std::slice::from_ref(x)), f, w)
}
fn basic(fractions: [Rational; 2], shape: PrefixShape) -> PrefixPiece {
    PrefixPiece {
        fractions,
        shape,
        roots: Vec::new(),
        fibre: FibreProposal::Native {
            lower: Atom::zero(),
            upper: None,
            reversed: false,
        },
        rewrites: Vec::new(),
    }
}
fn source_factors(
    owner: &Arc<PositiveProjectiveQuotient>,
    cell: usize,
    units: &[Symbol],
) -> Result<Vec<(usize, usize, Atom)>> {
    let map =
        PositiveProjectiveCellMap::new(owner.clone(), cell, units.to_vec()).map_err(native)?;
    let source = map.axes().iter().map(|a| a.symbol()).collect::<Vec<_>>();
    let mut result = Vec::new();
    for (ti, t) in owner.preparation().terms().iter().enumerate() {
        for (fi, p) in t.factors().iter().enumerate() {
            if p.role() != FactorRole::Polynomial && !p.exponent().is_zero() {
                result.push((
                    ti,
                    fi,
                    pull(
                        p.polynomial(),
                        &source,
                        &units.iter().copied().map(Atom::var).collect::<Vec<_>>(),
                    ),
                ));
            }
        }
    }
    Ok(result)
}
fn bounds(
    owner: &PositiveProjectiveQuotient,
    cell: usize,
    units: &[Symbol],
) -> Result<[Option<Atom>; 2]> {
    let images = units.iter().copied().map(Atom::var).collect::<Vec<_>>();
    Ok([SectionSide::Lower, SectionSide::Upper]
        .map(|side| {
            geometry::bound(owner, cell, 1, side).and_then(|b| {
                b.map(|b| geometry::native_polynomial(owner, b, &images))
                    .transpose()
            })
        })
        .into_iter()
        .collect::<Result<Vec<_>>>()?
        .try_into()
        .unwrap())
}
struct RewriteTarget<'a> {
    image: &'a [Atom],
    body: &'a Atom,
    fibre: usize,
}
fn rewrite_rows(
    p: &Polynomial,
    factors: &[(usize, usize, Atom)],
    target: RewriteTarget<'_>,
    units: &[Symbol],
    field: &mut Field,
    w: &Work<'_>,
) -> Result<Vec<FactorRewrite>> {
    let mut rows = Vec::new();
    for (ti, fi, source) in factors {
        let s = field.polynomial(source, w)?;
        let (q, r) = s.quot_rem(p, false);
        if r.is_zero() {
            let q = field.expression(&q)?;
            rows.push(FactorRewrite {
                fibre: target.fibre,
                term: *ti,
                factor: *fi,
                expression: pull(&q, units, target.image) * target.body,
            });
        }
    }
    Ok(rows)
}
fn discriminant(p: &Polynomial, f: &mut Field, w: &Work<'_>) -> Result<Atom> {
    if p.degree(1) != 2 {
        return Err(unsupported(
            "automatic paired discovery currently requires quadratic fibre",
        ));
    }
    let u = p.to_univariate(1);
    let r = u.resultant(&u.derivative());
    // Native discriminant identity for degree2: -Res(P,P') / leading(P).
    let result = -f.expression(&r)? / f.expression(&u.lcoeff())?;
    canonical(&result, f, w)
}
fn bracket(h: &Atom, u: Symbol, f: &mut Field, w: &Work<'_>) -> Result<(Rational, Rational)> {
    let samples = [Rational::zero(), Rational::from((1, 2)), Rational::one()]
        .into_iter()
        .map(|x| at(h, u, &Atom::num(x), f, w))
        .collect::<Result<Vec<_>>>()?;
    let mut low = Rational::one();
    let mut high = Rational::one();
    for _ in 0..64 {
        let square = Atom::num(&low * &low);
        let mut positive = true;
        for s in &samples {
            positive &= geometry::constant_sign(&(s - &square), f, w)? == Ordering::Greater;
        }
        if positive {
            break;
        }
        low /= Rational::from(2);
    }
    for _ in 0..64 {
        let square = Atom::num(&high * &high);
        let mut positive = true;
        for s in &samples {
            positive &= geometry::constant_sign(&(&square - s), f, w)? == Ordering::Greater;
        }
        if positive {
            return Ok((low, high));
        }
        high *= Rational::from(2);
    }
    Err(unsupported(
        "bounded normalized-root bracket proposal search",
    ))
}

pub(super) fn enumerate(
    owner: &Arc<PositiveProjectiveQuotient>,
    cell: usize,
    units: &[Symbol],
    root: Symbol,
    primitive: Symbol,
    options: &DiscoveryOptions,
    w: &mut Work<'_>,
) -> Result<Vec<CellProposal>> {
    let lo = geometry::selected_constant(
        owner,
        ConstantReference {
            cell,
            side: SectionSide::Lower,
        },
        w.limits,
    )?;
    let hi = geometry::bound(owner, cell, 0, SectionSide::Upper)?
        .map(|_| {
            geometry::selected_constant(
                owner,
                ConstantReference {
                    cell,
                    side: SectionSide::Upper,
                },
                w.limits,
            )
        })
        .transpose()?;
    let constants = std::iter::once(lo.clone())
        .chain(hi.iter().cloned())
        .collect::<Vec<_>>();
    let roles = units.iter().copied().chain([root]).collect::<Vec<_>>();
    let mut field = Field::new(&constants, &roles, primitive, w)?;
    let factors = source_factors(owner, cell, units)?;
    let bound_polys = bounds(owner, cell, units)?;
    let mut moving = None;
    for p in bound_polys.iter().flatten() {
        let pp = field.polynomial(p, w)?;
        if pp.degree(0) > 0 {
            if pp.degree(1) != 2 {
                return Err(unsupported(
                    "automatic nonlinear fibre discovery needs a simple quadratic pair",
                ));
            }
            if let Some(old) = &moving {
                if !zero(&(p - old), &mut field, w)? {
                    return Err(unsupported("distinct moving fibre polynomials"));
                }
            } else {
                moving = Some(p.clone())
            }
        }
    }
    let context = Context {
        owner,
        cell,
        units,
        root,
        lo,
        hi,
        factors,
        field,
    };
    match moving {
        Some(p) => pair::moving(context, p, w),
        None => ray::rays(context, bound_polys, options, w),
    }
}
