use super::{BracketProposal, Budget, Error, NormalizedPrefix, Result, transform, witness};
use std::{collections::BTreeSet, sync::Arc};
use symbolica::{
    atom::{Atom, Symbol},
    domains::rational::Rational,
};
use symgcad::{
    algebra::Poly,
    domain::{Domain, DomainCertificate},
    output::RootBound,
    roots::FiberRoot,
};

/// Proposed positive monomial coordinates for the actual source prefix.
/// `parameter_images` use only prior unit and admitted root slots. The source
/// chart must compose to the exact prior prefix and have nonsingular exponent
/// matrix. Its rational witness is independently checked in that native cell.
#[derive(Clone, Debug)]
pub struct CoefficientChart {
    pub parameters: Vec<Symbol>,
    pub bounds: Vec<[Rational; 2]>,
    pub source_prefix: Vec<Atom>,
    pub parameter_images: Vec<Atom>,
    pub witness: Vec<Rational>,
}
#[derive(Clone, Debug)]
pub struct SectionProposal {
    pub chart: CoefficientChart,
    pub root_symbol: Symbol,
    pub scale: Atom,
    /// Positive coefficient of the removed root-presentation monomial. Its
    /// powers are derived by native valuation, never supplied as authority.
    pub removed_coefficient: Rational,
    pub bracket: BracketProposal,
}

/// A new normalized equation, associated with one original upper selector.
/// The removed monomial is equation provenance; it is NEVER a density factor.
#[derive(Debug)]
pub struct NormalizedSection {
    pub(super) prefix: Arc<NormalizedPrefix>,
    pub(super) chart: CoefficientChart,
    pub(super) root_symbol: Symbol,
    pub(super) scale: Atom,
    pub(super) lower: Rational,
    pub(super) bracket: BracketProposal,
    pub(super) bound: RootBound,
    original: Poly,
    total: Atom,
    removed: Atom,
    pub(super) equation: Atom,
    pub(super) coefficients: Vec<Atom>,
    pub(super) orientation: i8,
    derivative: DomainCertificate,
    endpoints: [Option<DomainCertificate>; 2],
    range: Vec<Option<DomainCertificate>>,
    constraints: Vec<Poly>,
    domain: Domain,
    anchor: FiberRoot,
    determinant: Rational,
}
impl NormalizedSection {
    pub fn prefix(&self) -> &Arc<NormalizedPrefix> {
        &self.prefix
    }
    pub fn chart(&self) -> &CoefficientChart {
        &self.chart
    }
    pub fn root_symbol(&self) -> Symbol {
        self.root_symbol
    }
    pub fn scale(&self) -> &Atom {
        &self.scale
    }
    pub fn original_bound(&self) -> &RootBound {
        &self.bound
    }
    pub fn original_polynomial(&self) -> &Poly {
        &self.original
    }
    pub fn total_pullback(&self) -> &Atom {
        &self.total
    }
    pub fn removed_presentation_monomial(&self) -> &Atom {
        &self.removed
    }
    pub fn equation(&self) -> &Atom {
        &self.equation
    }
    pub fn coefficients(&self) -> &[Atom] {
        &self.coefficients
    }
    pub fn bracket(&self) -> &BracketProposal {
        &self.bracket
    }
    pub fn derivative_certificate(&self) -> &DomainCertificate {
        &self.derivative
    }
    pub fn endpoint_certificates(&self) -> &[Option<DomainCertificate>; 2] {
        &self.endpoints
    }
    pub fn range_certificates(&self) -> &[Option<DomainCertificate>] {
        &self.range
    }
    pub fn constraints(&self) -> &[Poly] {
        &self.constraints
    }
    pub fn domain(&self) -> &Domain {
        &self.domain
    }
    pub fn anchor(&self) -> &FiberRoot {
        &self.anchor
    }
    pub fn exponent_determinant(&self) -> &Rational {
        &self.determinant
    }
}

