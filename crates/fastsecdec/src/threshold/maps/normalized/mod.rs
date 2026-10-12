//! Checked open prefix maps and normalized selected sections. These owners
//! retain the original CAD cell; they do not certify any physical closed face.
pub mod affine;
pub(crate) mod algebra;
mod certificate;
pub mod constant;
mod constant_pair;
pub mod program;
#[cfg(test)]
pub(crate) mod tests;
mod transform;
mod witness;

pub use super::regular::{BracketProposal, Budget, Error, Limits, Result};
use super::{OpenProjectiveAxis, PositiveProjectiveCellMap};
pub use certificate::{CoefficientChart, NormalizedSection, SectionProposal};
use std::sync::Arc;
use symbolica::{
    atom::{Atom, AtomCore, Symbol},
    domains::rational::Rational,
};

#[derive(Debug)]
pub enum PrefixStep {
    ConstantInterval {
        lower: Box<constant::ConstantEndpoint>,
        upper: Box<constant::ConstantEndpoint>,
        power: u32,
        image: Atom,
        diagonal: Atom,
    },
    Power {
        lower: Rational,
        upper: Rational,
        power: u32,
        image: Atom,
        diagonal: Atom,
    },
    Section {
        section: Arc<NormalizedSection>,
        image: Atom,
        diagonal: Atom,
    },
    Open {
        axis: Arc<OpenProjectiveAxis>,
        image: Atom,
        diagonal: Atom,
    },
}
impl PrefixStep {
    pub fn image(&self) -> &Atom {
        match self {
            Self::ConstantInterval { image, .. }
            | Self::Power { image, .. }
            | Self::Section { image, .. }
            | Self::Open { image, .. } => image,
        }
    }
    pub fn diagonal(&self) -> &Atom {
        match self {
            Self::ConstantInterval { diagonal, .. }
            | Self::Power { diagonal, .. }
            | Self::Section { diagonal, .. }
            | Self::Open { diagonal, .. } => diagonal,
        }
    }
}

