use super::*;
use fastsecdec_sectors::{
    DecompositionPhase, ParametricDomain, PolynomialSupport, SectorMap, decompose,
};
use std::{collections::BTreeSet, ops::ControlFlow};
use symbolica::{domains::atom::AtomField, tensors::matrix::Matrix};
use symgcad::{algebra::Poly, output::RootIndexDomain};

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum DivisorTarget {
    Leading,
    Discriminant,
}
#[derive(Clone)]
pub struct FactoredDivisorProposal {
    pub target: DivisorTarget,
    pub coefficient: Rational,
    pub factors: Vec<Atom>,
}
pub struct DivisorRewrite {
    pub target: DivisorTarget,
    pub coefficient: Rational,
    pub original: Atom,
    pub product: Atom,
    pub quotients: Vec<Atom>,
}
impl DivisorRewrite {
    pub fn target(&self) -> DivisorTarget {
        self.target
    }
    pub fn coefficient(&self) -> &Rational {
        &self.coefficient
    }
    pub fn original(&self) -> &Atom {
        &self.original
    }
    pub fn product(&self) -> &Atom {
        &self.product
    }
    pub fn quotients(&self) -> &[Atom] {
        &self.quotients
    }
}
pub struct TransportedDivisor {
    pub(super) total: Atom,
    pub(super) powers: Vec<u32>,
    pub(super) residual: Atom,
}
impl TransportedDivisor {
    pub fn total(&self) -> &Atom {
        &self.total
    }
    pub fn powers(&self) -> &[u32] {
        &self.powers
    }
    pub fn residual(&self) -> &Atom {
        &self.residual
    }
}
pub struct AffinePrefixRestriction {
    half: usize,
    sector: usize,
    map: SectorMap,
    images: Vec<Atom>,
    signed: Atom,
    measure: Atom,
    orientation: i8,
    divisors: Vec<TransportedDivisor>,
}
impl AffinePrefixRestriction {
    pub fn half(&self) -> usize {
        self.half
    }
    pub fn sector(&self) -> usize {
        self.sector
    }
    pub fn native_map(&self) -> &SectorMap {
        &self.map
    }
    pub fn unit_images(&self) -> &[Atom] {
        &self.images
    }
    pub fn signed_jacobian(&self) -> &Atom {
        &self.signed
    }
    pub fn measure(&self) -> &Atom {
        &self.measure
    }
    pub fn orientation(&self) -> i8 {
        self.orientation
    }
    pub fn divisors(&self) -> &[TransportedDivisor] {
        &self.divisors
    }
}

pub(super) fn discriminant(p: &Poly, axis: usize, b: &Budget) -> Result<Poly> {
    algebra::check(p, b)?;
    let u = p.to_univariate(axis);
    let d = u.degree();
    if d == 0 {
        return Err(Error::Invalid("positive next-root degree required".into()));
    }
    let leading = u.lcoeff();
    let resultant = u.resultant(&u.derivative());
    algebra::check(&resultant, b)?;
    let mut result = resultant
        .try_div(&leading)
        .ok_or_else(|| Error::Invalid("native discriminant exact leading division".into()))?;
    if !(d / 2).is_multiple_of(2) {
        result = -result;
    }
    algebra::check(&result, b)?;
    Ok(result)
}

