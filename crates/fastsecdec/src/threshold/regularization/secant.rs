//! Sufficient regular-section endpoint issuer. No artifact authority.
#[path = "secant/continuation.rs"]
mod continuation;
#[path = "secant/density.rs"]
mod density;
#[path = "secant/geometry.rs"]
mod geometry;

pub use super::ConvergenceStrip;
use super::meromorphic;
use crate::threshold::maps::regular;
use crate::{
    generation::GenerationOptions,
    threshold::{gcad::VerifiedDecomposition, maps::CellMap},
};
pub use continuation::ContinuedFamily;
pub use geometry::{SecantGeometry, WidthNormalForm};
use std::{collections::BTreeSet, ops::ControlFlow, sync::Arc};
use symbolica::{
    atom::{Atom, AtomCore, Symbol},
    domains::rational::Rational,
    evaluate::FunctionMap,
};
#[derive(Debug)]
pub enum Error {
    Invalid(String),
    Unsupported(String),
    ResourceIncomplete(String),
    Cancelled,
    Native(String),
}
impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{self:?}")
    }
}
impl std::error::Error for Error {}
pub type Result<T> = std::result::Result<T, Error>;
fn native(e: impl std::fmt::Display) -> Error {
    Error::Native(e.to_string())
}
fn invalid(s: impl Into<String>) -> Error {
    Error::Invalid(s.into())
}
fn unsupported(s: impl Into<String>) -> Error {
    Error::Unsupported(s.into())
}
impl From<regular::Error> for Error {
    fn from(e: regular::Error) -> Self {
        match e {
            regular::Error::ResourceIncomplete(s) => Self::ResourceIncomplete(s),
            regular::Error::Cancelled => Self::Cancelled,
            regular::Error::Unsupported(s) => Self::Unsupported(s),
            regular::Error::Invalid(s) => Self::Invalid(s),
            regular::Error::Native(s) => Self::Native(s),
        }
    }
}
fn meromorphic_error(e: meromorphic::Error) -> Error {
    match e {
        meromorphic::Error::Cancelled => Error::Cancelled,
        meromorphic::Error::ResourceIncomplete(s) => Error::ResourceIncomplete(s.into()),
        meromorphic::Error::Unsupported(s) => unsupported(s),
        meromorphic::Error::Invalid(s) => invalid(s),
        meromorphic::Error::Native(s) => Error::Native(s),
    }
}
#[derive(Clone, Debug)]
pub struct Limits {
    pub geometry: regular::Limits,
    pub density: meromorphic::Limits,
    pub unit_margin: Rational,
    pub cancellation: Option<Arc<std::sync::atomic::AtomicBool>>,
}
impl Default for Limits {
    fn default() -> Self {
        Self {
            geometry: regular::Limits::default(),
            density: meromorphic::Limits::default(),
            unit_margin: Rational::from((1, 2)),
            cancellation: None,
        }
    }
}
#[derive(Clone, Debug)]
pub struct FactorLedger {
    term: usize,
    factor: usize,
    source: Atom,
    exponent: Atom,
    kind: FactorKind,
}
#[derive(Clone, Debug)]
pub enum FactorKind {
    Section {
        scale: Rational,
    },
    ClosedUnit {
        certificate: Arc<symgcad::domain::DomainCertificate>,
    },
    PolynomialNumerator,
}
impl FactorLedger {
    pub fn source(&self) -> &Atom {
        &self.source
    }
    pub fn exponent(&self) -> &Atom {
        &self.exponent
    }
    pub fn indices(&self) -> (usize, usize) {
        (self.term, self.factor)
    }
    pub fn kind(&self) -> &FactorKind {
        &self.kind
    }
}
pub struct NormalizedTerm {
    prefactor: Atom,
    regular: Atom,
    powers: Vec<Atom>,
    ledger: Vec<FactorLedger>,
    term: usize,
}
impl NormalizedTerm {
    pub fn prefactor(&self) -> &Atom {
        &self.prefactor
    }
    pub fn regular(&self) -> &Atom {
        &self.regular
    }
    pub fn powers(&self) -> &[Atom] {
        &self.powers
    }
    pub fn ledger(&self) -> &[FactorLedger] {
        &self.ledger
    }
    pub fn source_term(&self) -> usize {
        self.term
    }
}
pub struct NormalizedChart {
    geometry: Arc<SecantGeometry>,
    image: Atom,
    measure: Atom,
    terms: Vec<NormalizedTerm>,
}
impl NormalizedChart {
    pub fn geometry(&self) -> &Arc<SecantGeometry> {
        &self.geometry
    }
    pub fn image(&self) -> &Atom {
        &self.image
    }
    pub fn positive_measure(&self) -> &Atom {
        &self.measure
    }
    pub fn terms(&self) -> &[NormalizedTerm] {
        &self.terms
    }
}
pub struct NormalizedFamily {
    owner: Arc<VerifiedDecomposition>,
    coordinates: Vec<Symbol>,
    regulators: Vec<Symbol>,
    charts: Vec<NormalizedChart>,
    strip: ConvergenceStrip,
    witness: meromorphic::MeromorphicWitness,
    root: Atom,
    scope: regular::callback::Scope,
    numerator: Symbol,
    bodies: Vec<Atom>,
    formals: Vec<Symbol>,
}
impl NormalizedFamily {
    pub fn admit(
        owner: Arc<VerifiedDecomposition>,
        coordinates: Vec<Symbol>,
        limits: Limits,
    ) -> Result<Self> {
        density::admit(owner, coordinates, limits, false)
    }
    /// Issue the same checked family with an explicit normalized static-bracket
    /// root recipe. This changes numerical arithmetic, not geometry or measure.
    pub fn admit_affine(
        owner: Arc<VerifiedDecomposition>,
        coordinates: Vec<Symbol>,
        limits: Limits,
    ) -> Result<Self> {
        density::admit(owner, coordinates, limits, true)
    }
    pub fn owner(&self) -> &Arc<VerifiedDecomposition> {
        &self.owner
    }
    pub fn coordinates(&self) -> &[Symbol] {
        &self.coordinates
    }
    pub fn regulators(&self) -> &[Symbol] {
        &self.regulators
    }
    pub fn charts(&self) -> &[NormalizedChart] {
        &self.charts
    }
    pub fn strip(&self) -> &ConvergenceStrip {
        &self.strip
    }
    pub fn witness(&self) -> &meromorphic::MeromorphicWitness {
        &self.witness
    }
    pub fn root(&self) -> &Atom {
        &self.root
    }
    pub fn callback_scope(&self) -> &regular::callback::Scope {
        &self.scope
    }
}

impl From<super::Error> for Error {
    fn from(error: super::Error) -> Self {
        match error {
            super::Error::Invalid(s) => Self::Invalid(s),
            super::Error::Unsupported(s) => Self::Unsupported(s),
            super::Error::ResourceIncomplete(s) => Self::ResourceIncomplete(s.into()),
            super::Error::Cancelled => Self::Cancelled,
            other => native(other),
        }
    }
}
#[cfg(test)]
#[path = "secant/tests.rs"]
mod tests;
