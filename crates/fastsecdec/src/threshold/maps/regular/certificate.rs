use super::{Budget, Error, Result, native};
use crate::threshold::maps::CellMap;
use std::sync::Arc;
use symbolica::{
    atom::{Atom, AtomCore, Symbol},
    domains::rational::Rational,
    id::{Pattern, Replacement},
};
use symgcad::{
    algebra::{Algebra, Poly, rational},
    domain::{Domain, DomainCertificate, certify_nonzero, derive_domain, verify_certificate},
    output::{RootBound, RootIndexDomain},
    roots::{FiberRoot, isolate_union, refine_once},
};

/// Only explicit certificate/branch proposal misses may be retried. Other
/// errors retain their owner type, including unsupported immutable input.
#[derive(Debug)]
pub(crate) enum ProposalError<E> {
    Miss(&'static str),
    Failure(E),
}
impl<E> From<E> for ProposalError<E> {
    fn from(error: E) -> Self {
        Self::Failure(error)
    }
}
impl<E> ProposalError<E> {
    pub(crate) fn into_error(self, unsupported: impl FnOnce(&'static str) -> E) -> E {
        match self {
            Self::Miss(reason) => unsupported(reason),
            Self::Failure(error) => error,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SectionSide {
    Lower,
    Upper,
}
#[derive(Clone, Debug)]
pub struct BracketProposal {
    pub lower: Rational,
    pub upper: Rational,
    pub derivative_margin: Rational,
}

/// Immutable exact admission tied to one actual native cell section. It proves
/// one regular branch on a box closure, not a cell width/unit or finite atlas.
#[derive(Debug)]
pub struct RegularSection {
    source: Arc<CellMap>,
    axis: usize,
    side: SectionSide,
    bound: RootBound,
    polynomial: Poly,
    equation: Atom,
    coefficients: Vec<Atom>,
    original_constraints: Vec<Poly>,
    original_domain: Domain,
    cylinder_constraints: Vec<Poly>,
    cylinder: Domain,
    bracket: BracketProposal,
    orientation: i8,
    derivative: DomainCertificate,
    endpoints: [Option<DomainCertificate>; 2],
    anchor: FiberRoot,
}
impl RegularSection {
    pub fn prepare(
        source: Arc<CellMap>,
        axis: usize,
        side: SectionSide,
        bracket: BracketProposal,
        b: &mut Budget,
    ) -> Result<Arc<Self>> {
        Self::prepare_proposal(source, axis, side, bracket, b)
            .map_err(|error| error.into_error(|reason| Error::Unsupported(reason.into())))
    }
    pub(crate) fn prepare_proposal(
        source: Arc<CellMap>,
        axis: usize,
        side: SectionSide,
        bracket: BracketProposal,
        b: &mut Budget,
    ) -> std::result::Result<Arc<Self>, ProposalError<Error>> {
        b.check()?;
        for value in [&bracket.lower, &bracket.upper, &bracket.derivative_margin] {
            b.rational(value)?;
        }
        if bracket.lower >= bracket.upper || bracket.derivative_margin <= Rational::zero() {
            return Err(Error::Invalid(
                "ordered bracket and strictly positive derivative margin required".into(),
            )
            .into());
        }
        let raw = source.decomposition().native_result();
        let axis_record = source
            .native_axes()
            .get(axis)
            .ok_or_else(|| Error::Invalid("section axis".into()))?;
        let bound = match side {
            SectionSide::Lower => &axis_record.lower,
            SectionSide::Upper => &axis_record.upper,
        }
        .clone()
        .ok_or_else(|| Error::Unsupported("unbounded section needs compactification".into()))?;
        let algebra = Algebra::new(&raw.order).map_err(native)?;
        let polynomial = algebra
            .parse(
                raw.polynomials
                    .get(bound.polynomial)
                    .ok_or_else(|| Error::Invalid("native polynomial index".into()))?,
            )
            .map_err(native)?;
        let degree = polynomial.degree(axis) as usize;
        if degree == 0 || (axis + 1..raw.order.len()).any(|i| polynomial.degree(i) > 0) {
            return Err(Error::Invalid(
                "root equation must have positive degree and triangular support".into(),
            )
            .into());
        }
        if (0..raw.order.len()).any(|i| polynomial.degree(i) as usize > b.limits.max_degree)
            || degree > symgcad::roots::MAX_ROOT_DEGREE
            || polynomial.nterms() > b.limits.max_terms
        {
            return Err(Error::ResourceIncomplete(
                "section polynomial degree/term allowance".into(),
            )
            .into());
        }
        let original_constraints = raw
            .normalized_constraints
            .iter()
            .map(|p| algebra.parse(p).map_err(native))
            .collect::<Result<Vec<_>>>()?;
        // Native derive_domain inspects only ONE-variable affine original
        // constraints. The explicit consequence check below prevents current/
        // later variables or any target sign from narrowing prefix bounds.
        let original_domain =
            derive_domain(&algebra, &original_constraints, &[]).map_err(native)?;
        if original_domain.empty {
            return Err(Error::Invalid("empty original domain".into()).into());
        }
        let mut cylinder_constraints = Vec::new();
        for i in 0..axis {
            let d = &original_domain.bounds[i];
            if polynomial.degree(i) > 0 && (d.lower.is_none() || d.upper.is_none()) {
                return Err(Error::Unsupported(
                    "used prefix coordinate lacks finite original affine bounds".into(),
                )
                .into());
            }
            for (endpoint, index, lower) in [
                (&d.lower, d.lower_constraint, true),
                (&d.upper, d.upper_constraint, false),
            ] {
                if let Some(endpoint) = endpoint {
                    let constraint = &original_constraints[index.ok_or_else(|| {
                        Error::Invalid("missing original bound association".into())
                    })?];
                    if constraint.degree(i) != 1
                        || (0..raw.order.len()).any(|j| j != i && constraint.degree(j) > 0)
                    {
                        return Err(Error::Invalid(
                            "prefix bound is not an original single-coordinate consequence".into(),
                        )
                        .into());
                    }
                    let endpoint = rational(endpoint).map_err(native)?;
                    b.rational(&endpoint)?;
                    let coordinate = algebra.parse(&raw.order[i]).map_err(native)?;
                    let bound_poly = if lower {
                        &coordinate - &coordinate.constant(endpoint)
                    } else {
                        &coordinate.constant(endpoint) - &coordinate
                    };
                    // Positive proportionality proves exactly the same strict
                    // half-space, without a separate canonicalizer.
                    let mut exponent = vec![0; raw.order.len()];
                    exponent[i] = 1;
                    let slope = constraint.coefficient(&exponent).unwrap();
                    let positive = if lower { slope.clone() } else { -slope.clone() };
                    if positive <= Rational::zero()
                        || constraint != &(&bound_poly * &bound_poly.constant(positive))
                    {
                        return Err(Error::Invalid(
                            "prefix half-space reconstruction failed".into(),
                        )
                        .into());
                    }
                    cylinder_constraints.push(bound_poly);
                }
            }
        }
        let z = algebra.parse(&raw.order[axis]).map_err(native)?;
        cylinder_constraints.push(&z - &z.constant(bracket.lower.clone()));
        cylinder_constraints.push(&z.constant(bracket.upper.clone()) - &z);
        let cylinder = derive_domain(&algebra, &cylinder_constraints, &[]).map_err(native)?;
        let derivative_poly = polynomial.derivative(axis);
        let mut admission = None;
        for orientation in [1i8, -1] {
            b.check()?;
            let signed = &derivative_poly
                * &derivative_poly.constant(Rational::from(i64::from(orientation)));
            let margin = &signed - &signed.constant(bracket.derivative_margin.clone());
            if let Some(c) = positive(&algebra, &margin, &cylinder_constraints, &cylinder, b)? {
                admission = Some((orientation, c));
                break;
            }
        }
        let (orientation, derivative) = admission.ok_or(ProposalError::Miss(
            "no native uniform derivative-margin certificate",
        ))?;
        let mut endpoints = [None, None];
        for (slot, value) in [bracket.lower.clone(), bracket.upper.clone()]
            .into_iter()
            .enumerate()
        {
            b.check()?;
            let endpoint = polynomial.replace(axis, &value);
            if endpoint.is_zero() {
                continue;
            }
            let sign = if slot == 0 { -orientation } else { orientation };
            let target = &endpoint * &endpoint.constant(Rational::from(i64::from(sign)));
            endpoints[slot] = Some(
                positive(&algebra, &target, &cylinder_constraints, &cylinder, b)?.ok_or(
                    ProposalError::Miss("no native uniform endpoint-sign certificate"),
                )?,
            );
        }
        let prefix = source.native_axes()[..axis]
            .iter()
            .map(|a| rational(&a.sample).map_err(native))
            .collect::<Result<Vec<_>>>()?;
        let fiber = algebra
            .specialize_univariate(&polynomial, axis, &prefix)
            .map_err(native)?;
        b.check()?;
        let mut roots = isolate_union(vec![(bound.polynomial, fiber)]).map_err(native)?;
        if bound.index_domain == RootIndexDomain::Positive {
            for root in &mut roots {
                let mut steps = 0;
                while root.interval.0 < Rational::zero() && root.interval.1 > Rational::zero() {
                    b.check()?;
                    steps += 1;
                    if steps > b.limits.max_refinements {
                        return Err(Error::ResourceIncomplete(
                            "root sign isolation allowance".into(),
                        )
                        .into());
                    }
                    refine_once(root);
                }
            }
            roots.retain(|root| {
                root.interval.0 > Rational::zero()
                    || root.interval.1 > Rational::zero() && root.interval.0 == Rational::zero()
            });
        } else if bound.index_domain == RootIndexDomain::RealDescending {
            roots.reverse();
        }
        let mut anchor = roots
            .get(bound.index)
            .cloned()
            .ok_or_else(|| Error::Invalid("native selected root is absent".into()))?;
        let mut steps = 0;
        loop {
            b.check()?;
            if anchor.interval.0 >= bracket.lower && anchor.interval.1 <= bracket.upper {
                break;
            }
            if anchor.interval.1 < bracket.lower || anchor.interval.0 > bracket.upper {
                return Err(ProposalError::Miss(
                    "certificate bracket selects a different native root",
                ));
            }
            // A bracket endpoint can itself be the unique rational root.
            if anchor.fiber.evaluate(&bracket.lower).is_zero()
                && anchor.interval.0 <= bracket.lower
                && anchor.interval.1 >= bracket.lower
            {
                anchor.interval = (bracket.lower.clone(), bracket.lower.clone());
                break;
            }
            if anchor.fiber.evaluate(&bracket.upper).is_zero()
                && anchor.interval.0 <= bracket.upper
                && anchor.interval.1 >= bracket.upper
            {
                anchor.interval = (bracket.upper.clone(), bracket.upper.clone());
                break;
            }
            steps += 1;
            if steps > b.limits.max_refinements {
                return Err(
                    Error::ResourceIncomplete("root/bracket association allowance".into()).into(),
                );
            }
            refine_once(&mut anchor);
        }
        let replace = |a: Atom| {
            a.replace_multiple(raw.order.iter().zip(source.axes()).map(|(name, axis)| {
                Replacement::new(
                    Pattern::Literal(Atom::var(Symbol::parse(name, "symgcad").unwrap())),
                    Pattern::Literal(Atom::var(axis.symbol())),
                )
            }))
        };
        let equation = replace(polynomial.to_expression());
        let mut coefficients = vec![Atom::zero(); degree + 1];
        for (p, power) in polynomial.to_univariate_polynomial_list(axis) {
            coefficients[power as usize] = replace(p.to_expression());
        }
        b.check()?;
        Ok(Arc::new(Self {
            source,
            axis,
            side,
            bound,
            polynomial,
            equation,
            coefficients,
            original_constraints,
            original_domain,
            cylinder_constraints,
            cylinder,
            bracket,
            orientation,
            derivative,
            endpoints,
            anchor,
        }))
    }
    pub fn source(&self) -> &Arc<CellMap> {
        &self.source
    }
    pub fn axis(&self) -> usize {
        self.axis
    }
    pub fn side(&self) -> SectionSide {
        self.side
    }
    pub fn bound(&self) -> &RootBound {
        &self.bound
    }
    pub fn polynomial(&self) -> &Poly {
        &self.polynomial
    }
    pub fn equation(&self) -> &Atom {
        &self.equation
    }
    pub fn coefficients(&self) -> &[Atom] {
        &self.coefficients
    }
    pub fn original_constraints(&self) -> &[Poly] {
        &self.original_constraints
    }
    pub fn original_domain(&self) -> &Domain {
        &self.original_domain
    }
    pub fn cylinder_constraints(&self) -> &[Poly] {
        &self.cylinder_constraints
    }
    pub fn cylinder(&self) -> &Domain {
        &self.cylinder
    }
    pub fn bracket(&self) -> &BracketProposal {
        &self.bracket
    }
    pub fn orientation(&self) -> i8 {
        self.orientation
    }
    pub fn derivative_certificate(&self) -> &DomainCertificate {
        &self.derivative
    }
    pub fn endpoint_certificates(&self) -> &[Option<DomainCertificate>; 2] {
        &self.endpoints
    }
    pub fn anchor(&self) -> &FiberRoot {
        &self.anchor
    }
}
pub(in crate::threshold::maps) fn positive(
    algebra: &Algebra,
    p: &Poly,
    constraints: &[Poly],
    domain: &Domain,
    b: &Budget,
) -> Result<Option<DomainCertificate>> {
    b.check()?;
    let answer = certify_nonzero(algebra, p, domain, &b.limits.certificates).map_err(native)?;
    if let Some(c) = answer {
        verify_certificate(algebra, p, constraints, &[], &c).map_err(native)?;
        b.check()?;
        Ok((c.sign == 1).then_some(c))
    } else {
        Ok(None)
    }
}