/// Sufficient native polynomial-remainder equality, never a new CAS or a claim
/// that nonzero remainder proves inequality in the selected field.
pub(super) fn reduce(
    ring: &algebra::Ring,
    roles: &[Symbol],
    body: &Atom,
    relations: &[(Symbol, Atom)],
    b: &Budget,
) -> Result<(Atom, Vec<Atom>)> {
    let original = ring.polynomial(body, b)?;
    let mut rem = original.clone();
    let mut recombined = original.zero();
    let mut quotients = vec![Atom::zero(); relations.len()];
    for (i, (dependent, equation)) in relations.iter().enumerate().rev() {
        b.check()?;
        let axis = roles
            .iter()
            .position(|s| s == dependent)
            .ok_or_else(|| Error::Invalid("owned relation role absent".into()))?;
        let relation = ring.polynomial(equation, b)?;
        if relation.degree(axis) == 0 {
            return Err(Error::Invalid("owned dependent equation degree".into()));
        }
        let before = rem.to_univariate(axis);
        let divisor = relation.to_univariate(axis);
        let (q, r) = before.quot_rem(&divisor);
        if &q * &divisor + r.clone() != before {
            return Err(Error::Invalid(
                "native tower remainder recombination".into(),
            ));
        }
        // Native quotient coefficients may drop unused variable roles (in
        // particular constant quotients/remainders). flatten() requires its
        // outer variable in coefficient zero's map. Native Horner composition
        // followed by native role unification retains the exact original frame.
        let variable = relation
            .variable(&ring.native.variables[axis])
            .map_err(native)?;
        let mut q = q.evaluate(&variable);
        rem = r.evaluate(&variable);
        let mut frame = relation.constant(Rational::zero());
        frame.unify_variables(&mut q);
        frame.unify_variables(&mut rem);
        algebra::check(&q, b)?;
        algebra::check(&rem, b)?;
        // Preserve the complete identity in the native polynomial frame. A
        // factored Atom sum can trip the AST expansion preflight even though
        // all original/quotient/remainder polynomials are already bounded.
        let product = &q * &relation;
        algebra::check(&product, b)?;
        recombined = &recombined + &product;
        algebra::check(&recombined, b)?;
        quotients[i] = ring.expression(&q);
    }
    let expression = ring.expression(&rem);
    let rebuilt = &rem + &recombined;
    algebra::check(&rebuilt, b)?;
    if rebuilt == original {
        Ok((expression, quotients))
    } else {
        Err(Error::Invalid("complete tower remainder identity".into()))
    }
}

