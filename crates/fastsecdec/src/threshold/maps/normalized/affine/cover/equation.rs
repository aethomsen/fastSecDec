//! Full next equation, exact common content, and closed selected-tower units.
use super::*;
use symgcad::domain::{Domain, DomainCertificate};

/// The representative equals the original modulo the actual owned parent
/// equations. A certificate proves `sign*representative - margin > 0` on the
/// unchanged independent slot box; no slot is thereby made an input.
pub struct UnitReceipt {
    original: Atom,
    representative: Atom,
    quotients: Vec<Atom>,
    sign: i8,
    margin: Rational,
    target: Atom,
    certificate: DomainCertificate,
}
impl UnitReceipt {
    pub fn original(&self) -> &Atom {
        &self.original
    }
    pub fn representative(&self) -> &Atom {
        &self.representative
    }
    pub fn relation_quotients(&self) -> &[Atom] {
        &self.quotients
    }
    pub fn sign(&self) -> i8 {
        self.sign
    }
    pub fn margin(&self) -> &Rational {
        &self.margin
    }
    pub fn target(&self) -> &Atom {
        &self.target
    }
    pub fn certificate(&self) -> &DomainCertificate {
        &self.certificate
    }
}

/// This is equation preparation, not selected-next-root or density authority.
/// In particular a discriminant coordinate power still requires ramification.
pub struct RestrictedRootEquation {
    leaf: AffinePrefixLeaf,
    total: Atom,
    content: Atom,
    content_powers: Vec<u32>,
    equation: Atom,
    coefficients: Vec<Atom>,
    relations: Vec<(Symbol, Atom)>,
    leading: UnitReceipt,
    divisor_units: Vec<UnitReceipt>,
    discriminant_powers: Vec<u32>,
    discriminant_identity_quotients: Vec<Atom>,
    discriminant_unit: Atom,
    discriminant_unit_quotients: Vec<Atom>,
    constraints: Vec<symgcad::algebra::Poly>,
    domain: Domain,
}
impl RestrictedRootEquation {
    pub fn leaf(&self) -> &AffinePrefixLeaf {
        &self.leaf
    }
    pub fn total_pullback(&self) -> &Atom {
        &self.total
    }
    pub fn common_content(&self) -> &Atom {
        &self.content
    }
    pub fn content_powers(&self) -> &[u32] {
        &self.content_powers
    }
    pub fn equation(&self) -> &Atom {
        &self.equation
    }
    pub fn coefficients(&self) -> &[Atom] {
        &self.coefficients
    }
    pub fn relations(&self) -> &[(Symbol, Atom)] {
        &self.relations
    }
    pub fn leading_unit(&self) -> &UnitReceipt {
        &self.leading
    }
    pub fn divisor_units(&self) -> &[UnitReceipt] {
        &self.divisor_units
    }
    pub fn discriminant_powers(&self) -> &[u32] {
        &self.discriminant_powers
    }
    pub fn discriminant_identity_quotients(&self) -> &[Atom] {
        &self.discriminant_identity_quotients
    }
    /// Product of the retained certified residual representatives and original
    /// coefficient. The normalized discriminant equals its coordinate monomial
    /// times this unit modulo the genuine inherited tower, not necessarily as
    /// a formal polynomial in independent root slots.
    pub fn discriminant_unit(&self) -> &Atom {
        &self.discriminant_unit
    }
    pub fn discriminant_unit_quotients(&self) -> &[Atom] {
        &self.discriminant_unit_quotients
    }
    pub fn constraints(&self) -> &[symgcad::algebra::Poly] {
        &self.constraints
    }
    pub fn domain(&self) -> &Domain {
        &self.domain
    }
}

// Keep the native ring, exact role frame, and certificate domain explicit at
// this private proof boundary; no independently reconstructed context is used.
#[allow(clippy::too_many_arguments)]
pub(super) fn unit(
    ring: &algebra::Ring,
    roles: &[Symbol],
    original: &Atom,
    relations: &[(Symbol, Atom)],
    constraints: &[symgcad::algebra::Poly],
    domain: &Domain,
    margin: &Rational,
    b: &Budget,
) -> Result<UnitReceipt> {
    for candidate in representatives(ring, roles, original, relations, b)? {
        let (representative, quotients) = candidate?;
        for sign in [1i8, -1] {
            let target = Atom::num(sign) * &representative - Atom::num(margin.clone());
            let p = ring.polynomial(&target, b)?;
            match ring.positive(&p, constraints, domain, b) {
                Ok(certificate) => {
                    return Ok(UnitReceipt {
                        original: original.clone(),
                        representative,
                        quotients,
                        sign,
                        margin: margin.clone(),
                        target,
                        certificate,
                    });
                }
                Err(Error::Unsupported(_)) => {}
                Err(e) => return Err(e),
            }
        }
    }
    Err(Error::Unsupported(
        "closed selected-tower unit not certified".into(),
    ))
}

