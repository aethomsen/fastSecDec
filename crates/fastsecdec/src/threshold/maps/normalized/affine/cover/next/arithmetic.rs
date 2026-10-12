use super::*;
use symbolica::{
    domains::{integer::Z, rational::Q},
    poly::PolyVariable,
};
use symgcad::algebra::Poly;

pub(super) struct Ratio {
    pub numerator: Atom,
    pub denominator: Atom,
}
pub(super) fn ratio(a: &Atom, roles: &[Symbol], b: &Budget) -> Result<Ratio> {
    if !a
        .get_all_symbols(true)
        .is_subset(&roles.iter().copied().collect())
    {
        return Err(Error::Invalid(
            "next section rational expression has foreign roles".into(),
        ));
    }
    let limits = crate::threshold::regularization::meromorphic::Limits {
        degree: u16::try_from(b.limits.max_degree).map_err(native)?,
        terms: b.limits.max_terms,
        source_nodes: b.limits.max_terms.saturating_mul(8),
        integer_power: i64::try_from(b.limits.max_degree).map_err(native)?,
        ..Default::default()
    };
    crate::threshold::regularization::meromorphic::preflight_expression(a, limits)
        .map_err(|e| Error::ResourceIncomplete(format!("next rational preflight: {e:?}")))?;
    let variables = Arc::new(roles.iter().copied().map(PolyVariable::from).collect());
    b.check()?;
    let fraction = a
        .try_to_rational_polynomial::<_, _, u32>(&Q, &Z, Some(variables))
        .map_err(native)?;
    let n = fraction
        .numerator
        .map_coeff(|c| Rational::from(c.clone()), Q);
    let d = fraction
        .denominator
        .map_coeff(|c| Rational::from(c.clone()), Q);
    algebra::check(&n, b)?;
    algebra::check(&d, b)?;
    if d.is_zero() {
        return Err(Error::Invalid("next rational zero denominator".into()));
    }
    Ok(Ratio {
        numerator: n.to_expression(),
        denominator: d.to_expression(),
    })
}
pub(super) fn mul(a: &Poly, c: &Poly, b: &Budget) -> Result<Poly> {
    if a.variables() != c.variables() {
        return Err(Error::Invalid("next product role map".into()));
    }
    if a.nterms()
        .checked_mul(c.nterms())
        .is_none_or(|n| n > b.limits.max_terms)
        || (0..a.nvars()).any(|i| {
            a.degree(i)
                .checked_add(c.degree(i))
                .is_none_or(|d| d as usize > b.limits.max_degree)
        })
    {
        return Err(Error::ResourceIncomplete(
            "next exact product degree/term allowance".into(),
        ));
    }
    b.check()?;
    let p = a * c;
    algebra::check(&p, b)?;
    Ok(p)
}

/// Clear the single original-root substitution by the native linear-first
/// resultant. This retains one shared denominator instead of re-expanding a
/// rational Atom term by term. Native homogeneous Horner is degree-generic.
pub(super) fn substitute_root(
    original: &Atom,
    root: Symbol,
    roles: &[Symbol],
    centre: &RationalReceipt,
    scale: &RationalReceipt,
    b: &Budget,
) -> Result<Ratio> {
    if roles.is_empty() || roles.contains(&root) {
        return Err(Error::Invalid(
            "distinct complete rational substitution roles".into(),
        ));
    }
    let mut all = roles.to_vec();
    all.push(root);
    let ring = algebra::Ring::new(all)?;
    let source = ring.polynomial(original, b)?;
    let axis = roles.len();
    let degree = source.degree(axis) as usize;
    if degree == 0 {
        return Err(Error::Invalid(
            "original selected-root equation degree vanished".into(),
        ));
    }
    // Canonicalize only this small affine image, retaining one common
    // denominator. The full original P never enters the rational AST preflight.
    let image = ratio(
        &(centre.expression() + scale.expression() * Atom::var(roles[axis - 1])),
        roles,
        b,
    )?;
    let numerator = ring.polynomial(&image.numerator, b)?;
    let denominator = ring.polynomial(&image.denominator, b)?;
    if denominator.is_zero() {
        return Err(Error::Invalid(
            "zero canonical substitution denominator".into(),
        ));
    }
    // Native homogeneous Horner also computes the final unused d^(degree+1).
    // Bound that temporary separately; the returned denominator remains d^degree.
    let workspace_power = degree
        .checked_add(1)
        .and_then(|n| u32::try_from(n).ok())
        .ok_or_else(|| Error::ResourceIncomplete("native denominator workspace exponent".into()))?;
    if denominator
        .nterms()
        .checked_pow(workspace_power)
        .is_none_or(|v| v > b.limits.max_terms)
        || (0..axis).any(|i| {
            (denominator.degree(i) as usize)
                .checked_mul(workspace_power as usize)
                .is_none_or(|v| v > b.limits.max_degree)
        })
    {
        return Err(Error::ResourceIncomplete(
            "native root denominator power allowance".into(),
        ));
    }
    let coefficients = source.to_univariate_polynomial_list(axis);
    let mut terms = 0usize;
    // Bound every term of d^degree*P(n/d) before invoking native resultant.
    // This is a workspace bound, not a claim that cancellation is absent.
    for (coefficient, exponent) in coefficients {
        let exponent = exponent as usize;
        let count = numerator
            .nterms()
            .checked_pow(exponent as u32)
            .and_then(|v| {
                denominator
                    .nterms()
                    .checked_pow((degree - exponent) as u32)
                    .and_then(|d| v.checked_mul(d))
            })
            .and_then(|v| v.checked_mul(coefficient.nterms()));
        terms = count
            .and_then(|v| terms.checked_add(v))
            .filter(|v| *v <= b.limits.max_terms)
            .ok_or_else(|| {
                Error::ResourceIncomplete("native root resultant term allowance".into())
            })?;
        for i in 0..axis {
            let bound = (numerator.degree(i) as usize)
                .checked_mul(exponent)
                .and_then(|n| {
                    (denominator.degree(i) as usize)
                        .checked_mul(degree - exponent)
                        .and_then(|d| n.checked_add(d))
                })
                .and_then(|v| v.checked_add(coefficient.degree(i) as usize));
            if bound.is_none_or(|v| v > b.limits.max_degree) {
                return Err(Error::ResourceIncomplete(
                    "native root resultant degree allowance".into(),
                ));
            }
        }
    }
    let variable = ring.polynomial(&Atom::var(root), b)?;
    let linear = &mul(&denominator, &variable, b)? - &numerator;
    algebra::check(&linear, b)?;
    b.check()?;
    let cleared = linear
        .to_univariate(axis)
        .resultant(&source.to_univariate(axis));
    algebra::check(&cleared, b)?;
    let denominator = denominator.pow(degree);
    algebra::check(&denominator, b)?;
    if let Some(i) = cleared
        .variables()
        .iter()
        .position(|v| v == &ring.native.variables[axis])
        && cleared.degree(i) != 0
    {
        return Err(Error::Invalid(
            "native resultant retained original root role".into(),
        ));
    }
    Ok(Ratio {
        numerator: ring.expression(&cleared),
        denominator: ring.expression(&denominator),
    })
}