pub(super) fn prepare(
    parent: Arc<AffineNormalizedPrefix>,
    side: SectionSide,
    proposals: &[FactoredDivisorProposal],
    options: &DecompositionOptions,
    b: &mut Budget,
) -> Result<Arc<AffinePrefixCover>> {
    b.check()?;
    let n = parent.len();
    if n == 0 || n >= parent.source().axes().len() {
        return Err(Error::Unsupported(
            "nonempty proper affine prefix required".into(),
        ));
    }
    let half_count = u32::try_from(n)
        .ok()
        .and_then(|n| 1usize.checked_shl(n))
        .ok_or_else(|| Error::ResourceIncomplete("prefix half count overflow".into()))?;
    if half_count > options.max_sectors || half_count > b.limits.max_terms {
        return Err(Error::ResourceIncomplete(
            "complete prefix half inventory allowance".into(),
        ));
    }
    let source = parent.source();
    let bound = match side {
        SectionSide::Lower => &source.native_axes()[n].lower,
        SectionSide::Upper => &source.native_axes()[n].upper,
    }
    .clone()
    .ok_or_else(|| Error::Unsupported("finite next selected bound required".into()))?;
    if bound.index_domain != RootIndexDomain::Real {
        return Err(Error::Unsupported(
            "distinct-real next selector required".into(),
        ));
    }
    let original_poly = super::super::witness::polynomial_in(
        super::super::witness::Source::Affine(source),
        &bound,
        b,
    )?;
    let raw = source.decomposition().native_result();
    if original_poly.degree(n) == 0 || (n + 1..raw.order.len()).any(|i| original_poly.degree(i) > 0)
    {
        return Err(Error::Invalid(
            "next selected equation triangular support".into(),
        ));
    }
    let aliases = raw
        .order
        .iter()
        .map(|s| Symbol::parse(s, "symgcad").map_err(native))
        .collect::<Result<Vec<_>>>()?;
    let root = source.axes()[n].symbol();
    let slots = parent.slots();
    let roles = slots.iter().map(|(s, _)| *s).collect::<Vec<_>>();
    if roles.contains(&root) {
        return Err(Error::Invalid(
            "next source root collides with prefix slots".into(),
        ));
    }
    let original = algebra::pull(
        &original_poly.to_expression(),
        &aliases,
        &source
            .axes()
            .iter()
            .map(|a| Atom::var(a.symbol()))
            .collect::<Vec<_>>(),
    );
    let mut images = parent.images();
    images.push(Atom::var(root));
    let pulled = algebra::pull(&original_poly.to_expression(), &aliases[..=n], &images);
    let mut equation_roles = roles.clone();
    equation_roles.push(root);
    let equation_ring = algebra::Ring::new(equation_roles)?;
    let full = equation_ring.polynomial(&pulled, b)?;
    let leading = equation_ring.expression(&full.to_univariate(roles.len()).lcoeff());
    let disc = equation_ring.expression(&discriminant(&full, roles.len(), b)?);
    if disc.is_zero() {
        return Err(Error::Unsupported(
            "identically repeated next-root equation".into(),
        ));
    }
    let ring = algebra::Ring::new(roles.clone())?;
    let relations = relation_inventory(&parent, b)?;
    let relation_rows = relations
        .iter()
        .map(|r| (r.dependent, r.equation.clone()))
        .collect::<Vec<_>>();
    let mut seen = BTreeSet::new();
    for p in proposals {
        if !seen.insert(p.target)
            || p.factors.is_empty()
            || p.factors.len() > b.limits.max_terms
            || p.coefficient.is_zero()
        {
            return Err(Error::Invalid(
                "complete nonzero distinct divisor proposal".into(),
            ));
        }
        b.rational(&p.coefficient)?;
    }
    let (mut divisors, mut divisor_targets, mut rewrites) = (Vec::new(), Vec::new(), Vec::new());
    for (target, actual) in [
        (DivisorTarget::Leading, leading),
        (DivisorTarget::Discriminant, disc),
    ] {
        let (coefficient, factors) = if let Some(p) = proposals.iter().find(|p| p.target == target)
        {
            (p.coefficient.clone(), p.factors.clone())
        } else {
            (Rational::one(), vec![actual.clone()])
        };
        let product = Atom::num(coefficient.clone()) * factors.iter().cloned().product::<Atom>();
        let (rem, quotients) = reduce(&ring, &roles, &(&actual - &product), &relation_rows, b)?;
        if !rem.is_zero() {
            return Err(Error::Unsupported(
                "proposed divisor identity not proved in owned tower".into(),
            ));
        }
        // The nonzero rational coefficient remains in the receipt; it does not
        // affect the native support or create/discard a vanishing divisor.
        for factor in factors {
            if ring.polynomial(&factor, b)?.is_zero() {
                return Err(Error::Invalid("identically zero divisor proposal".into()));
            }
            divisor_targets.push(target);
            divisors.push(factor);
        }
        rewrites.push(DivisorRewrite {
            target,
            coefficient,
            original: actual,
            product,
            quotients,
        });
    }
    let (half_counts, leaves) = fan(&ring, &roles[..n], &divisors, options, b)?;
    if !complete_inventory(n, &half_counts, &leaves) {
        return Err(Error::Invalid(
            "complete source-owned prefix inventory".into(),
        ));
    }
    let dependent_symbols = slots.iter().skip(n).map(|(s, _)| *s).collect();
    Ok(Arc::new(AffinePrefixCover {
        parent,
        side,
        bound,
        root,
        original,
        pulled,
        slots,
        dependent_symbols,
        relations,
        divisors,
        divisor_targets,
        rewrites,
        half_counts,
        leaves,
    }))
}

pub(super) fn complete_inventory(
    n: usize,
    counts: &[usize],
    leaves: &[AffinePrefixRestriction],
) -> bool {
    if u32::try_from(n).ok().and_then(|n| 1usize.checked_shl(n)) != Some(counts.len()) {
        return false;
    }
    let mut index = 0;
    for (half, &count) in counts.iter().enumerate() {
        if count == 0 {
            return false;
        }
        for sector in 0..count {
            let Some(leaf) = leaves.get(index) else {
                return false;
            };
            if leaf.half != half || leaf.sector != sector {
                return false;
            }
            index += 1;
        }
    }
    index == leaves.len()
}

