//! Uniform closed implicit proof and actual original selected-branch admission.
use super::*;
pub(super) fn prepare(
    parent: Arc<RestrictedRootEquation>,
    witness: InteriorWitness,
    proposal: NextSectionProposal,
    b: &mut Budget,
) -> Result<Arc<RestrictedNextSection>> {
    b.check()?;
    let cover = parent.leaf().cover();
    let n = cover.parent().len();
    if !Arc::ptr_eq(witness.leaf().cover(), cover)
        || witness.leaf().index() != parent.leaf().index()
    {
        return Err(Error::Invalid(
            "next section witness belongs to another source leaf".into(),
        ));
    }
    if proposal.unit_powers.len() != n
        || proposal
            .unit_powers
            .iter()
            .any(|k| *k == 0 || *k as usize > b.limits.max_degree)
    {
        return Err(Error::Invalid(
            "complete bounded positive integer unit powers required".into(),
        ));
    }
    let roles = cover.slots().iter().map(|(s, _)| *s).collect::<Vec<_>>();
    let units = &roles[..n];
    let mut occupied = cover.parent().occupied();
    occupied.extend(&roles);
    occupied.insert(cover.root_symbol());
    if occupied.contains(&proposal.root_symbol) || !proposal.root_symbol.is_exportable() {
        return Err(Error::Invalid("next root role is not fresh".into()));
    }
    let powers = units
        .iter()
        .zip(&proposal.unit_powers)
        .map(|(u, k)| Atom::var(*u).pow(*k))
        .collect::<Vec<_>>();
    let measure = units
        .iter()
        .zip(&proposal.unit_powers)
        .map(|(u, k)| Atom::num(*k) * Atom::var(*u).pow(k - 1))
        .product::<Atom>();
    let ring = algebra::Ring::new(roles.clone())?;
    ring.polynomial(&measure, b)?;
    let bounds = cover
        .slots()
        .iter()
        .map(|(_, v)| v.clone())
        .collect::<Vec<_>>();
    let (constraints, domain) = ring.box_domain(&bounds, b)?;
    let relations = parent
        .relations()
        .iter()
        .map(|(s, p)| (*s, algebra::pull(p, units, &powers)))
        .collect::<Vec<_>>();
    b.rational(&proposal.unit_margin)?;
    b.rational(&proposal.bracket.lower)?;
    b.rational(&proposal.bracket.upper)?;
    b.rational(&proposal.bracket.derivative_margin)?;
    if proposal.unit_margin <= 0
        || proposal.bracket.derivative_margin <= 0
        || proposal.bracket.lower >= proposal.bracket.upper
    {
        return Err(Error::Invalid(
            "positive next margins and ordered bracket required".into(),
        ));
    }
    let receipt = |a: &Atom, nonzero: bool| -> Result<RationalReceipt> {
        let fraction = arithmetic::ratio(a, &roles, b)?;
        let denominator_unit = equation::unit(
            &ring,
            &roles,
            &fraction.denominator,
            &relations,
            &constraints,
            &domain,
            &proposal.unit_margin,
            b,
        )?;
        let mut numerator_unit = None;
        let mut orders = vec![0; n];
        if nonzero {
            let p = ring.polynomial(&fraction.numerator, b)?;
            if p.is_zero() {
                return Err(Error::Invalid("zero next scale/presentation".into()));
            }
            orders = (0..n).map(|i| p.degree_bounds(i).0).collect();
            let monomial = units
                .iter()
                .zip(&orders)
                .map(|(u, e)| Atom::var(*u).pow(*e))
                .product::<Atom>();
            let m = ring.polynomial(&monomial, b)?;
            let residual = p
                .try_div(&m)
                .ok_or_else(|| Error::Invalid("native next content division".into()))?;
            if arithmetic::mul(&m, &residual, b)? != p {
                return Err(Error::Invalid("native next content recombination".into()));
            }
            numerator_unit = Some(equation::unit(
                &ring,
                &roles,
                &ring.expression(&residual),
                &relations,
                &constraints,
                &domain,
                &proposal.unit_margin,
                b,
            )?);
        }
        Ok(RationalReceipt {
            numerator: fraction.numerator,
            denominator: fraction.denominator,
            powers: orders,
            numerator_unit,
            denominator_unit,
        })
    };
    let centre = receipt(&proposal.centre, false)?;
    let scale = receipt(&proposal.scale, true)?;
    let presentation = receipt(&proposal.presentation, true)?;
    if scale.numerator_unit.as_ref().unwrap().sign() * scale.denominator_unit.sign() != 1 {
        return Err(Error::Invalid(
            "next scale is not positive on open leaf".into(),
        ));
    }
    let mut root_roles = roles.clone();
    root_roles.push(proposal.root_symbol);
    let root_ring = algebra::Ring::new(root_roles.clone())?;
    let q = root_ring.polynomial(&proposal.equation, b)?;
    let degree = q.degree(roles.len()) as usize;
    if degree == 0 || degree > b.limits.max_degree {
        return Err(Error::Unsupported(
            "positive bounded normalized next root degree required".into(),
        ));
    }
    let total = arithmetic::substitute_root(
        &algebra::pull(parent.total_pullback(), units, &powers),
        cover.root_symbol(),
        &root_roles,
        &centre,
        &scale,
        b,
    )?;
    let left = arithmetic::mul(
        &root_ring.polynomial(&total.numerator, b)?,
        &root_ring.polynomial(&presentation.denominator, b)?,
        b,
    )?;
    let right = arithmetic::mul(
        &arithmetic::mul(
            &root_ring.polynomial(&total.denominator, b)?,
            &root_ring.polynomial(&presentation.numerator, b)?,
            b,
        )?,
        &q,
        b,
    )?;
    let difference = &left - &right;
    let (remaining, identity_quotients) = transport::reduce(
        &root_ring,
        &root_roles,
        &root_ring.expression(&difference),
        &relations,
        b,
    )?;
    if !remaining.is_zero() {
        return Err(Error::Unsupported(
            "proposed full next-root normalization identity not proved".into(),
        ));
    }
    // Runtime evaluates the canonical centre/scale receipts above. Their
    // denominators, and this canonical composed denominator, are closed units.
    // Original proposal syntax is retained only as provenance; a removable
    // syntactic pole is not reintroduced into the admitted runtime map.
    let mut root_bounds = bounds;
    root_bounds.push([
        proposal.bracket.lower.clone(),
        proposal.bracket.upper.clone(),
    ]);
    let (root_constraints, root_domain) = root_ring.box_domain(&root_bounds, b)?;
    let total_denominator = equation::unit(
        &root_ring,
        &root_roles,
        &total.denominator,
        &relations,
        &root_constraints,
        &root_domain,
        &proposal.unit_margin,
        b,
    )?;
    let derivative = equation::unit(
        &root_ring,
        &root_roles,
        &root_ring.expression(&q.derivative(roles.len())),
        &relations,
        &root_constraints,
        &root_domain,
        &proposal.bracket.derivative_margin,
        b,
    )?;
    let orientation = derivative.sign();
    let mut endpoints = Vec::with_capacity(2);
    for (i, value) in [
        proposal.bracket.lower.clone(),
        proposal.bracket.upper.clone(),
    ]
    .into_iter()
    .enumerate()
    {
        let endpoint = q.replace(roles.len(), &value);
        let target = root_ring.expression(&endpoint)
            * Atom::num(if i == 0 { -orientation } else { orientation });
        let mut accepted = None;
        for candidate in equation::representatives(&root_ring, &root_roles, &target, &relations, b)?
        {
            let (representative, quotients) = candidate?;
            let certificate = if representative.is_zero() {
                None
            } else {
                match root_ring.positive(
                    &root_ring.polynomial(&representative, b)?,
                    &root_constraints,
                    &root_domain,
                    b,
                ) {
                    Ok(c) => Some(c),
                    Err(Error::Unsupported(_)) => continue,
                    Err(e) => return Err(e),
                }
            };
            accepted = Some(EndpointReceipt {
                original: target.clone(),
                representative,
                quotients,
                certificate,
            });
            break;
        }
        endpoints.push(accepted.ok_or_else(|| {
            Error::Unsupported(format!(
                "next normalized {} endpoint sign not certified",
                if i == 0 { "lower" } else { "upper" }
            ))
        })?);
    }
    let mut witness_slots = witness::slot_values(&witness, b)?;
    for (i, k) in proposal.unit_powers.iter().enumerate() {
        let original = witness_slots[i].clone();
        witness_slots[i] = witness::positive_power(&original, &Rational::from((1, *k)), b)?;
        let forward = witness_slots[i].pow(u64::from(*k));
        if forward.lower_bound() > original.lower_bound()
            || forward.upper_bound() < original.upper_bound()
        {
            return Err(Error::Unsupported(
                "inverse power forward enclosure inconclusive".into(),
            ));
        }
    }
    let bits = witness.precision();
    let exact = |q: &Rational| RealBall::from_rational_bounds(q, q, bits);
    let eval = |r: &RationalReceipt| -> Result<RealBall> {
        let n = ring
            .polynomial(&r.numerator, b)?
            .evaluate(exact, &witness_slots);
        let d = ring
            .polynomial(&r.denominator, b)?
            .evaluate(exact, &witness_slots);
        if !d.is_strictly_positive() && !(-d.clone()).is_strictly_positive() {
            return Err(Error::Unsupported(
                "next witness denominator enclosure inconclusive".into(),
            ));
        }
        Ok(n / d)
    };
    let actual = witness::selected(
        cover.parent(),
        n,
        cover.next_bound(),
        witness.source_prefix(),
        bits,
        b,
    )?;
    let scale_at = eval(&scale)?;
    if !scale_at.is_strictly_positive() {
        return Err(Error::Unsupported(
            "next witness scale enclosure inconclusive".into(),
        ));
    }
    let normalized_witness = (actual - eval(&centre)?) / scale_at;
    if !(&normalized_witness - exact(&proposal.bracket.lower)).is_strictly_positive()
        || !(exact(&proposal.bracket.upper) - &normalized_witness).is_strictly_positive()
    {
        return Err(Error::Unsupported(
            "actual selected root not strictly inside proposed normalized bracket".into(),
        ));
    }
    let mut coefficients = vec![Atom::zero(); degree + 1];
    for (p, e) in q.to_univariate_polynomial_list(roles.len()) {
        coefficients[e as usize] = root_ring.expression(&p);
    }
    b.check()?;
    Ok(Arc::new(RestrictedNextSection {
        parent,
        witness,
        proposal,
        powers,
        measure,
        relations,
        centre,
        scale,
        presentation,
        identity_quotients,
        coefficients,
        orientation,
        total_denominator,
        derivative,
        endpoints,
        witness_slots,
        normalized_witness,
    }))
}