pub(super) fn prepare(
    prefix: Arc<NormalizedPrefix>,
    p: SectionProposal,
    b: &mut Budget,
) -> Result<Arc<NormalizedSection>> {
    b.check()?;
    let n = prefix.len();
    if n == 0 || prefix.complete_open_map() {
        return Err(Error::Unsupported(
            "normalized upper section requires a nonempty admitted prefix".into(),
        ));
    }
    let chart = &p.chart;
    if [
        chart.parameters.len(),
        chart.bounds.len(),
        chart.source_prefix.len(),
        chart.parameter_images.len(),
        chart.witness.len(),
    ]
    .iter()
    .any(|len| *len != n)
    {
        return Err(Error::Invalid(
            "complete coefficient chart inventory required".into(),
        ));
    }
    let mut used = prefix.source.decomposition().occupied_symbols().clone();
    used.extend(prefix.source.unit_coordinates());
    used.extend(prefix.slots().iter().map(|(s, _)| *s));
    let roles = chart
        .parameters
        .iter()
        .copied()
        .chain([p.root_symbol])
        .collect::<BTreeSet<_>>();
    if roles.len() != n + 1 || roles.iter().any(|s| used.contains(s)) {
        return Err(Error::Invalid(
            "fresh distinct coefficient/root roles required".into(),
        ));
    }
    for q in [
        &p.bracket.lower,
        &p.bracket.upper,
        &p.bracket.derivative_margin,
        &p.removed_coefficient,
    ] {
        b.rational(q)?;
    }
    if p.bracket.lower <= Rational::zero()
        || p.bracket.lower >= p.bracket.upper
        || p.bracket.derivative_margin <= Rational::zero()
        || p.removed_coefficient <= Rational::zero()
    {
        return Err(Error::Invalid(
            "positive normalized bracket, margin and presentation coefficient required".into(),
        ));
    }
    let (determinant, range) = transform::chart(&prefix, chart, b)?;
    let lower = witness::constant_bound(&prefix.source, n, false, b)?;
    let bound = prefix.source.native_axes()[n]
        .upper
        .clone()
        .ok_or_else(|| Error::Unsupported("normalized upper bound must be finite".into()))?;
    let original = witness::polynomial(&prefix.source, &bound, b)?;
    let (ring, total, removed, q) = transform::equation(&prefix, &p, &original, &lower, b)?;
    let degree = q.degree(n) as usize;
    if degree == 0 || degree > symgcad::roots::MAX_ROOT_DEGREE {
        return Err(Error::Unsupported(
            "normalized polynomial root degree".into(),
        ));
    }
    let mut bounds = chart.bounds.clone();
    bounds.push([p.bracket.lower.clone(), p.bracket.upper.clone()]);
    let (constraints, domain) = ring.box_domain(&bounds, b)?;
    let derivative = q.derivative(n);
    let mut admitted = None;
    for orientation in [1i8, -1] {
        let signed = &derivative * &derivative.constant(Rational::from(i64::from(orientation)));
        let margin = &signed - &signed.constant(p.bracket.derivative_margin.clone());
        match ring.positive(&margin, &constraints, &domain, b) {
            Ok(proof) => {
                admitted = Some((orientation, proof));
                break;
            }
            Err(Error::Unsupported(_)) => {}
            Err(e) => return Err(e),
        }
    }
    let (orientation, derivative) = admitted
        .ok_or_else(|| Error::Unsupported("normalized derivative margin not certified".into()))?;
    let mut endpoints = [None, None];
    for (i, value) in [&p.bracket.lower, &p.bracket.upper].into_iter().enumerate() {
        let at = q.replace(n, value);
        if !at.is_zero() {
            let sign = if i == 0 { -orientation } else { orientation };
            let signed = &at * &at.constant(Rational::from(i64::from(sign)));
            endpoints[i] = Some(ring.positive(&signed, &constraints, &domain, b)?);
        }
    }
    let anchor = witness::associate(&prefix, &p, &bound, &original, &lower, b)?;
    let equation = ring.expression(&q);
    let mut coefficients = vec![Atom::zero(); degree + 1];
    for (coefficient, power) in q.to_univariate_polynomial_list(n) {
        coefficients[power as usize] = ring.expression(&coefficient);
    }
    b.check()?;
    Ok(Arc::new(NormalizedSection {
        prefix,
        chart: p.chart,
        root_symbol: p.root_symbol,
        scale: p.scale,
        lower,
        bracket: p.bracket,
        bound,
        original,
        total: ring.expression(&total),
        removed: ring.expression(&removed),
        equation,
        coefficients,
        orientation,
        derivative,
        endpoints,
        range,
        constraints,
        domain,
        anchor,
        determinant,
    }))
}
