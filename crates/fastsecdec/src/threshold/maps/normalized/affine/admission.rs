use super::super::{BracketProposal, CoefficientChart, transform};
use super::{AffineNormalizedPrefix, Budget, Error, Result, SectionProposal, witness};
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

/// A new normalized equation, associated with one original upper selector.
/// The removed monomial is equation provenance; it is NEVER a density factor.
#[derive(Debug)]
pub struct AffineNormalizedSection {
    prefix: Arc<AffineNormalizedPrefix>,
    chart: CoefficientChart,
    root_symbol: Symbol,
    scale: Atom,
    lower: Rational,
    bracket: BracketProposal,
    bound: RootBound,
    original: Poly,
    total: Atom,
    removed: Atom,
    equation: Atom,
    coefficients: Vec<Atom>,
    orientation: i8,
    derivative: DomainCertificate,
    endpoints: [Option<DomainCertificate>; 2],
    range: Vec<Option<DomainCertificate>>,
    constraints: Vec<Poly>,
    domain: Domain,
    anchor: FiberRoot,
    determinant: Rational,
}
impl AffineNormalizedSection {
    pub fn prefix(&self) -> &Arc<AffineNormalizedPrefix> {
        &self.prefix
    }
    pub fn lower(&self) -> &Rational {
        &self.lower
    }
    pub fn orientation(&self) -> i8 {
        self.orientation
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
    prefix: Arc<AffineNormalizedPrefix>,
    p: SectionProposal,
    b: &mut Budget,
) -> Result<Arc<AffineNormalizedSection>> {
    b.check()?;
    let n = prefix.len();
    if n == 0 || prefix.len() == prefix.source().axes().len() {
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
    let used = prefix.occupied();
    // Non-unit slots are independent selected constants or prior normalized roots.
    // This sufficient monomial chart requires each to have a positive closed enclosure.
    if prefix
        .slots()
        .iter()
        .skip(n)
        .any(|(_, bounds)| bounds[0] <= Rational::zero())
    {
        return Err(Error::Unsupported(
            "positive closed prior root-slot bounds required for the monomial coefficient chart"
                .into(),
        ));
    }
    let roles = chart
        .parameters
        .iter()
        .copied()
        .chain([p.root_symbol])
        .collect::<BTreeSet<_>>();
    if roles.len() != n + 1 || roles.iter().any(|s| used.contains(s) || !s.is_exportable()) {
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
    let source = witness::Source::Affine(prefix.source());
    let (determinant, range) =
        transform::chart_in(source, &prefix.images(), &prefix.slots(), chart, b)?;
    let lower = witness::constant_bound_in(source, n, false, b)?;
    let bound = prefix.source().native_axes()[n]
        .upper
        .clone()
        .ok_or_else(|| Error::Unsupported("normalized upper bound must be finite".into()))?;
    let original = witness::polynomial_in(source, &bound, b)?;
    let (ring, total, removed, q) = transform::equation_in(source, n, &p, &original, &lower, b)?;
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
    let anchor = witness::associate_in(source, n, &p, &bound, &lower, b)?;
    let equation = ring.expression(&q);
    let mut coefficients = vec![Atom::zero(); degree + 1];
    for (coefficient, power) in q.to_univariate_polynomial_list(n) {
        coefficients[power as usize] = ring.expression(&coefficient);
    }
    b.check()?;
    Ok(Arc::new(AffineNormalizedSection {
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
