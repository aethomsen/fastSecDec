//! Complete three-cell partition between two separated native regular sections.
//! This sufficient issuer needs strict interior brackets and a closed positive
//! second secant; it does not resolve collisions or infer brackets from samples.
use super::*;
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
                    let axis = &self.source.axes()[i];
                    Atom::var(
                        axis.unit_index()
                            .map_or(axis.symbol(), |slot| self.source.unit_coordinates()[slot]),
                    )
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

#[path = "paired/topology.rs"]
mod topology;

/// Shared uniform shape/neighbor admission; no endpoint proof is issued.
pub(super) fn topology_uniform(
    owner: &Arc<VerifiedDecomposition>,
    coordinates: &[Symbol],
) -> Result<[Arc<CellMap>; 3]> {
    topology::uniform(owner, coordinates)
}

/// Derive the complete fixed native-cube partition before trying a bracket.
pub(super) fn topology(
    owner: &Arc<VerifiedDecomposition>,
    coordinates: &[Symbol],
) -> Result<[Arc<CellMap>; 3]> {
    topology::fixed(owner, coordinates)
}

pub(super) fn admit(
    owner: &Arc<VerifiedDecomposition>,
    coordinates: &[Symbol],
    proposals: [BracketProposal; 2],
    limits: &Limits,
    budget: &mut Budget,
) -> ProposalResult<Vec<Arc<PairedGeometry>>> {
    check_proposals(&proposals, limits, budget)?;
    let maps = topology(owner, coordinates)?;
    admit_maps(owner, maps, proposals, limits, budget)
}

pub(super) fn admit_uniform(
    owner: &Arc<VerifiedDecomposition>,
    coordinates: &[Symbol],
    proposals: [BracketProposal; 2],
    limits: &Limits,
    budget: &mut Budget,
) -> ProposalResult<Vec<Arc<PairedGeometry>>> {
    check_proposals(&proposals, limits, budget)?;
    let maps = topology::uniform(owner, coordinates)?;
    admit_maps(owner, maps, proposals, limits, budget)
}

fn check_proposals(
    proposals: &[BracketProposal; 2],
    limits: &Limits,
    budget: &Budget,
) -> ProposalResult<()> {
    budget.check()?;
    let [lo, hi] = proposals;
    if lo.lower <= Rational::zero()
        || lo.upper >= hi.lower
        || hi.upper >= Rational::one()
        || limits.unit_margin <= Rational::zero()
    {
        return Err(invalid(
            "paired brackets must be disjoint and strictly inside the unit interval",
        )
        .into());
    }
    Ok(())
}

fn admit_maps(
    owner: &Arc<VerifiedDecomposition>,
    maps: [Arc<CellMap>; 3],
    proposals: [BracketProposal; 2],
    limits: &Limits,
    budget: &mut Budget,
) -> ProposalResult<Vec<Arc<PairedGeometry>>> {
    let [lower, middle, upper] = maps;
    let n = middle.axes().len();
    let axis = n - 1;
    let [lo, hi] = proposals;
    let a = RegularSection::prepare_proposal(middle.clone(), axis, SectionSide::Lower, lo, budget)?;
    let b = RegularSection::prepare_proposal(middle.clone(), axis, SectionSide::Upper, hi, budget)?;
    if a.polynomial() != b.polynomial() || a.bound() == b.bound() {
        return Err(unsupported(
            "paired sections need distinct roots of the same source polynomial",
        )
        .into());
    }
    let raw = owner.native_result();
    let mut names = raw.order.clone();
    for name in ["fastsecdec_pair_lower", "fastsecdec_pair_upper"] {
        if names.iter().any(|n| n == name) {
            return Err(invalid("paired formal polynomial variable collision").into());
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
        return Err(
            Error::ResourceIncomplete("paired divided-difference support bound".into()).into(),
        );
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
        return Err(invalid("native first divided difference").into());
    }
    let (second, remainder) = (&first - &first.replace_with_poly(axis, &rb)).quot_rem(&yb, false);
    if !remainder.is_zero()
        || &gap * &(&polynomial - &(&(&ya * &yb) * &second)) != &(&(&rb - &y) * &pa) + &(&ya * &pb)
    {
        return Err(invalid("native second divided-difference recombination").into());
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
    let certify = |target: &Poly| {
        geometry::positive_proposal(&algebra, target, &constraints, &domain, limits, budget)
    };
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