/// Try the actual original before doing any tower reduction. Later candidates
/// are the complete reduction and at most six single-relation omissions, in
/// inherited order: at most eight representatives, never subset enumeration.
/// Every candidate retains a complete indexed quotient identity and receives
/// its own certificate on the unchanged box. A miss is not non-equality.
pub(super) fn representatives<'a>(
    ring: &'a algebra::Ring,
    roles: &'a [Symbol],
    original: &Atom,
    relations: &'a [(Symbol, Atom)],
    b: &'a Budget,
) -> Result<impl Iterator<Item = Result<(Atom, Vec<Atom>)>> + 'a> {
    let canonical = ring.expression(&ring.polynomial(original, b)?);
    let first = std::iter::once(Ok((canonical.clone(), vec![Atom::zero(); relations.len()])));
    let mut seen = vec![canonical.clone()];
    let later = (0..=relations.len().min(6)).filter_map(move |candidate| {
        let reduced = if candidate == 0 {
            transport::reduce(ring, roles, &canonical, relations, b)
        } else {
            let omitted = candidate - 1;
            let subset = relations
                .iter()
                .enumerate()
                .filter(|(i, _)| *i != omitted)
                .map(|(_, r)| r.clone())
                .collect::<Vec<_>>();
            transport::reduce(ring, roles, &canonical, &subset, b).map(|(p, mut q)| {
                q.insert(omitted, Atom::zero());
                (p, q)
            })
        };
        match reduced {
            Ok((p, q)) => {
                if seen.contains(&p) {
                    None
                } else {
                    seen.push(p.clone());
                    Some(Ok((p, q)))
                }
            }
            Err(e) => Some(Err(e)),
        }
    });
    Ok(first.chain(later))
}