/// An immutable induction: exactly `len` original axes have an orientation
/// preserving open bijection from their unit prefix. Geometry and accepted
/// ancestors are shared, not copied with the large native proof.
#[derive(Debug)]
pub struct NormalizedPrefix {
    source: Arc<PositiveProjectiveCellMap>,
    previous: Option<Arc<Self>>,
    step: Option<PrefixStep>,
    len: usize,
}
impl NormalizedPrefix {
    pub fn new(source: Arc<PositiveProjectiveCellMap>, b: &Budget) -> Result<Arc<Self>> {
        b.check()?;
        if source.axes().is_empty()
            || source.axes().len() > 64
            || source.axes().iter().any(|a| a.unit_index().is_none())
        {
            return Err(Error::Unsupported(
                "normalized prefix needs 1..64 genuine integration axes".into(),
            ));
        }
        Ok(Arc::new(Self {
            source,
            previous: None,
            step: None,
            len: 0,
        }))
    }
    pub fn source(&self) -> &Arc<PositiveProjectiveCellMap> {
        &self.source
    }
    pub fn len(&self) -> usize {
        self.len
    }
    pub fn is_empty(&self) -> bool {
        self.len == 0
    }
    pub fn complete_open_map(&self) -> bool {
        self.len == self.source.axes().len()
    }
    pub fn previous(&self) -> Option<&Arc<Self>> {
        self.previous.as_ref()
    }
    pub fn step(&self) -> Option<&PrefixStep> {
        self.step.as_ref()
    }
    pub fn steps(&self) -> Vec<&PrefixStep> {
        let mut out = Vec::with_capacity(self.len);
        let mut p = self;
        while let Some(previous) = &p.previous {
            out.push(p.step.as_ref().expect("issued step"));
            p = previous;
        }
        out.reverse();
        out
    }
    pub fn images(&self) -> Vec<Atom> {
        self.steps()
            .into_iter()
            .map(|s| s.image().clone())
            .collect()
    }
    pub fn diagonal_product(&self) -> Atom {
        self.steps()
            .into_iter()
            .map(|s| s.diagonal().clone())
            .product()
    }
    pub(crate) fn slots(&self) -> Vec<(Symbol, [Rational; 2])> {
        let mut v = self.source.unit_coordinates()[..self.len]
            .iter()
            .map(|s| (*s, [Rational::zero(), Rational::one()]))
            .collect::<Vec<_>>();
        for step in self.steps() {
            if let PrefixStep::ConstantInterval { lower, upper, .. } = step {
                for endpoint in [lower, upper] {
                    if let Some(symbol) = endpoint.symbol() {
                        v.push((symbol, endpoint.enclosure()));
                    }
                }
            }
            if let PrefixStep::Section { section, .. } = step {
                v.push((
                    section.root_symbol,
                    [section.bracket.lower.clone(), section.bracket.upper.clone()],
                ));
            }
        }
        v
    }
    fn next(self: &Arc<Self>, step: PrefixStep, b: &Budget) -> Result<Arc<Self>> {
        b.check()?;
        if self.complete_open_map() {
            return Err(Error::Invalid("all original axes already mapped".into()));
        }
        Ok(Arc::new(Self {
            source: self.source.clone(),
            previous: Some(self.clone()),
            step: Some(step),
            len: self.len + 1,
        }))
    }
    /// Map the complete original first interval, including arbitrary-degree
    /// certified constant endpoints. No endpoint is a caller-owned parameter.
    pub fn constant_interval(self: &Arc<Self>, power: u32, b: &mut Budget) -> Result<Arc<Self>> {
        if !self.is_empty() {
            return Err(Error::Unsupported(
                "constant interval starts the original positive prefix".into(),
            ));
        }
        if power == 0 || power as usize > b.limits.max_degree {
            return Err(Error::ResourceIncomplete(
                "positive constant-interval power allowance".into(),
            ));
        }
        let (lower, upper) = constant::prepare_pair(&self.source, b)?;
        let u = Atom::var(self.source.unit_coordinates()[0]);
        let width = upper.expression() - lower.expression();
        let image = lower.expression() + &width * u.pow(power);
        let diagonal = Atom::num(power) * width * u.pow(power - 1);
        self.next(
            PrefixStep::ConstantInterval {
                lower: Box::new(lower),
                upper: Box::new(upper),
                power,
                image,
                diagonal,
            },
            b,
        )
    }
    /// A genuine constant source interval, with its actual bounds retained in
    /// the cell owner. Positive integer powers preserve its full open coverage.
    pub fn power(self: &Arc<Self>, power: u32, b: &Budget) -> Result<Arc<Self>> {
        if power == 0 || power as usize > b.limits.max_degree {
            return Err(Error::ResourceIncomplete(
                "positive interval power allowance".into(),
            ));
        }
        let lower = witness::constant_bound(&self.source, self.len, false, b)?;
        let upper = witness::constant_bound(&self.source, self.len, true, b)?;
        if lower >= upper {
            return Err(Error::Invalid("constant source interval ordering".into()));
        }
        let u = Atom::var(self.source.unit_coordinates()[self.len]);
        let width = &upper - &lower;
        let image = Atom::num(lower.clone()) + Atom::num(width.clone()) * u.clone().pow(power);
        let diagonal = Atom::num(Rational::from(power) * width) * u.pow(power - 1);
        self.next(
            PrefixStep::Power {
                lower,
                upper,
                power,
                image,
                diagonal,
            },
            b,
        )
    }
    pub fn section(
        self: &Arc<Self>,
        proposal: SectionProposal,
        b: &mut Budget,
    ) -> Result<Arc<Self>> {
        let section = certificate::prepare(self.clone(), proposal, b)?;
        let scale = algebra::pull(
            &section.scale,
            &section.chart.parameters,
            &section.chart.parameter_images,
        );
        let width = scale * Atom::var(section.root_symbol);
        let image = Atom::num(section.lower.clone())
            + &width * Atom::var(self.source.unit_coordinates()[self.len]);
        self.next(
            PrefixStep::Section {
                section,
                image,
                diagonal: width,
            },
            b,
        )
    }
    /// Existing exact interval issuance is composed only after the entire
    /// preceding prefix has been admitted. Missing faces remain explicit rays.
    pub fn open(self: &Arc<Self>, b: &Budget) -> Result<Arc<Self>> {
        b.check()?;
        let axis = self.source.open_linear_axis(self.len).map_err(native)?;
        let variables = self.source.axes()[..self.len]
            .iter()
            .map(|a| a.symbol())
            .collect::<Vec<_>>();
        let images = self.images();
        let image = algebra::pull(axis.image(), &variables, &images);
        let diagonal = algebra::pull(axis.diagonal_jacobian(), &variables, &images);
        self.next(
            PrefixStep::Open {
                axis,
                image,
                diagonal,
            },
            b,
        )
    }
}
fn native(e: impl std::fmt::Display) -> Error {
    Error::Native(e.to_string())
}
