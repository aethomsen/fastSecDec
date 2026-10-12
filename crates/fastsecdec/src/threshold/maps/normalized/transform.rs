use super::{
    Budget, Error, NormalizedPrefix, Result,
    algebra::{self, Ring},
    certificate::{CoefficientChart, SectionProposal},
    native,
    witness::Source,
};
use symbolica::{
    atom::{Atom, AtomCore, Symbol},
    domains::rational::Rational,
    prelude::Q,
    tensors::matrix::Matrix,
};
use symgcad::{algebra::Poly, domain::DomainCertificate};

pub(super) fn chart(
    prefix: &NormalizedPrefix,
    c: &CoefficientChart,
    b: &Budget,
) -> Result<(Rational, Vec<Option<DomainCertificate>>)> {
    chart_in(
        Source::Positive(prefix.source()),
        &prefix.images(),
        &prefix.slots(),
        c,
        b,
    )
}
pub(super) fn chart_in(
    source: Source<'_>,
    images: &[Atom],
    slots: &[(Symbol, [Rational; 2])],
    c: &CoefficientChart,
    b: &Budget,
) -> Result<(Rational, Vec<Option<DomainCertificate>>)> {
    let n = images.len();
    if n > source.axes().len()
        || c.parameters.len() != n
        || c.source_prefix.len() != n
        || c.parameter_images.len() != n
        || c.bounds.len() != n
    {
        return Err(Error::Invalid(
            "complete accepted chart prefix required".into(),
        ));
    }
    let ring = Ring::new(c.parameters.clone())?;
    let mut matrix = Vec::with_capacity(n * n);
    for image in &c.source_prefix {
        matrix.extend(
            algebra::monomial(&ring.polynomial(image, b)?)?
                .into_iter()
                .map(Rational::from),
        );
    }
    let det = Matrix::from_linear(matrix, n as u32, n as u32, Q)
        .map_err(native)?
        .det()
        .map_err(native)?;
    if det.is_zero() {
        return Err(Error::Unsupported(
            "singular positive monomial coefficient chart".into(),
        ));
    }
    let slot_symbols = slots.iter().map(|(s, _)| *s).collect::<Vec<_>>();
    let slot_ring = Ring::new(slot_symbols)?;
    let (constraints, domain) =
        slot_ring.box_domain(&slots.iter().map(|(_, b)| b.clone()).collect::<Vec<_>>(), b)?;
    let mut range = Vec::with_capacity(2 * n);
    for (image, [lo, hi]) in c.parameter_images.iter().zip(&c.bounds) {
        b.rational(lo)?;
        b.rational(hi)?;
        if *lo < Rational::zero() || lo >= hi {
            return Err(Error::Unsupported(
                "positive coefficient parameter box required".into(),
            ));
        }
        // Units are open positive and all previous normalized roots have a
        // positive closed lower bound. Thus the monomial is strictly positive
        // even when a root is exactly on its proposed bracket boundary.
        let polynomial = slot_ring.polynomial(image, b)?;
        algebra::monomial(&polynomial)?;
        for target in [image - Atom::num(lo.clone()), Atom::num(hi.clone()) - image] {
            let p = slot_ring.polynomial(&target, b)?;
            range.push(if p.is_zero() {
                None
            } else {
                Some(slot_ring.positive(&p, &constraints, &domain, b)?)
            });
        }
    }
    for (source, current) in c.source_prefix.iter().zip(images) {
        let pulled = algebra::pull(source, &c.parameters, &c.parameter_images);
        if slot_ring.polynomial(&pulled, b)? != slot_ring.polynomial(current, b)? {
            return Err(Error::Invalid(
                "coefficient chart does not equal the accepted source prefix".into(),
            ));
        }
    }
    Ok((det, range))
}

pub(super) fn equation(
    prefix: &NormalizedPrefix,
    p: &SectionProposal,
    original: &Poly,
    lower: &Rational,
    b: &Budget,
) -> Result<(Ring, Poly, Poly, Poly)> {
    equation_in(
        Source::Positive(prefix.source()),
        prefix.len(),
        p,
        original,
        lower,
        b,
    )
}
pub(super) fn equation_in(
    source: Source<'_>,
    n: usize,
    p: &SectionProposal,
    original: &Poly,
    lower: &Rational,
    b: &Budget,
) -> Result<(Ring, Poly, Poly, Poly)> {
    let mut roles = p.chart.parameters.clone();
    roles.push(p.root_symbol);
    let ring = Ring::new(roles)?;
    let scale = ring.polynomial(&p.scale, b)?;
    let powers = algebra::monomial(&scale)?;
    if powers[n] != 0 {
        return Err(Error::Invalid("root scale depends on root".into()));
    }
    let raw = source.raw();
    if original.degree(n) == 0 || (n + 1..raw.order.len()).any(|i| original.degree(i) > 0) {
        return Err(Error::Invalid("original bound is not triangular".into()));
    }
    let variables = raw
        .order
        .iter()
        .map(|name| Symbol::parse(name, "symgcad").map_err(native))
        .collect::<Result<Vec<_>>>()?;
    let mut values = p.chart.source_prefix.clone();
    values.push(Atom::num(lower.clone()) + &p.scale * Atom::var(p.root_symbol));
    let total_atom = algebra::pull(&original.to_expression(), &variables[..=n], &values);
    let total = ring.polynomial(&total_atom, b)?;
    if total.is_zero() {
        return Err(Error::Invalid("zero normalized root pullback".into()));
    }
    let (removed, q) = presentation_quotient(
        &ring,
        &total,
        &p.chart.parameters,
        &p.removed_coefficient,
        b,
    )?;
    Ok((ring, total, removed, q))
}

pub(super) fn presentation_quotient(
    ring: &Ring,
    total: &Poly,
    parameters: &[Symbol],
    coefficient: &Rational,
    b: &Budget,
) -> Result<(Poly, Poly)> {
    let mut monomial = Atom::num(coefficient.clone());
    for (i, s) in parameters.iter().enumerate() {
        // Native ldegree is the leading MONOMIAL's exponent, not a valuation.
        // The minimum must range over every original term.
        monomial *= Atom::var(*s).pow(total.degree_bounds(i).0);
    }
    let removed = ring.polynomial(&monomial, b)?;
    let (q, remainder) = total.quot_rem(&removed, false);
    algebra::check(&q, b)?;
    if !remainder.is_zero() || &q * &removed != *total {
        return Err(Error::Invalid(
            "native normalized quotient reconstruction failed".into(),
        ));
    }
    Ok((removed, q))
}