pub(super) fn prepare(
    leaf: AffinePrefixLeaf,
    margin: Rational,
    b: &mut Budget,
) -> Result<Arc<RestrictedRootEquation>> {
    b.check()?;
    b.rational(&margin)?;
    if margin <= Rational::zero() {
        return Err(Error::Invalid(
            "positive restricted unit margin required".into(),
        ));
    }
    let owner = leaf.cover();
    let part = leaf.restriction();
    let n = owner.parent().len();
    let roles = owner.slots().iter().map(|(s, _)| *s).collect::<Vec<_>>();
    let units = &roles[..n];
    let ring = algebra::Ring::new(roles.clone())?;
    let bounds = owner
        .slots()
        .iter()
        .map(|(_, v)| v.clone())
        .collect::<Vec<_>>();
    let (constraints, domain) = ring.box_domain(&bounds, b)?;
    let relations = owner
        .relations()
        .iter()
        .map(|r| {
            (
                r.dependent(),
                algebra::pull(r.equation(), units, part.unit_images()),
            )
        })
        .collect::<Vec<_>>();
    let mut all_roles = roles.clone();
    all_roles.push(owner.root_symbol());
    let full_ring = algebra::Ring::new(all_roles)?;
    let total = full_ring.polynomial(
        &algebra::pull(owner.total_pullback(), units, part.unit_images()),
        b,
    )?;
    if total.is_zero() {
        return Err(Error::Invalid("restricted next equation vanished".into()));
    }
    // Minimum over the entire polynomial, hence all root coefficients.
    let content_powers = (0..n).map(|i| total.degree_bounds(i).0).collect::<Vec<_>>();
    let content = units
        .iter()
        .zip(&content_powers)
        .map(|(s, e)| Atom::var(*s).pow(*e))
        .product::<Atom>();
    let content_poly = full_ring.polynomial(&content, b)?;
    let q = total
        .try_div(&content_poly)
        .ok_or_else(|| Error::Invalid("restricted full-equation content division".into()))?;
    if &content_poly * &q != total {
        return Err(Error::Invalid("restricted P=M*Q identity".into()));
    }
    let degree = q.degree(roles.len()) as usize;
    if degree == 0 || degree > symgcad::roots::MAX_ROOT_DEGREE {
        return Err(Error::Unsupported("restricted root degree".into()));
    }
    let leading_atom = full_ring.expression(&q.to_univariate(roles.len()).lcoeff());
    let leading = unit(
        &ring,
        &roles,
        &leading_atom,
        &relations,
        &constraints,
        &domain,
        &margin,
        b,
    )?;
    let mut divisor_units = Vec::new();
    for divisor in part.divisors() {
        divisor_units.push(unit(
            &ring,
            &roles,
            divisor.residual(),
            &relations,
            &constraints,
            &domain,
            &margin,
            b,
        )?);
    }
    let mut discriminant_powers = vec![0u32; n];
    for (target, divisor) in owner.divisor_targets.iter().zip(part.divisors()) {
        if *target == DivisorTarget::Discriminant {
            for (a, e) in discriminant_powers.iter_mut().zip(divisor.powers()) {
                *a = a.checked_add(*e).ok_or_else(|| {
                    Error::ResourceIncomplete("discriminant valuation overflow".into())
                })?;
            }
        }
    }
    let scale = u32::try_from(2 * (degree - 1)).map_err(native)?;
    for (a, e) in discriminant_powers.iter_mut().zip(&content_powers) {
        *a = a
            .checked_sub(
                e.checked_mul(scale).ok_or_else(|| {
                    Error::ResourceIncomplete("discriminant content scaling".into())
                })?,
            )
            .ok_or_else(|| {
                Error::Unsupported(
                    "proposed discriminant factors do not expose common content".into(),
                )
            })?;
    }
    let normalized_disc = transport::discriminant(&q, roles.len(), b)?;
    let disc = full_ring.expression(&normalized_disc);
    let rewrite = owner
        .rewrites
        .iter()
        .find(|r| r.target == DivisorTarget::Discriminant)
        .ok_or_else(|| Error::Invalid("discriminant provenance absent".into()))?;
    // Each full factor was already transported by exact native polynomial
    // substitution. Recombine those retained totals and the original rational
    // coefficient; do not expand the original reflected AST a second time.
    let mut transported = ring.polynomial(&Atom::num(rewrite.coefficient.clone()), b)?;
    for (target, factor) in owner.divisor_targets.iter().zip(part.divisors()) {
        if *target != DivisorTarget::Discriminant {
            continue;
        }
        let next = ring.polynomial(factor.total(), b)?;
        if transported
            .nterms()
            .checked_mul(next.nterms())
            .is_none_or(|v| v > b.limits.max_terms)
        {
            return Err(Error::ResourceIncomplete(
                "transported discriminant product allowance".into(),
            ));
        }
        transported = &transported * &next;
        algebra::check(&transported, b)?;
    }
    let transported = ring.expression(&transported);
    let divisor = content.pow(scale);
    let cross = &disc * &divisor - transported;
    let (rem, discriminant_identity_quotients) =
        transport::reduce(&ring, &roles, &cross, &relations, b)?;
    if !rem.is_zero() {
        return Err(Error::Invalid(
            "normalized discriminant/content identity".into(),
        ));
    }
    let mut unit = ring.polynomial(&Atom::num(rewrite.coefficient.clone()), b)?;
    for (target, factor) in owner.divisor_targets.iter().zip(&divisor_units) {
        if *target == DivisorTarget::Discriminant {
            let next = ring.polynomial(factor.representative(), b)?;
            if unit
                .nterms()
                .checked_mul(next.nterms())
                .is_none_or(|v| v > b.limits.max_terms)
                || (0..unit.nvars()).any(|i| {
                    unit.degree(i)
                        .checked_add(next.degree(i))
                        .is_none_or(|v| v as usize > b.limits.max_degree)
                })
            {
                return Err(Error::ResourceIncomplete(
                    "discriminant unit product allowance".into(),
                ));
            }
            unit = &unit * &next;
            algebra::check(&unit, b)?;
        }
    }
    let discriminant_unit = ring.expression(&unit);
    let monomial = units
        .iter()
        .zip(&discriminant_powers)
        .map(|(s, e)| Atom::var(*s).pow(*e))
        .product::<Atom>();
    let cross = &disc - &monomial * &discriminant_unit;
    let (remaining, discriminant_unit_quotients) =
        transport::reduce(&ring, &roles, &cross, &relations, b)?;
    if !remaining.is_zero() {
        return Err(Error::Unsupported(
            "normalized discriminant-unit identity not proved".into(),
        ));
    }
    let mut coefficients = vec![Atom::zero(); degree + 1];
    for (c, e) in q.to_univariate_polynomial_list(roles.len()) {
        coefficients[e as usize] = full_ring.expression(&c);
    }
    let equation = full_ring.expression(&q);
    let total = full_ring.expression(&total);
    b.check()?;
    Ok(Arc::new(RestrictedRootEquation {
        leaf,
        total,
        content,
        content_powers,
        equation,
        coefficients,
        relations,
        leading,
        divisor_units,
        discriminant_powers,
        discriminant_identity_quotients,
        discriminant_unit,
        discriminant_unit_quotients,
        constraints,
        domain,
    }))
}