fn fan(
    ring: &algebra::Ring,
    units: &[Symbol],
    divisors: &[Atom],
    options: &DecompositionOptions,
    b: &Budget,
) -> Result<(Vec<usize>, Vec<AffinePrefixRestriction>)> {
    let n = units.len();
    let identity = units.iter().copied().map(Atom::var).collect::<Vec<_>>();
    let original = divisors
        .iter()
        .map(|p| ring.polynomial(p, b))
        .collect::<Result<Vec<_>>>()?;
    let mut counts = Vec::new();
    let mut leaves = Vec::new();
    for half in 0..1usize
        .checked_shl(u32::try_from(n).map_err(native)?)
        .ok_or_else(|| Error::ResourceIncomplete("half count".into()))?
    {
        b.check()?;
        let h = identity
            .iter()
            .enumerate()
            .map(|(i, u)| {
                if half & (1 << i) == 0 {
                    u / Atom::num(2)
                } else {
                    Atom::one() - u / Atom::num(2)
                }
            })
            .collect::<Vec<_>>();
        // Native sparse polynomial reflection avoids the exponential AST bound
        // of (1-u/2)^d. Each reflected monomial produces at most e+1 terms.
        let half_polynomials = original
            .iter()
            .map(|p| {
                let terms = p
                    .exponents_iter()
                    .try_fold(0usize, |sum, es| {
                        let term = es[..n].iter().enumerate().try_fold(1usize, |v, (i, e)| {
                            if half & (1 << i) == 0 {
                                Some(v)
                            } else {
                                v.checked_mul(usize::try_from(*e).ok()?.checked_add(1)?)
                            }
                        })?;
                        sum.checked_add(term)
                    })
                    .ok_or_else(|| {
                        Error::ResourceIncomplete("native sparse reflection term overflow".into())
                    })?;
                if terms > b.limits.max_terms {
                    return Err(Error::ResourceIncomplete(
                        "native sparse reflection term allowance".into(),
                    ));
                }
                let mut result = p.clone();
                for (i, image) in h.iter().enumerate() {
                    b.check()?;
                    result = result.replace_with_poly(i, &ring.polynomial(image, b)?);
                    algebra::check(&result, b)?;
                }
                Ok(result)
            })
            .collect::<Result<Vec<_>>>()?;
        let supports = half_polynomials
            .iter()
            .map(|p| {
                PolynomialSupport::new(
                    p.exponents_iter()
                        .map(|es| es[..n].iter().map(|e| i64::from(*e)).collect())
                        .collect(),
                )
                .map_err(native)
            })
            .collect::<Result<Vec<_>>>()?;
        let mut complete = false;
        let result = decompose(ParametricDomain::UnitCube, &supports, options, |p| {
            if p.phase == DecompositionPhase::Complete {
                complete = true;
            }
            if b.check().is_ok() {
                ControlFlow::Continue(())
            } else {
                ControlFlow::Break(())
            }
        });
        b.check()?;
        let result = result.map_err(native)?;
        if !complete || result.sectors.is_empty() {
            return Err(Error::Invalid("complete native fan absent".into()));
        }
        if leaves
            .len()
            .checked_add(result.sectors.len())
            .is_none_or(|v| v > options.max_sectors)
        {
            return Err(Error::ResourceIncomplete(
                "whole prefix sector allowance".into(),
            ));
        }
        counts.push(result.sectors.len());
        for (sector, map) in result.sectors.into_iter().enumerate() {
            b.check()?;
            map.validate(ParametricDomain::UnitCube).map_err(native)?;
            if map.dimension() != n
                || map.source_dimension() != n
                || map.fixed_parameter.is_some()
                || map.factor_valuations.len() != divisors.len()
            {
                return Err(Error::Invalid("native prefix fan role inventory".into()));
            }
            let mon = crate::generation::coordinates_from_parts(
                units,
                ParametricDomain::UnitCube,
                &map,
                units,
            );
            let images = h
                .iter()
                .map(|a| algebra::pull(a, units, mon.images()))
                .collect::<Vec<_>>();
            let measure = Atom::num(map.determinant.clone())
                * map
                    .jacobian_powers
                    .iter()
                    .zip(&identity)
                    .map(|(e, u)| u.pow(Atom::num(e.clone())))
                    .product::<Atom>()
                / Atom::num(2).pow(n as i64);
            let size = u32::try_from(n).map_err(native)?;
            let signed = Matrix::from_linear(
                images
                    .iter()
                    .flat_map(|p| units.iter().map(|u| p.derivative(*u)))
                    .collect(),
                size,
                size,
                AtomField {
                    statistical_zero_test: false,
                    ..Default::default()
                },
            )
            .map_err(native)?
            .det()
            .map_err(native)?;
            let orientation = if ring.polynomial(&(&signed - &measure), b)?.is_zero() {
                1
            } else if ring.polynomial(&(&signed + &measure), b)?.is_zero() {
                -1
            } else {
                return Err(Error::Invalid("native signed prefix determinant".into()));
            };
            let mut points = mon
                .images()
                .iter()
                .map(|a| ring.polynomial(a, b))
                .collect::<Result<Vec<_>>>()?;
            for i in n..ring.native.variables.len() {
                points.push(ring.native.parse(&format!("n{i}")).map_err(native)?);
            }
            if points.iter().any(|p| p.nterms() != 1) {
                return Err(Error::Invalid(
                    "native fan substitution must be monomial".into(),
                ));
            }
            let mut factors = Vec::new();
            for (i, p) in half_polynomials.iter().enumerate() {
                // Simultaneous native substitution preserves mixed-coordinate
                // maps; sequential role substitution would be incorrect.
                for es in p.exponents_iter() {
                    for target in 0..p.nvars() {
                        let degree = es
                            .iter()
                            .zip(&points)
                            .try_fold(0usize, |sum, (e, q)| {
                                sum.checked_add(
                                    usize::try_from(*e)
                                        .ok()?
                                        .checked_mul(usize::try_from(q.degree(target)).ok()?)?,
                                )
                            })
                            .ok_or_else(|| {
                                Error::ResourceIncomplete("native fan degree overflow".into())
                            })?;
                        if degree > b.limits.max_degree || u32::try_from(degree).is_err() {
                            return Err(Error::ResourceIncomplete(
                                "native fan degree allowance".into(),
                            ));
                        }
                    }
                }
                b.check()?;
                let total = p.evaluate_with_coeff_map(
                    |q| p.constant(q.clone()),
                    &points,
                    &symbolica::poly::polynomial::PolynomialRing::new(
                        symbolica::domains::rational::Q,
                    ),
                );
                algebra::check(&total, b)?;
                if total.nterms() > p.nterms() {
                    return Err(Error::Invalid(
                        "native monomial substitution term bound".into(),
                    ));
                }
                if total.is_zero() {
                    return Err(Error::Invalid(
                        "transported divisor identically zero".into(),
                    ));
                }
                let powers = (0..n)
                    .map(|axis| total.degree_bounds(axis).0)
                    .collect::<Vec<_>>();
                if powers.len() != map.factor_valuations[i].len()
                    || powers
                        .iter()
                        .zip(&map.factor_valuations[i])
                        .any(|(e, v)| *v != *e)
                {
                    return Err(Error::Invalid("native divisor support valuation".into()));
                }
                let content = ring.polynomial(
                    &identity
                        .iter()
                        .zip(&powers)
                        .map(|(u, e)| u.pow(*e))
                        .product::<Atom>(),
                    b,
                )?;
                let residual = total.try_div(&content).ok_or_else(|| {
                    Error::Invalid("native exact divisor monomial quotient".into())
                })?;
                if &content * &residual != total {
                    return Err(Error::Invalid("native divisor recombination".into()));
                }
                factors.push(TransportedDivisor {
                    total: ring.expression(&total),
                    powers,
                    residual: ring.expression(&residual),
                });
            }
            leaves.push(AffinePrefixRestriction {
                half,
                sector,
                map,
                images,
                signed,
                measure,
                orientation,
                divisors: factors,
            });
        }
    }
    Ok((counts, leaves))
}
