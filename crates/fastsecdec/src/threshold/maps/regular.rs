//! Checked regular sections; no density, atlas, or endpoint-continuation authority.
pub mod callback;
mod certificate;
pub mod program;
pub(crate) use certificate::ProposalError;
pub use certificate::{BracketProposal, RegularSection, SectionSide};

use std::{
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
    time::{Duration, Instant},
};
use symgcad::domain::CertificateOptions;

#[derive(Debug, Clone, PartialEq, Eq)]
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

#[derive(Clone, Debug)]
pub struct Limits {
    pub max_degree: usize,
    pub max_terms: usize,
    pub max_rational_bits: u64,
    pub max_refinements: usize,
    pub wall_time: Duration,
    pub certificates: CertificateOptions,
}
impl Default for Limits {
    fn default() -> Self {
        Self {
            max_degree: 256,
            max_terms: 20_000,
            max_rational_bits: 4096,
            max_refinements: 1024,
            wall_time: Duration::from_secs(30),
            certificates: CertificateOptions::default(),
        }
    }
}
pub struct Budget {
    limits: Limits,
    started: Instant,
    cancellation: Option<Arc<AtomicBool>>,
}
impl Budget {
    pub fn new(limits: Limits) -> Self {
        Self {
            limits,
            started: Instant::now(),
            cancellation: None,
        }
    }
    pub fn with_cancellation(mut self, cancellation: Arc<AtomicBool>) -> Self {
        self.cancellation = Some(cancellation);
        self
    }
    pub fn rational(&self, value: &symbolica::domains::rational::Rational) -> Result<()> {
        self.check()?;
        if value.numerator_ref().significant_bits() > self.limits.max_rational_bits
            || value.denominator_ref().significant_bits() > self.limits.max_rational_bits
        {
            return Err(Error::ResourceIncomplete(
                "section rational bit allowance".into(),
            ));
        }
        Ok(())
    }
    pub fn check(&self) -> Result<()> {
        if self
            .cancellation
            .as_ref()
            .is_some_and(|c| c.load(Ordering::Relaxed))
        {
            return Err(Error::Cancelled);
        }
        if self.started.elapsed() >= self.limits.wall_time {
            return Err(Error::ResourceIncomplete(
                "section preparation wall allowance".into(),
            ));
        }
        Ok(())
    }
}
#[cfg(test)]
mod tests;

impl From<crate::kernel::algebraic::Error> for Error {
    fn from(error: crate::kernel::algebraic::Error) -> Self {
        match error {
            crate::kernel::algebraic::Error::Unsupported(s) => Self::Unsupported(s),
            error => Self::Native(error.to_string()),
        }
    }
}
