//! Complete three-cell partition between two separated native regular sections.
//! This sufficient issuer needs strict interior brackets and a closed positive
//! second secant; it does not resolve collisions or infer brackets from samples.
use super::*;
use crate::threshold::gcad::{AliasRole, DomainOrigin};
use regular::{BracketProposal, Budget, RegularSection, SectionSide};
use symbolica::id::{Pattern, Replacement};
use symgcad::{
    algebra::{Algebra, Poly},
    domain::{DomainCertificate, derive_domain},
};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PairedPart {
    LowerOuter,
    LowerMiddle,
    UpperMiddle,
    UpperOuter,
}
impl PairedPart {
    pub fn reversed(self) -> bool {
        matches!(self, Self::LowerOuter | Self::UpperMiddle)
    }
}
/// Shared exact polynomial and branch evidence. Its constructor is private.
pub struct SectionPair {
    sections: [Arc<RegularSection>; 2],
    algebra: Algebra,
    polynomial: Poly,
    second_secant: Poly,
    unit: DomainCertificate,
    gap: DomainCertificate,
    widths: [DomainCertificate; 3],
}
impl SectionPair {
    pub fn sections(&self) -> &[Arc<RegularSection>; 2] {
        &self.sections
    }
    pub fn polynomial(&self) -> &Poly {
        &self.polynomial
    }
    pub fn second_secant(&self) -> &Poly {
        &self.second_secant
    }
    pub fn unit_certificate(&self) -> &DomainCertificate {
        &self.unit
    }
    pub fn gap_certificate(&self) -> &DomainCertificate {
        &self.gap
    }
    pub fn width_certificates(&self) -> &[DomainCertificate; 3] {
        &self.widths
    }
}
pub struct PairedGeometry {
    source: Arc<CellMap>,
    pair: Arc<SectionPair>,
    part: PairedPart,
}
impl PairedGeometry {
    pub fn source(&self) -> &Arc<CellMap> {
        &self.source
    }
    pub fn pair(&self) -> &Arc<SectionPair> {
        &self.pair
    }
    pub fn part(&self) -> PairedPart {
        self.part
    }
    pub(super) fn normalized(&self, roots: &[Atom]) -> (Atom, Atom, Atom) {
        let a = &roots[0];
        let b = &roots[1];
        let t = Atom::var(*self.source.unit_coordinates().last().unwrap());
        let gap = b - a;
        let half = &gap / Atom::num(2);
        let (image, measure, other) = match self.part {
            PairedPart::LowerOuter => (a * (Atom::one() - &t), a.clone(), &gap + a * &t),
            PairedPart::LowerMiddle => (a + &half * &t, half.clone(), &gap - &half * &t),
            PairedPart::UpperMiddle => (b - &half * &t, half.clone(), &gap - &half * &t),
            PairedPart::UpperOuter => {
                let width = Atom::one() - b;
                (b + &width * &t, width.clone(), &gap + &width * &t)
            }
        };
        let n = self.source.axes().len();
        let q = self.pair.second_secant.to_expression().replace_multiple(
            self.pair.algebra.names.iter().enumerate().map(|(i, name)| {
                let value = if i == n {
                    a.clone()
                } else if i == n + 1 {
                    b.clone()
                } else if i == n - 1 {
                    image.clone()
                } else {
                    Atom::var(self.source.unit_coordinates()[i])
                };
                Replacement::new(
                    Pattern::Literal(Atom::var(Symbol::parse(name, "symgcad").unwrap())),
                    Pattern::Literal(value),
                )
            }),
        );
        let magnitude = &measure * other * q;
        (image, measure, magnitude)
    }
}

