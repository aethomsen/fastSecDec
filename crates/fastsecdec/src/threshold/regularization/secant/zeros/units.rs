//! Inverse factors may only use already issued open-chart nonzero authority.
use super::*;

pub(super) fn admitted(
    family: &NormalizedFamily,
    chart: usize,
    roles: [Symbol; 2],
    variables: &Arc<Vec<PolyVariable>>,
) -> Result<Option<Vec<Poly>>> {
    let ChartGeometry::Paired(geometry) = &family.charts[chart].geometry else {
        return Ok(None);
    };
    let n = family.coordinates.len();
    let t = Atom::var(family.coordinates[n - 1]);
    let mut atoms = vec![Atom::one()];
    // Coordinates and complements are nonzero on the sampled open cube only.
    for coordinate in &family.coordinates {
        atoms.extend([Atom::var(*coordinate), Atom::one() - Atom::var(*coordinate)]);
    }
    let (image, measure, magnitude) = geometry.normalized(&family.roots);
    // The paired issuer proves the entire magnitude product is strictly positive
    // on the closed root box, including both endpoint faces. Every scalar factor
    // is therefore nonzero there; this is stronger than bulk-only nonvanishing.
    for value in [measure, magnitude] {
        for face in [None, Some(0), Some(1)] {
            let value = match face {
                None => value.clone(),
                Some(v) => value.replace(t.clone()).with(Atom::num(v)),
            };
            factors(&value, &mut atoms);
        }
    }
    let mapped = family.coordinates[..n - 1]
        .iter()
        .copied()
        .map(Atom::var)
        .chain([image])
        .collect::<Vec<_>>();
    for term in &family.charts[chart].terms {
        for ledger in &term.ledger {
            if !matches!(ledger.kind, FactorKind::ClosedUnit { .. }) {
                continue;
            }
            let value = ledger
                .source
                .replace_multiple(family.formals.iter().zip(&mapped).map(|(s, a)| {
                    Replacement::new(Pattern::Literal(Atom::var(*s)), Pattern::Literal(a.clone()))
                }));
            // These ledger certificates are closed-box units, not open cells.
            for face in [None, Some(0), Some(1)] {
                let value = match face {
                    None => value.clone(),
                    Some(v) => value.replace(t.clone()).with(Atom::num(v)),
                };
                factors(&value, &mut atoms);
            }
        }
    }
    for (section, root) in geometry.pair().sections().iter().zip(&family.roots) {
        let actual = family.coordinates[..n - 1]
            .iter()
            .copied()
            .map(Atom::var)
            .chain([root.clone()]);
        atoms.push(
            section
                .equation()
                .derivative(family.formals[n - 1])
                .replace_multiple(family.formals.iter().zip(actual).map(|(s, a)| {
                    Replacement::new(Pattern::Literal(Atom::var(*s)), Pattern::Literal(a))
                })),
        );
    }
    // The positive gap certificate is separate from the two simple-root facts.
    atoms.push(&family.roots[1] - &family.roots[0]);
    if atoms.len() > family.limits.density.terms {
        return Err(Error::ResourceIncomplete(
            "coefficient-zero inverse inventory".into(),
        ));
    }
    let mut result = Vec::new();
    for atom in atoms {
        let Some(atom) = materialize(&atom, family, &[], roles)? else {
            return Ok(None);
        };
        let Some(p) = polynomial(&atom, variables) else {
            return Ok(None);
        };
        if p.is_zero() {
            return Err(invalid("issued nonzero unit vanished"));
        }
        if !result.contains(&p) {
            result.push(p)
        }
    }
    result.sort_by_cached_key(|p| {
        p.into_iter()
            .map(|m| (m.exponents.to_vec(), m.coefficient.to_string()))
            .collect::<Vec<_>>()
    });
    Ok(Some(result))
}
fn factors(a: &Atom, out: &mut Vec<Atom>) {
    match a.as_view() {
        AtomView::Mul(m) => {
            for a in m {
                factors(&a.to_owned(), out)
            }
        }
        AtomView::Pow(p) => out.push(p.get_base().to_owned()),
        _ => out.push(a.clone()),
    }
}

pub(super) fn inverses(
    scratch: &Atom,
    units: &[Poly],
    variables: &Arc<Vec<PolyVariable>>,
    family: &NormalizedFamily,
) -> Result<Option<Vec<(usize, u32)>>> {
    fn match_base(
        a: &Atom,
        power: u32,
        units: &[Poly],
        variables: &Arc<Vec<PolyVariable>>,
        out: &mut Vec<(usize, u32)>,
    ) -> bool {
        if let AtomView::Mul(m) = a.as_view() {
            return m
                .into_iter()
                .all(|v| match_base(&v.to_owned(), power, units, variables, out));
        }
        let Some(p) = polynomial(a, variables) else {
            return false;
        };
        for (i, unit) in units.iter().enumerate() {
            let (q, r) = p.quot_rem(unit, false);
            if r.is_zero() && q.is_constant() && !q.is_zero() {
                out.push((i, power));
                return true;
            }
        }
        false
    }
    let mut out = Vec::new();
    let mut supported = true;
    scratch.visitor(&mut |node| {
        if !supported {
            return false;
        }
        if let AtomView::Pow(p) = node {
            let Ok(exponent) = Rational::try_from(p.get_exp()) else {
                supported = false;
                return false;
            };
            let Some(exponent) = exponent
                .is_integer()
                .then(|| exponent.numerator_ref().to_i64())
                .flatten()
            else {
                supported = false;
                return false;
            };
            if exponent < 0 {
                let Some(power) = exponent.checked_neg().and_then(|v| u32::try_from(v).ok()) else {
                    supported = false;
                    return false;
                };
                if power > u32::from(family.limits.density.degree)
                    || !match_base(&p.get_base().to_owned(), power, units, variables, &mut out)
                {
                    supported = false;
                    return false;
                }
            }
        }
        true
    });
    if out.len() > family.limits.density.terms {
        return Err(Error::ResourceIncomplete(
            "coefficient-zero inverse factors".into(),
        ));
    }
    out.sort_unstable();
    Ok(supported.then_some(out))
}
