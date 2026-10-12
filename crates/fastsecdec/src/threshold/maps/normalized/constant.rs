//! Constant sections retaining the genuine positive-projective source owner.
use super::{
    Atom, Budget, Error, PositiveProjectiveCellMap, Rational, Result, Symbol, constant_pair, native,
};
use crate::threshold::maps::regular::{
    BracketProposal, SectionSide,
    certificate::{Geometry, ProposalError, SectionCertificate},
};
use std::sync::Arc;
use symgcad::{
    algebra::Poly,
    domain::{Domain, DomainCertificate},
    output::RootBound,
    roots::FiberRoot,
};
#[derive(Debug)]
pub struct PositiveConstantSection {
    source: Arc<PositiveProjectiveCellMap>,
    certificate: Arc<SectionCertificate>,
}
impl PositiveConstantSection {
    pub fn prepare(
        source: Arc<PositiveProjectiveCellMap>,
        side: SectionSide,
        bracket: BracketProposal,
        b: &mut Budget,
    ) -> Result<Arc<Self>> {
        Self::prepare_proposal(source, side, bracket, b)
            .map_err(|e| e.into_error(|s| Error::Unsupported(s.into())))
    }
    fn prepare_proposal(
        source: Arc<PositiveProjectiveCellMap>,
        side: SectionSide,
        bracket: BracketProposal,
        b: &mut Budget,
    ) -> std::result::Result<Arc<Self>, ProposalError<Error>> {
        let certificate =
            SectionCertificate::prepare_proposal(Geometry::positive(&source), 0, side, bracket, b)?;
        Ok(Arc::new(Self {
            source,
            certificate,
        }))
    }
    pub fn source(&self) -> &Arc<PositiveProjectiveCellMap> {
        &self.source
    }
    pub fn axis(&self) -> usize {
        self.certificate.axis()
    }
    pub fn side(&self) -> SectionSide {
        self.certificate.side()
    }
    pub fn bound(&self) -> &RootBound {
        self.certificate.bound()
    }
    pub fn polynomial(&self) -> &Poly {
        self.certificate.polynomial()
    }
    pub fn equation(&self) -> &Atom {
        self.certificate.equation()
    }
    pub fn coefficients(&self) -> &[Atom] {
        self.certificate.coefficients()
    }
    pub fn original_constraints(&self) -> &[Poly] {
        self.certificate.original_constraints()
    }
    pub fn original_domain(&self) -> &Domain {
        self.certificate.original_domain()
    }
    pub fn cylinder_constraints(&self) -> &[Poly] {
        self.certificate.cylinder_constraints()
    }
    pub fn cylinder(&self) -> &Domain {
        self.certificate.cylinder()
    }
    pub fn bracket(&self) -> &BracketProposal {
        self.certificate.bracket()
    }
    pub fn orientation(&self) -> i8 {
        self.certificate.orientation()
    }
    pub fn derivative_certificate(&self) -> &DomainCertificate {
        self.certificate.derivative_certificate()
    }
    pub fn endpoint_certificates(&self) -> &[Option<DomainCertificate>; 2] {
        self.certificate.endpoint_certificates()
    }
    pub fn anchor(&self) -> &FiberRoot {
        self.certificate.anchor()
    }
    pub(in crate::threshold::maps) fn certificate(&self) -> &SectionCertificate {
        &self.certificate
    }
}
#[derive(Debug)]
enum Value {
    Rational(Rational),
    Root {
        section: Arc<PositiveConstantSection>,
        symbol: Symbol,
    },
}
/// One exact original first-axis endpoint, with selected-root identity retained.
#[derive(Debug)]
pub struct ConstantEndpoint {
    bound: RootBound,
    value: Value,
}
impl ConstantEndpoint {
    pub fn original_bound(&self) -> &RootBound {
        &self.bound
    }
    pub fn expression(&self) -> Atom {
        match &self.value {
            Value::Rational(q) => Atom::num(q.clone()),
            Value::Root { symbol, .. } => Atom::var(*symbol),
        }
    }
    pub fn rational(&self) -> Option<&Rational> {
        match &self.value {
            Value::Rational(q) => Some(q),
            _ => None,
        }
    }
    pub fn symbol(&self) -> Option<Symbol> {
        match &self.value {
            Value::Root { symbol, .. } => Some(*symbol),
            _ => None,
        }
    }
    pub fn section(&self) -> Option<&Arc<PositiveConstantSection>> {
        match &self.value {
            Value::Root { section, .. } => Some(section),
            _ => None,
        }
    }
    pub fn enclosure(&self) -> [Rational; 2] {
        match &self.value {
            Value::Rational(q) => [q.clone(), q.clone()],
            Value::Root { section, .. } => [
                section.bracket().lower.clone(),
                section.bracket().upper.clone(),
            ],
        }
    }
}
fn prepare(
    source: &Arc<PositiveProjectiveCellMap>,
    side: SectionSide,
    bound: RootBound,
    root: FiberRoot,
    b: &mut Budget,
) -> Result<ConstantEndpoint> {
    let value = constant_pair::admit(root, b, |bracket, b| {
        PositiveConstantSection::prepare_proposal(source.clone(), side, bracket, b)
    })?;
    let value = match value {
        constant_pair::Admitted::Rational(q) => Value::Rational(q),
        constant_pair::Admitted::Root(section) => {
            let identity = super::program::source_digest(source)?;
            let occupied = source.decomposition().occupied_symbols();
            let mut selected = None;
            for attempt in 0..b.limits.max_refinements {
                b.check()?;
                let symbol = Symbol::parse(
                    format!(
                        "constant_{identity}_{}_{attempt}",
                        if matches!(side, SectionSide::Lower) {
                            "lower"
                        } else {
                            "upper"
                        }
                    ),
                    "fastsecdec_positive",
                )
                .map_err(native)?;
                if symbol.is_exportable()
                    && !occupied.contains(&symbol)
                    && !source.unit_coordinates().contains(&symbol)
                {
                    selected = Some(symbol);
                    break;
                }
            }
            let symbol = selected.ok_or_else(|| {
                Error::ResourceIncomplete("constant slot freshness allowance".into())
            })?;
            Value::Root { section, symbol }
        }
    };
    Ok(ConstantEndpoint { bound, value })
}
pub(super) fn prepare_pair(
    source: &Arc<PositiveProjectiveCellMap>,
    b: &mut Budget,
) -> Result<(ConstantEndpoint, ConstantEndpoint)> {
    let (lower, lo, upper, hi) =
        constant_pair::prepare(super::witness::Source::Positive(source), b)?;
    Ok((
        prepare(source, SectionSide::Lower, lower, lo, b)?,
        prepare(source, SectionSide::Upper, upper, hi, b)?,
    ))
}

#[cfg(test)]
mod tests;