pub(super) fn admit(
    owner: &Arc<VerifiedDecomposition>,
    coordinates: &[Symbol],
    proposals: [BracketProposal; 2],
    limits: &Limits,
    budget: &mut Budget,
) -> Result<Vec<Arc<PairedGeometry>>> {
    budget.check()?;
    let request = owner.request();
    let n = request.domain().coordinates().len();
    if n == 0
        || !matches!(request.domain().origin(), DomainOrigin::NativeUnitCube)
        || !request.kinematics().runtime_parameters.is_empty()
    {
        return Err(unsupported("paired issuer requires a fixed native cube"));
    }
    if owner.cells().len() != 3 {
        return Err(unsupported(
            "paired issuer requires the complete three-cell inventory",
        ));
    }
    let [lo, hi] = &proposals;
    if lo.lower <= Rational::zero()
        || lo.upper >= hi.lower
        || hi.upper >= Rational::one()
        || limits.unit_margin <= Rational::zero()
    {
        return Err(invalid(
            "paired brackets must be disjoint and strictly inside the unit interval",
        ));
    }
    let mut outer_lower = None;
    let mut middle = None;
    let mut outer_upper = None;
    for cell in owner.cells() {
        let map =
            CellMap::new(owner.clone(), cell.index(), coordinates.to_vec()).map_err(native)?;
        if map
            .axes()
            .iter()
            .any(|a| a.role() != AliasRole::IntegrationCoordinate)
        {
            return Err(unsupported("paired nonintegration axis"));
        }
        // Metadata currently binds section axes in original input order. Decline
        // other valid lifting orders until that projection is generalized.
        if !map.axes().iter().map(|axis| axis.symbol()).eq(request
            .domain()
            .coordinates()
            .iter()
            .copied())
        {
            return Err(unsupported(
                "paired issuer requires original integration-axis order",
            ));
        }
        for axis in 0..n - 1 {
            if geometry::constant_bound(&map, axis, SectionSide::Lower)? != Some(Rational::zero())
                || geometry::constant_bound(&map, axis, SectionSide::Upper)?
                    != Some(Rational::one())
            {
                return Err(unsupported(
                    "paired prefix requires additional coverage partition",
                ));
            }
        }
        let l = geometry::constant_bound(&map, n - 1, SectionSide::Lower)?;
        let u = geometry::constant_bound(&map, n - 1, SectionSide::Upper)?;
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
    let axis = n - 1;
    if lower.native_axes()[axis].upper != middle.native_axes()[axis].lower
        || middle.native_axes()[axis].upper != upper.native_axes()[axis].lower
    {
        return Err(invalid("paired neighbor branch association"));
    }
    let [lo, hi] = proposals;
    let a = RegularSection::prepare(middle.clone(), axis, SectionSide::Lower, lo, budget)?;
    let b = RegularSection::prepare(middle.clone(), axis, SectionSide::Upper, hi, budget)?;
    if a.polynomial() != b.polynomial() || a.bound() == b.bound() {
        return Err(unsupported(
            "paired sections need distinct roots of the same source polynomial",
        ));
    }
    let raw = owner.native_result();
    let mut names = raw.order.clone();
    for name in ["fastsecdec_pair_lower", "fastsecdec_pair_upper"] {
        if names.iter().any(|n| n == name) {
            return Err(invalid("paired formal polynomial variable collision"));
        }
        names.push(name.into());
    }
    let algebra = Algebra::new(&names).map_err(native)?;
    let polynomial = algebra
        .parse(&raw.polynomials[a.bound().polynomial])
        .map_err(native)?;
    if polynomial
        .nterms()
        .checked_mul(polynomial.degree(axis) as usize)
        .and_then(|v| v.checked_mul(polynomial.degree(axis) as usize))
        .is_none_or(|v| v > limits.geometry.max_terms)
    {
        return Err(Error::ResourceIncomplete(
            "paired divided-difference support bound".into(),
        ));
    }
    let y = algebra.parse(&names[axis]).map_err(native)?;
    let ra = algebra.parse(&names[n]).map_err(native)?;
    let rb = algebra.parse(&names[n + 1]).map_err(native)?;
    let pa = polynomial.replace_with_poly(axis, &ra);
    let pb = polynomial.replace_with_poly(axis, &rb);
    let ya = &y - &ra;
    let yb = &y - &rb;
    let gap = &rb - &ra;
    let (first, remainder) = (&polynomial - &pa).quot_rem(&ya, false);
    if !remainder.is_zero() {
        return Err(invalid("native first divided difference"));
    }
    let (second, remainder) = (&first - &first.replace_with_poly(axis, &rb)).quot_rem(&yb, false);
    if !remainder.is_zero()
        || &gap * &(&polynomial - &(&(&ya * &yb) * &second)) != &(&(&rb - &y) * &pa) + &(&ya * &pb)
    {
        return Err(invalid("native second divided-difference recombination"));
    }
    let mut constraints = raw
        .normalized_constraints
        .iter()
        .map(|p| algebra.parse(p).map_err(native))
        .collect::<Result<Vec<_>>>()?;
    for (r, section) in [(&ra, &a), (&rb, &b)] {
        constraints.push(r - &r.constant(section.bracket().lower.clone()));
        constraints.push(&r.constant(section.bracket().upper.clone()) - r);
    }
    let domain = derive_domain(&algebra, &constraints, &[]).map_err(native)?;
    let certify =
        |target: &Poly| geometry::positive(&algebra, target, &constraints, &domain, limits, budget);
    let unit = certify(&(&second - &second.constant(limits.unit_margin.clone())))?;
    let gap_certificate = certify(&(&gap - &gap.constant(limits.unit_margin.clone())))?;
    let lower_margin = &a.bracket().lower / &Rational::from(2);
    let middle_margin = (&b.bracket().lower - &a.bracket().upper) / Rational::from(4);
    let upper_margin = (Rational::one() - &b.bracket().upper) / Rational::from(2);
    let widths = [
        certify(&(&ra - &ra.constant(lower_margin)))?,
        certify(&(&gap * &gap.constant(Rational::from((1, 2))) - gap.constant(middle_margin)))?,
        certify(&(&rb.constant(Rational::one()) - &rb - rb.constant(upper_margin)))?,
    ];
    let pair = Arc::new(SectionPair {
        sections: [a, b],
        algebra,
        polynomial,
        second_secant: second,
        unit,
        gap: gap_certificate,
        widths,
    });
    Ok([
        (lower, PairedPart::LowerOuter),
        (middle.clone(), PairedPart::LowerMiddle),
        (middle, PairedPart::UpperMiddle),
        (upper, PairedPart::UpperOuter),
    ]
    .into_iter()
    .map(|(source, part)| {
        Arc::new(PairedGeometry {
            source,
            pair: pair.clone(),
            part,
        })
    })
    .collect())
}
